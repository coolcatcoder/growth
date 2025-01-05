use crate::prelude::*;

pub mod prelude {
    pub use super::{AmbientFriction, Gravity, Verlet};
}

/// The fixed time between every update to the particle physics.
pub const TIME_DELTA_SECONDS: f64 = 1. / 30.;
//const TIME_DELTA: Duration = Duration::from_secs_f64(0.1);

//MARK: Verlet
/// Performs velocity verlet integration.
/// I learned about this from https://www.algorithm-archive.org/contents/verlet_integration/verlet_integration.html
#[derive(Component)]
#[require(Transform)]
pub struct Verlet {
    // Separate from Transform's translation, so I can potentially perform extrapolation/interpolation.
    translation: Vec2,
    velocity: Vec2,
    acceleration: Vec2,
}

impl Verlet {
    /// Creates a verlet particle from just the translation.
    pub fn from_translation(translation: Vec2) -> Self {
        Self {
            translation,
            velocity: Vec2::ZERO,
            acceleration: Vec2::ZERO,
        }
    }

    /// Adds acceleration.
    /// Do not multiply your input acceleration by delta time.
    /// That will happen automatically later.
    /// This does mean that you can only accelerate in the physics schedule.
    pub fn accelerate(&mut self, acceleration: Vec2) {
        self.acceleration += acceleration;
    }

    /// The change in velocity by applying friction.
    /// Remember to -= this from velocity.
    /// Internally multiplies by time delta, so you don't have to.
    pub fn velocity_delta_from_friction(&self, friction: f32) -> Vec2 {
        self.velocity.abs() * self.velocity * friction * TIME_DELTA_SECONDS as f32
    }

    /// Updates all the particles to their next positions.
    pub fn update(mut particles: Query<&mut Self>) {
        particles.par_iter_mut().for_each(|mut particle| {
            const TIME_DELTA_SECONDS_F32: f32 = TIME_DELTA_SECONDS as f32;
            const HALFED_AFTER_SQUARED_TIME_DELTA_SECONDS: f32 =
                TIME_DELTA_SECONDS_F32 * TIME_DELTA_SECONDS_F32 * 0.5;

            let velocity = particle.velocity;
            let acceleration = particle.acceleration;

            particle.translation += velocity * TIME_DELTA_SECONDS_F32
                + acceleration * HALFED_AFTER_SQUARED_TIME_DELTA_SECONDS;
            particle.velocity += acceleration * TIME_DELTA_SECONDS_F32;
            particle.acceleration = Vec2::ZERO;
        });
    }

    pub fn solve_collisions(
        grid: Res<ColliderGrid>,
        particles: Query<(Entity, &Radius, &Transform, &Self)>,
        colliders: Query<(&Radius, &Transform, Option<&Self>)>,
        commands: ParallelCommands,
    ) {
        particles
            .par_iter()
            .for_each(|(entity, radius, transform, particle)| {
                // This is manually constructed, instead of using the ones already implemented on ColliderGrid.
                // This is for extra optimisation, and ease of tinkering.

                let Some(grid_index) = grid.translation_to_index(particle.translation) else {
                    return;
                };

                // At first we used any() so we could find 1 collision, solve it, and break early, but this caused some strange behaviour.
                // TODO: Profile a for loop.
                // TODO: Profile par_iter().
                grid.cells[grid_index].0.iter().for_each(|other_entity| {
                    // Checking for collisions with yourself is pointless.
                    if entity == *other_entity {
                        return;
                    }

                    // Get the collider information from the entity.
                    let Ok((other_radius, other_transform, other_particle)) =
                        colliders.get(*other_entity)
                    else {
                        return;
                    };

                    // This whole collision separation algorithm is taken and modified from https://www.youtube.com/watch?v=lS_qeBy3aQI at 4:09.
                    let radius_sum = radius.0 + other_radius.0;

                    let collision_axis =
                        transform.translation.xy() - other_transform.translation.xy();

                    // Collision can be checked using distance_squared, this saves a square root call.
                    let distance_squared = collision_axis.length_squared();

                    if distance_squared <= radius_sum * radius_sum {
                        let distance = distance_squared.sqrt();
                        // Normalise the axis, so that it has no magnitude.
                        // This works because distance is the magnitude of the collision axis.
                        let collision_axis = collision_axis / distance;
                        // How much to move along the collision axis to be not be intersecting each other.
                        let distance_delta = radius_sum - distance;
                        // The change in translation needed to move 1 collider out of the other.
                        // When we move both, we just move each by half of this, in opposite directions.
                        let translation_delta = distance_delta * collision_axis;

                        if let Some(_other_particle) = other_particle {
                            todo!();
                        } else {
                            // Surface friction.
                            let velocity_delta = particle.velocity_delta_from_friction(0.01);

                            commands.command_scope(move |mut commands| {
                                let Some(mut particle) = commands.get_entity(entity) else {
                                    return;
                                };

                                particle.entry::<Self>().and_modify(move |mut particle| {
                                    particle.translation += translation_delta;
                                    // Experimental surface friction experiment.
                                    particle.velocity -= velocity_delta;
                                });
                            });
                        }
                    }
                });
            });
    }

    /// Sets Transform's translation every frame a position roughly around the particle's translation.
    /// It may use interpolation/extrapolation to keep everything looking visually pleasant.
    pub fn sync_position(mut particles: Query<(&Self, &mut Transform)>) {
        particles
            .par_iter_mut()
            .for_each(|(particle, mut transform)| {
                //TODO: Idea. Sync with transform exactly every time the physics loop runs.
                // Then have an every-frame system that then performs the extrapolation.

                // Temp. Replace with better interpolation/extrapolation.
                transform.translation.x = particle.translation.x;
                transform.translation.y = particle.translation.y;
            });
    }
}

//MARK: AmbientFriction
/// Applies a small amount of friction every update.
#[derive(Component)]
pub struct AmbientFriction;

impl AmbientFriction {
    /// Applies the friction.
    pub fn update(mut particles: Query<&mut Verlet, With<Self>>) {
        particles.par_iter_mut().for_each(|mut particle| {
            let velocity_delta = particle.velocity_delta_from_friction(0.005);
            particle.velocity -= velocity_delta;
        });
    }
}

#[derive(Component)]
pub struct Gravity;

impl Gravity {
    /// Accelerates every particle downwards.
    pub fn update(mut particles: Query<&mut Verlet, With<Self>>) {
        particles.par_iter_mut().for_each(|mut particle| {
            particle.accelerate(Vec2::new(0., -98.));
        });
    }
}
