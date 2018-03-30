#[macro_use]
extern crate crayon;
#[macro_use]
extern crate failure;

pub mod errors;
pub mod components;
pub mod graphics;
pub mod scene;
pub mod ent;
pub mod assets;

pub mod prelude {
    pub use scene::Scene;
    pub use assets::prelude::*;
    pub use components::prelude::*;
    pub use crayon::ecs::world::Entity;
    pub use crayon::ecs::view::{ArenaGet, ArenaGetMut, Join};
    pub use graphics::DrawOrder;
    pub use ent::{EntAccessor, EntAccessorMut, EntReader, EntWriter};
}