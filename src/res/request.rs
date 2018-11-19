//! A asynchronous loading request.

use std::sync::Arc;

use sched::prelude::{LatchProbe, LockLatch};

pub type Response = Result<Box<[u8]>, failure::Error>;

/// A asynchronous loading request. You sould checks the completion status with
/// `poll` method manually. Once the polling returns true, you could fetch the
/// result by `response`.
pub enum Request {
    NotReady(Arc<LockLatch<Response>>),
    Ok(Response),
}

impl Request {
    #[inline]
    pub fn latch() -> Arc<LockLatch<Response>> {
        Arc::new(LockLatch::new())
    }

    #[inline]
    pub fn new(latch: Arc<LockLatch<Response>>) -> Self {
        Request::NotReady(latch)
    }

    #[inline]
    pub fn ok<T: Into<Box<[u8]>>>(bytes: T) -> Self {
        Request::Ok(Ok(bytes.into()))
    }

    #[inline]
    pub fn err<T: Into<failure::Error>>(err: T) -> Self {
        Request::Ok(Err(err.into()))
    }

    /// Attempt to resolve the request to a final state, and returns true if the
    /// loading result is ready for user.
    #[inline]
    pub fn poll(&mut self) -> bool {
        let rsp = match *self {
            Request::Ok(_) => return true,
            Request::NotReady(ref state) => {
                if !state.is_set() {
                    return false;
                }

                state.take()
            }
        };

        *self = Request::Ok(rsp);
        true
    }

    /// Return the response if exists.
    #[inline]
    pub fn response(&self) -> Option<&Response> {
        match self {
            &Request::Ok(ref rsp) => Some(rsp),
            _ => None,
        }
    }
}

impl Into<Option<Response>> for Request {
    fn into(self) -> Option<Response> {
        match self {
            Request::Ok(rsp) => Some(rsp),
            _ => None,
        }
    }
}

pub struct RequestQueue {
    // FIXME: Use FnOnce instead of Box<Fn> when its stable.
    tasks: Vec<(Request, Box<FnMut(Response)>)>,
    idxes: Vec<usize>,
}

impl RequestQueue {
    pub fn new() -> Self {
        RequestQueue {
            tasks: Vec::new(),
            idxes: Vec::new(),
        }
    }

    pub fn add<T: FnOnce(Response) + 'static>(&mut self, request: Request, func: T) {
        let mut v = Some(func);
        let wrapper = move |rsp| {
            let mut w = None;
            std::mem::swap(&mut v, &mut w);

            if let Some(func) = w {
                func(rsp);
            }
        };

        self.tasks.push((request, Box::new(wrapper)));
    }

    pub fn advance(&mut self) {
        self.idxes.clear();

        // FIXME: Use drain_filter instead of retain and `for` iteration.
        for (i, &mut (ref mut request, _)) in self.tasks.iter_mut().rev().enumerate() {
            if request.poll() {
                self.idxes.push(i)
            }
        }

        for i in self.idxes.drain(..) {
            let (request, mut func) = self.tasks.remove(i);
            let v: Option<Response> = request.into();
            func(v.unwrap());
        }
    }
}