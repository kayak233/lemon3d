use std::path::Path;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use crayon::{application, resource, utils, graphics, math};
use rusttype;

use super::font::{Font, FontHandle, LayoutIter};
use super::font_error::*;

pub struct FontSystem {
    fallback: Font,
    dpi_factor: f32,

    texture_cache: FontTextureCache,
    font_states: utils::ObjectPool<FontState>,
    font_requests: Arc<RwLock<HashMap<FontHandle, FontState>>>,
    handles: HashMap<utils::HashValue<Path>, FontHandle>,

    resource: Arc<resource::ResourceSystemShared>,
}

impl FontSystem {
    pub fn new(ctx: &application::Context) -> Self {
        let fallback = include_bytes!("../../resources/fonts/FiraSans-Regular.ttf");

        FontSystem {
            fallback: Font::new(&fallback[..]),
            dpi_factor: 1.0,
            texture_cache: FontTextureCache::new(ctx),
            font_states: utils::ObjectPool::new(),
            font_requests: Arc::new(RwLock::new(HashMap::new())),
            handles: HashMap::new(),
            resource: ctx.shared::<resource::ResourceSystem>().clone(),
        }
    }

    pub fn load<P>(&mut self, path: P) -> FontHandle
        where P: AsRef<Path>
    {
        let hash: utils::HashValue<Path> = path.as_ref().into();
        if let Some(handle) = self.handles.get(&hash) {
            return *handle;
        }

        let handle = self.font_states.create(FontState::NotReady).into();
        self.handles.insert(hash, handle);

        let loader = FontLoader::new(self.font_requests.clone(), handle);
        self.resource.load_async(loader, path);

        handle
    }

    pub fn unload<P>(&mut self, path: P)
        where P: AsRef<Path>
    {
        let hash = path.as_ref().into();
        if let Some(handle) = self.handles.remove(&hash) {
            self.font_states.free(handle);
        }
    }

    pub(crate) fn advance(&mut self) {
        {
            let mut requests = self.font_requests.write().unwrap();
            for (k, v) in requests.drain() {
                if let Some(state) = self.font_states.get_mut(&k as &utils::Handle) {
                    *state = v;
                }
            }
        }
    }

    pub fn set_dpi_factor(&mut self, dpi_factor: f32) {
        self.dpi_factor = dpi_factor;
    }

    /// The conservative pixel-boundary bounding box for this text. This is the smallest
    /// rectangle aligned to pixel boundaries that encloses the shape.
    pub fn bounding_box(&mut self,
                        handle: Option<FontHandle>,
                        text: &str,
                        scale: f32,
                        h_wrap: Option<f32>,
                        v_wrap: Option<f32>)
                        -> (math::Vector2<f32>, math::Vector2<f32>) {
        let font = if let Some(handle) = handle {
            if let Some(&FontState::Ready(ref v)) =
                self.font_states.get(&handle as &utils::Handle) {
                v
            } else {
                &self.fallback
            }
        } else {
            &self.fallback
        };

        font.bounding_box(text, scale, h_wrap, v_wrap)
    }

    /// A convenience function for laying out glyphs for a text.
    pub fn layout<'a, 'b>(&'a mut self,
                          handle: Option<FontHandle>,
                          text: &'b str,
                          scale: f32,
                          h_wrap_limit: Option<f32>,
                          v_wrap_limit: Option<f32>)
                          -> Result<(graphics::TextureHandle, FontGlyphIter<'a, 'b>)> {
        let (id, font) = if let Some(handle) = handle {
            if let Some(&FontState::Ready(ref v)) =
                self.font_states.get(&handle as &utils::Handle) {
                ((handle.index() + 1) as usize, v)
            } else {
                (0, &self.fallback)
            }
        } else {
            (0, &self.fallback)
        };

        let dpi_factor = self.dpi_factor;
        let h_wrap_limit = h_wrap_limit.map(|v| v * dpi_factor);
        let v_wrap_limit = v_wrap_limit.map(|v| v * dpi_factor);

        for v in font.layout(text, scale * self.dpi_factor, h_wrap_limit, v_wrap_limit) {
            self.texture_cache.add(id, v);
        }

        let handle = self.texture_cache.update_texture()?;

        Ok((handle,
            FontGlyphIter {
                texture_cache: &self.texture_cache,
                id: id,
                iter: font.layout(text, scale * self.dpi_factor, h_wrap_limit, v_wrap_limit),
                inverse_dpi_factor: 1.0 / self.dpi_factor,
            }))
    }
}

pub struct FontGlyphIter<'a, 'b> {
    texture_cache: &'a FontTextureCache,
    id: usize,
    iter: LayoutIter<'a, 'b>,
    inverse_dpi_factor: f32,
}

impl<'a, 'b> Iterator for FontGlyphIter<'a, 'b> {
    type Item = (rusttype::Rect<f32>, rusttype::Rect<i32>);

    fn next(&mut self) -> Option<Self::Item> {
        for v in &mut self.iter {
            if let Some((uv, mut screen)) = self.texture_cache.rect_for(self.id, &v) {
                screen.min.x = (screen.min.x as f32 * self.inverse_dpi_factor) as i32;
                screen.min.y = (screen.min.y as f32 * self.inverse_dpi_factor) as i32;
                screen.max.x = (screen.max.x as f32 * self.inverse_dpi_factor) as i32;
                screen.max.y = (screen.max.y as f32 * self.inverse_dpi_factor) as i32;
                return Some((uv, screen));
            }
        }

        None
    }
}

enum FontState {
    Disposed,
    Ready(Font),
    NotReady,
}

struct FontTextureCache {
    texture_cache: rusttype::gpu_cache::Cache<'static>,
    texture: Option<graphics::TextureHandle>,
    label: graphics::ResourceLabel,
    video: Arc<graphics::GraphicsSystemShared>,
}

impl FontTextureCache {
    fn new(ctx: &application::Context) -> Self {
        let video = ctx.shared::<graphics::GraphicsSystem>().clone();

        FontTextureCache {
            texture_cache: rusttype::gpu_cache::Cache::new(1024, 1024, 0.25, 0.25),
            texture: None,
            label: video.create_label(),
            video: video,
        }
    }

    #[inline]
    fn add(&mut self, id: usize, glyph: rusttype::PositionedGlyph) {
        self.texture_cache.queue_glyph(id, glyph.standalone());
    }

    #[inline]
    fn rect_for(&self,
                id: usize,
                glyph: &rusttype::PositionedGlyph)
                -> Option<(rusttype::Rect<f32>, rusttype::Rect<i32>)> {
        self.texture_cache.rect_for(id, glyph).unwrap()
    }

    fn update_texture(&mut self) -> Result<graphics::TextureHandle> {
        if self.texture.is_none() {
            let mut setup = graphics::TextureSetup::default();
            setup.filter = graphics::TextureFilter::Linear;
            setup.mipmap = false;
            setup.dimensions = (1024, 1024);
            setup.format = graphics::TextureFormat::U8;

            self.texture = Some(self.video.create_texture(self.label, setup, None)?);
        }

        let handle = self.texture.unwrap();
        let video = &self.video;
        self.texture_cache
            .cache_queued(|rect, data| {
                              let rect = utils::Rect::new(math::Point2::new(rect.min.x as i32,
                                                                            rect.min.y as i32),
                                                          math::Point2::new(rect.max.x as i32,
                                                                            rect.max.y as i32));
                              video.update_texture(handle, rect, data).unwrap();
                          })
            .unwrap();

        Ok(handle)
    }
}

struct FontLoader {
    font_requests: Arc<RwLock<HashMap<FontHandle, FontState>>>,
    handle: FontHandle,
}

impl FontLoader {
    fn new(font_requests: Arc<RwLock<HashMap<FontHandle, FontState>>>, handle: FontHandle) -> Self {
        FontLoader {
            font_requests: font_requests,
            handle: handle,
        }
    }

    #[inline(always)]
    fn try_parse(v: resource::errors::Result<&[u8]>) -> Result<Font> {
        Ok(Font::new(v?))
    }
}

impl resource::ResourceAsyncLoader for FontLoader {
    fn on_finished(&mut self, path: &Path, result: resource::errors::Result<&[u8]>) {
        let state = match Self::try_parse(result) {
            Ok(font) => FontState::Ready(font),
            Err(error) => {
                println!("Failed to load font from {:?}, due to {:?}.", path, error);
                FontState::Disposed
            }
        };

        let mut requests = self.font_requests.write().unwrap();
        requests.insert(self.handle, state);
    }
}