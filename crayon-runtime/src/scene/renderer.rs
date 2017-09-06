use core::application::Application;
use ecs::{World, Entity, ArenaGetter};

use math;
use math::Transform as MathTransform;

use resource;
use graphics;

use super::*;
use super::errors::*;

pub trait Renderable {
    fn is_visible(&self) -> bool;
    fn set_visible(&mut self, visible: bool);
    fn material(&self) -> Option<&resource::MaterialPtr>;
}

pub struct Renderer {
    sprite_renderer: SpriteRenderer,
    mesh_renderer: MeshRenderer,
    ambient: (graphics::Color, f32),
}

impl Renderer {
    pub fn new(mut app: &mut Application) -> Result<Renderer> {
        Ok(Renderer {
               sprite_renderer: SpriteRenderer::new(&mut app)?,
               mesh_renderer: MeshRenderer::new(&mut app)?,
               ambient: (graphics::Color::white(), 1.0f32),
           })
    }

    pub fn set_ambient_color(&mut self, color: graphics::Color, intensity: f32) {
        self.ambient = (color, intensity);
    }

    pub fn draw(&mut self, mut app: &mut Application, world: &World) -> Result<()> {
        // Collect all the enable camera in the world.
        let cameras = {
            let mut cameras = Vec::new();
            let (view, mut arenas) = world.view_with_2::<Transform, Camera>();
            for v in view {
                if let Ok(camera) = Renderer::parse_render_camera(&mut app.graphics,
                                                                  &mut arenas,
                                                                  v) {
                    cameras.push(camera);
                }
            }
            cameras
        };

        let env = self.parse_render_env(&world);

        // Draw from the viewport of camera.
        for v in cameras {
            self.mesh_renderer.draw(&mut app, &world, &env, &v)?;
            self.sprite_renderer.draw(&mut app, &world, &v)?;
        }

        Ok(())
    }

    fn parse_render_camera(mut video: &mut graphics::Graphics,
                           arenas: &mut (ArenaGetter<Transform>, ArenaGetter<Camera>),
                           camera: Entity)
                           -> Result<RenderCamera> {
        let mut c = arenas.1.get_mut(camera).unwrap();
        c.update_video_object(&mut video)?;

        let decomposed = Transform::world_decomposed(&arenas.0, camera)?;
        let inverse = decomposed
            .inverse_transform()
            .ok_or(ErrorKind::CanNotInverseTransform)?;

        Ok(RenderCamera {
               view: Camera::view_matrix(&arenas.0, camera)?,
               projection: c.projection_matrix(),
               inverse_transform: inverse,
               clip: c.clip_plane(),
               vso: c.video_object().unwrap(),
           })
    }

    fn parse_render_env(&self, world: &World) -> RenderEnvironment {
        let (view, arenas) = world.view_with_2::<Transform, Light>();
        for v in view {
            let light = arenas.1.get(*v).unwrap();
            if light.is_enable() {
                let (color, _) = match light {
                    &Light::Directional(v) => (v.color, v.intensity),
                    &Light::Point(v) => (v.color, v.intensity),
                };

                if let Ok(pos) = Transform::world_position(&arenas.0, v) {
                    return RenderEnvironment {
                               ambient: self.ambient.0,
                               light_pos: pos,
                               light_color: color,
                           };
                }
            }
        }

        return RenderEnvironment {
                   ambient: self.ambient.0,
                   light_pos: math::Vector3::unit_z(),
                   light_color: graphics::Color::white(),
               };
    }
}

pub struct RenderEnvironment {
    pub ambient: graphics::Color,
    pub light_pos: math::Vector3<f32>,
    pub light_color: graphics::Color,
}

pub struct RenderCamera {
    pub view: math::Matrix4<f32>,
    pub projection: math::Matrix4<f32>,
    pub inverse_transform: Decomposed,
    pub vso: graphics::ViewHandle,
    pub clip: (f32, f32),
}

impl RenderCamera {
    pub fn transform(&self, position: &math::Vector3<f32>) -> math::Vector3<f32> {
        self.inverse_transform.rot * (position * self.inverse_transform.scale) +
        self.inverse_transform.disp
    }

    pub fn is_inside(&self, position: &math::Vector3<f32>) -> bool {
        position.z >= self.clip.0 && position.z < self.clip.1
    }
}