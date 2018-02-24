use std::sync::Arc;
use std::ops::Deref;

use resource::prelude::*;

use graphics::GraphicsSystemShared;
use graphics::errors::Result;
use graphics::assets::prelude::*;
use graphics::assets::texture_loader::TextureParser;
use graphics::assets::mesh_loader::MeshParser;

pub struct GraphicsSystemGuard {
    stack: Vec<Resource>,
    video: Arc<GraphicsSystemShared>,
}

impl Deref for GraphicsSystemGuard {
    type Target = GraphicsSystemShared;

    fn deref(&self) -> &Self::Target {
        &self.video
    }
}

impl GraphicsSystemGuard {
    pub fn new(video: Arc<GraphicsSystemShared>) -> Self {
        GraphicsSystemGuard {
            stack: Vec::new(),
            video: video,
        }
    }

    #[inline]
    pub fn create_surface(&mut self, setup: SurfaceSetup) -> Result<SurfaceHandle> {
        let v = self.video.create_surface(setup)?;
        Ok(self.push(v))
    }

    #[inline]
    pub fn create_shader(
        &mut self,
        location: Location,
        setup: ShaderSetup,
    ) -> Result<ShaderHandle> {
        let v = self.video.create_shader(location, setup)?;
        Ok(self.push(v))
    }

    #[inline]
    pub fn create_mesh_from<T>(
        &mut self,
        location: Location,
        setup: MeshSetup,
    ) -> Result<MeshHandle>
    where
        T: MeshParser + Send + Sync + 'static,
    {
        let v = self.video.create_mesh_from::<T>(location, setup)?;
        Ok(self.push(v))
    }

    #[inline]
    pub fn create_mesh<'a, 'b, T1, T2>(
        &mut self,
        location: Location,
        setup: MeshSetup,
        verts: T1,
        idxes: T2,
    ) -> Result<MeshHandle>
    where
        T1: Into<Option<&'a [u8]>>,
        T2: Into<Option<&'b [u8]>>,
    {
        let v = self.video.create_mesh(location, setup, verts, idxes)?;
        Ok(self.push(v))
    }

    #[inline]
    pub fn create_texture_from<T>(
        &mut self,
        location: Location,
        setup: TextureSetup,
    ) -> Result<TextureHandle>
    where
        T: TextureParser + Send + Sync + 'static,
    {
        let v = self.video.create_texture_from::<T>(location, setup)?;
        Ok(self.push(v))
    }

    #[inline]
    pub fn create_render_texture(
        &mut self,
        setup: RenderTextureSetup,
    ) -> Result<RenderTextureHandle> {
        let v = self.video.create_render_texture(setup)?;
        Ok(self.push(v))
    }

    #[inline]
    pub fn create_texture<'a, T>(
        &mut self,
        location: Location,
        setup: TextureSetup,
        data: T,
    ) -> Result<TextureHandle>
    where
        T: Into<Option<&'a [u8]>>,
    {
        let v = self.video.create_texture(location, setup, data)?;
        Ok(self.push(v))
    }

    pub fn clear(&mut self) {
        for v in self.stack.drain(..) {
            match v {
                Resource::Texture(handle) => self.video.delete_texture(handle),
                Resource::RenderTexture(handle) => self.video.delete_render_texture(handle),
                Resource::Mesh(handle) => self.video.delete_mesh(handle),
                Resource::Surface(handle) => self.video.delete_surface(handle),
                Resource::ShaderState(handle) => self.video.delete_shader(handle),
            }
        }
    }

    fn push<T>(&mut self, resource: T) -> T
    where
        T: Copy + Into<Resource>,
    {
        self.stack.push(resource.into());
        resource
    }
}

impl Drop for GraphicsSystemGuard {
    fn drop(&mut self) {
        self.clear();
    }
}

enum Resource {
    Texture(TextureHandle),
    RenderTexture(RenderTextureHandle),
    Mesh(MeshHandle),
    Surface(SurfaceHandle),
    ShaderState(ShaderHandle),
}

impl From<TextureHandle> for Resource {
    fn from(handle: TextureHandle) -> Resource {
        Resource::Texture(handle)
    }
}

impl From<RenderTextureHandle> for Resource {
    fn from(handle: RenderTextureHandle) -> Resource {
        Resource::RenderTexture(handle)
    }
}

impl From<MeshHandle> for Resource {
    fn from(handle: MeshHandle) -> Resource {
        Resource::Mesh(handle)
    }
}

impl From<SurfaceHandle> for Resource {
    fn from(handle: SurfaceHandle) -> Resource {
        Resource::Surface(handle)
    }
}

impl From<ShaderHandle> for Resource {
    fn from(handle: ShaderHandle) -> Resource {
        Resource::ShaderState(handle)
    }
}
