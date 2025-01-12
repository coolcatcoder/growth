use bevy::ecs::system::SystemState;

use crate::prelude::*;

pub mod prelude {
    pub use super::{AmbientFriction, Gravity, Grounded, Verlet};
}

/// The fixed time between every update to the particle physics.
pub const TIME_DELTA_SECONDS: f64 = 1. / 30.;
pub const COLLISION_SUBSTEPS: u8 = 3;
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

    /// Stop particles from intersecting each other.
    pub fn solve_collisions(
        world: &mut World,
        system: &mut SystemState<(
            Query<(Entity, &Radius, &Self)>,
            Query<(&Radius, &Transform, Option<&Self>)>,
            Res<ColliderGrid>,
        )>,

        // (entity, translation, velocity)
        // Every entity should appear at most once in here.
        // TODO: We could store a component on each particle, that we mutate. Then we iter over all components and set verlet.translation to equal them.
        // I would prefer to avoid the extra memory if possible though.
        mut collision_resolutions: Local<Parallel<Vec<(Entity, Vec2, Vec2)>>>,
    ) {
        const COLLISION_TIME_DELTA_SECONDS: f32 =
            TIME_DELTA_SECONDS as f32 / COLLISION_SUBSTEPS as f32;

        for _ in 0..COLLISION_SUBSTEPS {
            let (particles, colliders, grid) = system.get(world);

            particles.par_iter().for_each(|(entity, radius, particle)| {
                // This is manually constructed, instead of using the ones already implemented on ColliderGrid.
                // This is for extra optimisation, and ease of tinkering.

                // We can keep this as the source of truth, and update it with every collision.
                // This allows future collisions to be more accurate.
                let mut translation = particle.translation;
                let mut velocity = particle.velocity;
                let radius = radius.0;

                // Because translation can change, this is technically incorrect.
                // We should instead work out grid index every time translation changes.
                // That sounds slow, and complicated though, so we aren't going to do that.
                let Some(grid_index) = grid.translation_to_index(translation) else {
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
                    // other_translation is Verlet if the collider has it, and if not, then we use Transform.
                    let (other_radius, other_translation, collision_delta_multiplier) = {
                        let Ok((other_radius, other_transform, other_translation)) =
                            colliders.get(*other_entity)
                        else {
                            return;
                        };

                        let (other_translation, collision_delta_multiplier) =
                            if let Some(other_translation) = other_translation {
                                (other_translation.translation, 0.5)
                            } else {
                                (other_transform.translation.xy(), 1.)
                            };

                        (
                            other_radius.0,
                            other_translation,
                            collision_delta_multiplier,
                        )
                    };

                    // This whole collision separation algorithm is taken and modified from https://www.youtube.com/watch?v=lS_qeBy3aQI at 4:09.
                    let radius_sum = radius + other_radius;

                    let collision_axis = translation - other_translation;

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
                        let translation_delta =
                            distance_delta * collision_axis * collision_delta_multiplier;
                        // Friction.
                        let velocity_delta =
                            velocity.abs() * velocity * 0.01 * COLLISION_TIME_DELTA_SECONDS as f32;

                        // By keeping translation up to date with deferred changes, we can massively improve collision resolution.
                        translation += translation_delta;
                        velocity -= velocity_delta;
                    }
                });

                collision_resolutions
                    .borrow_local_mut()
                    .push((entity, translation, velocity));
            });

            collision_resolutions
                .iter_mut()
                .for_each(|collision_resolutions| {
                    collision_resolutions
                        .drain(..)
                        .for_each(|(entity, translation, velocity)| {
                            let mut particle = world.entity_mut(entity);
                            let Some(mut particle) = particle.get_mut::<Verlet>() else {
                                return;
                            };
                            particle.translation = translation;
                            particle.velocity = velocity;
                        });
                });
        }
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

/// Applies a downward force to particles.
#[derive(Component)]
pub struct Gravity;

impl Gravity {
    // The force of gravity.
    pub const ACCELERATION: Vec2 = Vec2::new(0., -98.);

    /// Accelerates every particle downwards.
    pub fn update(mut particles: Query<&mut Verlet, With<Self>>) {
        particles.par_iter_mut().for_each(|mut particle| {
            particle.accelerate(Self::ACCELERATION);
        });
    }
}

/// Is the particle touching the ground?
#[derive(Component)]
pub struct Grounded(bool);

impl Grounded {
    /// Checks for any collision between a particle with Grounded and a non-particle.
    pub fn update(
        grid: Res<ColliderGrid>,
        mut particles: Query<(&mut Grounded, &Verlet, &Radius)>,
        colliders: Query<(&Radius, &Transform), Without<Verlet>>,
    ) {
        particles
            .par_iter_mut()
            .for_each(|(mut grounded, particle, radius)| {
                let Some(grid_index) = grid.translation_to_index(particle.translation) else {
                    return;
                };

                // If any collisions occur, we know we are grounded.
                grounded.0 = grid.cells[grid_index].0.iter().any(|other_entity| {
                    let Ok((other_radius, other_transform)) = colliders.get(*other_entity) else {
                        return false;
                    };

                    check_collision(
                        radius.0,
                        particle.translation,
                        other_radius.0,
                        other_transform.translation.xy(),
                    )
                });
            });
    }
}
