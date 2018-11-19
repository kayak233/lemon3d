#[macro_use]
extern crate crayon;

use crayon::errors::*;
use crayon::prelude::*;

impl_vertex!{
    Vertex {
        position => [Position; Float; 2; false],
    }
}

struct Pass {
    surface: SurfaceHandle,
    shader: ShaderHandle,
    mesh: MeshHandle,
}

struct Window {
    pass: Pass,
    post_effect: Pass,
    texture: RenderTextureHandle,
    batch: CommandBuffer,
    time: f32,
}

impl Window {
    pub fn new() -> Result<Self> {
        let attributes = AttributeLayoutBuilder::new()
            .with(Attribute::Position, 2)
            .finish();

        //
        let (pass, rendered_texture) = {
            let verts: [Vertex; 3] = [
                Vertex::new([0.0, 0.5]),
                Vertex::new([0.5, -0.5]),
                Vertex::new([-0.5, -0.5]),
            ];
            let idxes: [u16; 3] = [0, 1, 2];

            // Create vertex buffer object.
            let mut params = MeshParams::default();
            params.num_verts = 3;
            params.num_idxes = 3;
            params.layout = Vertex::layout();

            let data = MeshData {
                vptr: Vertex::encode(&verts[..]).into(),
                iptr: IndexFormat::encode(&idxes).into(),
            };

            let mesh = video::create_mesh(params, Some(data))?;

            // Create render texture for post effect.
            let mut params = RenderTextureParams::default();
            params.format = RenderTextureFormat::RGBA8;
            params.dimensions = (568, 320).into();
            let rendered_texture = video::create_render_texture(params)?;

            // Create the surface state for pass 1.
            let mut params = SurfaceParams::default();
            params.set_attachments(&[rendered_texture], None)?;
            params.set_clear(Color::gray(), None, None);
            let surface = video::create_surface(params)?;

            // Create shader state.
            let mut params = ShaderParams::default();
            params.attributes = attributes;
            let vs = include_str!("shaders/render_target_p1.vs").to_owned();
            let fs = include_str!("shaders/render_target_p1.fs").to_owned();
            let shader = video::create_shader(params, vs, fs)?;

            (
                Pass {
                    surface: surface,
                    shader: shader,
                    mesh: mesh,
                },
                rendered_texture,
            )
        };

        let post_effect = {
            let verts: [Vertex; 4] = [
                Vertex::new([-1.0, -1.0]),
                Vertex::new([1.0, -1.0]),
                Vertex::new([1.0, 1.0]),
                Vertex::new([-1.0, 1.0]),
            ];
            let idxes: [u16; 6] = [0, 1, 2, 0, 2, 3];

            let mut params = MeshParams::default();
            params.num_verts = 4;
            params.num_idxes = 6;
            params.layout = Vertex::layout();

            let data = MeshData {
                vptr: Vertex::encode(&verts[..]).into(),
                iptr: IndexFormat::encode(&idxes).into(),
            };

            let mesh = video::create_mesh(params, Some(data))?;

            let params = SurfaceParams::default();
            let surface = video::create_surface(params)?;

            let uniforms = UniformVariableLayout::build()
                .with("renderedTexture", UniformVariableType::RenderTexture)
                .with("time", UniformVariableType::F32)
                .finish();

            let mut params = ShaderParams::default();
            params.attributes = attributes;
            params.uniforms = uniforms;
            let vs = include_str!("shaders/render_target_p2.vs").to_owned();
            let fs = include_str!("shaders/render_target_p2.fs").to_owned();
            let shader = video::create_shader(params, vs, fs)?;

            Pass {
                surface: surface,
                shader: shader,
                mesh: mesh,
            }
        };

        Ok(Window {
            pass: pass,
            post_effect: post_effect,
            texture: rendered_texture,

            batch: CommandBuffer::new(),
            time: 0.0,
        })
    }
}

impl LifecycleListener for Window {
    fn on_update(&mut self) -> Result<()> {
        let surface = self.pass.surface;
        let dc = Draw::new(self.pass.shader, self.pass.mesh);
        self.batch.draw(dc);
        self.batch.submit(surface)?;

        let surface = self.post_effect.surface;
        let mut dc = Draw::new(self.post_effect.shader, self.post_effect.mesh);
        dc.set_uniform_variable("renderedTexture", self.texture);
        dc.set_uniform_variable("time", self.time);
        self.batch.draw(dc);
        self.batch.submit(surface)?;

        self.time += 0.05;
        Ok(())
    }

    fn on_exit(&mut self) -> Result<()> {
        video::delete_render_texture(self.texture);

        video::delete_mesh(self.pass.mesh);
        video::delete_shader(self.pass.shader);
        video::delete_surface(self.pass.surface);

        video::delete_mesh(self.post_effect.mesh);
        video::delete_shader(self.post_effect.shader);
        video::delete_surface(self.post_effect.surface);
        Ok(())
    }
}

fn run() {
    let mut params = Params::default();
    params.window.title = "CR: RenderTexture".into();
    params.window.size = (568, 320).into();
    crayon::application::setup(params, || Window::new()).unwrap();
}

fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    run();
}

#[cfg(target_arch = "wasm32")]
extern crate wasm_bindgen;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn wasm_main() {
    run();
}