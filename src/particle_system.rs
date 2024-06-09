use super::{ParticleLifetime, Rltk, RGB};
use super::{Position, Renderable};
use specs::prelude::*;

pub struct ParticleSpawnSystem {}

impl<'a> System<'a> for ParticleSpawnSystem {
    #[allow(clippy::type_complexity)]
    type SystemData = (
        Entities<'a>,
        WriteStorage<'a, Position>,
        WriteStorage<'a, Renderable>,
        WriteStorage<'a, ParticleLifetime>,
        WriteExpect<'a, ParticleBuilder>,
    );

    fn run(&mut self, data: Self::SystemData) {
        let (entities, mut positions, mut renderables, mut particles, mut particle_builder) = data;

        for new_particle in particle_builder.requests.iter() {
            let p = entities.create();
            positions
                .insert(
                    p,
                    Position {
                        x: new_particle.x,
                        y: new_particle.y,
                    },
                )
                .expect("Unable to insert Position");
            renderables
                .insert(
                    p,
                    Renderable {
                        glyph: new_particle.glyph,
                        fg: new_particle.fg,
                        bg: new_particle.bg,
                        render_order: 0,
                    },
                )
                .expect("Unable to insert Renderable");
            particles
                .insert(
                    p,
                    ParticleLifetime {
                        lifetime_ms: new_particle.lifetime,
                    },
                )
                .expect("Unable to insert Particle Lifetime");
        }

        particle_builder.requests.clear();
    }
}

pub fn tick_and_cull_dead_particles(ecs: &mut World, ctx: &Rltk) {
    let mut dead_particles: Vec<Entity> = Vec::new();

    {
        let entities = ecs.entities();
        let mut particles = ecs.write_storage::<ParticleLifetime>();
        for (entity, particle) in (&entities, &mut particles).join() {
            particle.lifetime_ms -= ctx.frame_time_ms;
            if particle.lifetime_ms < 0.0 {
                dead_particles.push(entity);
            }
        }
    }

    for dead in dead_particles.iter() {
        ecs.delete_entity(*dead).expect("Particle will not die");
    }
}

pub struct ParticleRequest {
    x: i32,
    y: i32,
    fg: RGB,
    bg: RGB,
    glyph: rltk::FontCharType,
    lifetime: f32,
}

pub struct ParticleBuilder {
    pub requests: Vec<ParticleRequest>,
}

impl ParticleBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> ParticleBuilder {
        ParticleBuilder {
            requests: Vec::new(),
        }
    }

    pub fn request(
        &mut self,
        x: i32,
        y: i32,
        fg: RGB,
        bg: RGB,
        glyph: rltk::FontCharType,
        lifetime: f32,
    ) {
        self.requests.push(ParticleRequest {
            x,
            y,
            fg,
            bg,
            glyph,
            lifetime,
        });
    }
}
