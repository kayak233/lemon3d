//! A stateless, layered, multithread graphics system with OpenGL backends.
//!
//! # Overview and Goals
//!
//! The management of graphics effects has become an important topic and key feature of
//! rendering engines. With the increasing number of effects it is not sufficient anymore
//! to only support them, but also to integrate them into the rendering engine in a clean
//! and extensible way.
//!
//! The goal of this work and simultaneously its main contribution is to design and
//! implement an advanced effects framework. Using this framework it should be easy for
//! further applications to combine several small effects like texture mapping, shading
//! and shadowing in an automated and transparent way and apply them to any 3D model.
//! Additionally, it should be possible to integrate new effects and use the provided
//! framework for rapid prototyping.
//!
//! ### Multi Platform
//!
//! Ideally, crayon should be able to run on macOS, windows and popular mobile-platforms.
//! There still are a huge number of performance and feature limited devices, so this
//! graphics module will always be limited by lower-end 3D APIs like OpenGL ES2.0.
//!
//! ### Stateless Pipeline
//!
//! Ordinary OpenGL application deals with stateful APIs, which is error-prone. This
//! means whenever you change any state in the API for subsequent draw calls, this state
//! change also affects draw calls submitted at a later point in time. Ideally, submitting
//! a draw call with whatever state we want should not affect any of the other draw calls,
//! even in multi-thread environments.
//!
//! Modern 3D-APIs like [gfx-rs](https://github.com/gfx-rs/gfx), [glium](https://github.com/glium/glium)
//! bundles render state and data into a few, precompiled resource objects which are
//! combined into final render pipeline. We should follow the same philosophy.
//!
//! ### Multi-thread
//!
//! In most cases, dividing OpenGL rendering across multiple threads will not result in
//! any performance improvement due the pipeline nature of OpenGL. What we are about
//! to do is actually exploiting parallelism in resource preparation, and provides a set of
//! multi-thread friendly APIs.
//!
//! The most common solution is by using a double-buffer of commands. This consists of
//! running the renderer backend in a speparate thread, where all draw calls and communication
//! with the OpenGL API are performed. The frontend thread that runs the game logic
//! communicates with the backend renderer via a command double-buffer.
//!
//! ### Layered Rendering
//!
//! Its important to sort video commands (generated by different threads) before submiting
//! them to OpenGL, for the sack of both correctness and performance. For example, to draw
//! transparent objects via blending, we need draw opaque object first, usually from front-to-back,
//! and draw translucents from back-to-front.
//!
//! The idea here is to assign a integer key to a command which is used for sorting. Depending
//! on where those bits are stored in the integer, you can apply different sorting criteria
//! for the same array of commands, as long as you know how the keys were built.
//!
//! # Resource Objects
//!
//! Render state and data, which are combined into final render pipeline, are bundled into a
//! few, precompiled resource objects in graphics module.
//!
//! All resources types can be created instantly from data in memory, and meshes, textures
//! can also be loaded asynchronously from the filesystem.
//!
//! And the actual resource objects are usually private and opaque, you will get a `Handle`
//! immediately for every resource objects you created instead of some kind of reference.
//! Its the unique identifier for the resource, its type-safe and copyable.
//!
//! When you are done with the created resource objects, its your responsiblity to delete the
//! resource object with `Handle` to avoid leaks.
//!
//! For these things loaded from filesystem, it could be safely shared by the `Location`. We
//! keeps a use-counting internally. It will not be freed really, before all the users deletes
//! its `Handle`.
//!
//! ### Surface Object
//!
//! Surface object plays as the `Layer` role we mentioned above, all the commands we submitted
//! in application code is attached to a specific `Surface`. Commands inside `Surface` are
//! sorted before submitting to underlying OpenGL.
//!
//! Surface object also holds references to render target, and wraps rendering operations to
//! it. Likes clearing, offscreen-rendering, MSAA resolve etc..
//!
//! ```rust,ignore
//! // Creates a `SurfaceSetup` object.
//! let setup = SurfaceSetup::default();
//! // Sets he render target of this `Surface` layer. If `framebuffer` is none,
//! // default framebuffer will be used as render target.
//! setup.set_framebuffer(framebuffer);
//! // Sets the clear flags for this surface and its underlying framebuffer.
//! setup.set_clear(Color::white(), 1.0, None);
//! // Sets the viewport of view. This specifies the affine transformation of (x, y) from
//! // NDC(normalized device coordinates) to normalized window coordinates.
//! setup.set_viewport((0.0, 0.0), (1.0, 1.0));
//! // Creats a `SurfaceObject` by handing the setup parameters.
//! let surface = graphics.create_surface(setup).unwrap();
//!
//! // Deletes surface object.
//! graphics.delete_surface(surface);
//! ```
//!
//! ### Shader Object
//!
//! `ShahderObject` is introduced to encapsulate all stateful things we need to configurate
//! graphics pipeline. This would also enable us to easily change the order of draw calls
//! and get rid of redundant state changes.
//!
//! ```rust,ignore
//! // Creates a `ShaderSetup` object.
//! let mut setup = ShaderSetup::from(vs, fs, layout, render_state);
//! setup.layout = /* Layout of shader attributes. */
//! setup.vs = /* The source of vertex shader. */
//! setup.fs = /* The source of pixel shader. */
//! setup.render_state = /* Configurable render state like blending, depth_test, etc. */
//! // Creats a `ShaderObject` by handing the setup parameters.
//! let shader = graphics.create_shader(setup).unwrap();
//!
//! // Deletes shader object.
//! graphics.delete_shader(setup);
//! ```
//!
//! _TODO_: SPIRV based shader compiling and information generations.
//!
//! ### Texture Object
//!
//! _TODO_: Compressed texture.
//! _TODO_: Cube texture.
//! _TODO_: 3D texture.
//!
//! ### Mesh Object
//!
//! _TODO_: Mesh abstraction.
//! _TODO_: Mesh loader.
//! _TODO_: Mesh builder.
//!
//! # Commands
//!
//! There are two kinds of commands that could be submitted into `Surface` object, the
//! resource manipulation and draw call.
//!
//! And every commands is assigned to a u64 integer key which is then used for sorting
//! in ascending order. Typically, it could be encoded with certain data like distance,
//! material, shader etc. Depending on where those bits are stored in the integer, u
//! can apply different sorting criteria for the same array of draw calls, as long as
//! u know how the keys were built.
//!
//! ```rust,ignore
//! // Updates the vertex buffer object.
//! let slice = Vertex::as_bytes(vertices);
//! let cmd = Command::update_vertex_buffer(vbo, 0, slice);
//! graphics.submit(surface, 0, cmd).unwrap();
//!
//! // Update the index buffer object.
//! let slice = IndexFormat::encode(indices);
//! let cmd = Command::update_index_buffer(ibo, 0, slice);
//! graphics.submit(surface, 1, cmd).unwrap();
//!
//! // Update the texture object.
//! let cmd = Command::update_texture(texture, rect, data);
//! graphics.submit(surface, 0, cmd).unwrap();
//!
//! // Set the scissor state.
//! let scissor = graphics::SurfaceScissor::Enable(scissor_pos, scissor_size);
//! let cmd = graphics::Command::set_scissor(scissor);
//! self.video.submit(surface, 0, cmd).unwrap();
//! ```
//!
//! Draw call command is a little bit complex than resource manipulations above, so we
//! provide a helper builder `DrawCall`, it could be used as follows:
//!
//! ```rust,ignore
//! // Creates a DrawCall buidler from shader.
//! let mut dc = graphics::DrawCall::new(self.shader, mesh);
//! // Sets the specified uniform variable.
//! dc.set_uniform_variable("matrix", matrix);
//! dc.set_uniform_variable("texture", texture);
//! // Builds a command and submits it.
//! let cmd = dc.build(from, len)?;
//! self.video.submit(self.surface, 0, cmd).unwrap();
//! ```

/// Maximum number of attributes in vertex layout.
pub const MAX_VERTEX_ATTRIBUTES: usize = 12;
/// Maximum number of attachments in framebuffer.
pub const MAX_FRAMEBUFFER_ATTACHMENTS: usize = 8;
/// Maximum number of uniform variables in shader.
pub const MAX_UNIFORM_VARIABLES: usize = 32;
/// Maximum number of textures in shader.
pub const MAX_UNIFORM_TEXTURE_SLOTS: usize = 8;

#[macro_use]
pub mod assets;
pub mod errors;
pub mod window;
pub mod guard;
pub mod command;

mod backend;
mod service;

pub use self::service::{GraphicsFrameInfo, GraphicsSystem, GraphicsSystemShared};

pub mod prelude {
    pub use super::{GraphicsFrameInfo, GraphicsSystem, GraphicsSystemShared};
    pub use super::guard::GraphicsSystemGuard;

    pub use super::command::{Command, DrawCall};
    pub use super::assets::mesh::{MeshHandle, MeshIndex};
    pub use super::assets::shader::ShaderHandle;
    pub use super::assets::surface::{SurfaceHandle, SurfaceScissor, SurfaceViewport};
    pub use super::assets::texture::{RenderTextureHandle, TextureHandle};
}
