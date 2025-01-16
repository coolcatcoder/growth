use bevy::ecs::system::SystemState;

use crate::prelude::*;

pub mod prelude {
    pub use super::{AmbientFriction, Extrapolate, Gravity, Verlet};
}

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
    fn velocity_delta_from_friction(&self, friction: f32, time_delta_seconds: f32) -> Vec2 {
        self.velocity.abs() * self.velocity * friction * time_delta_seconds
    }
}

/// Updates all the particles to their next positions.
#[system(Update::Physics::Update)]
fn verlet_update(mut particles: Query<&mut Verlet>, time: Res<Time>) {
    let time_delta_seconds = time.delta_secs();
    let halfed_after_squared_time_delta_seconds = time_delta_seconds * time_delta_seconds * 0.5;
    particles.par_iter_mut().for_each(|mut particle| {
        let velocity = particle.velocity;
        let acceleration = particle.acceleration;

        particle.translation +=
            velocity * time_delta_seconds + acceleration * halfed_after_squared_time_delta_seconds;
        particle.velocity += acceleration * time_delta_seconds;
        particle.acceleration = Vec2::ZERO;
    });
}

/// Stop particles from intersecting each other.
#[system(Update::Physics::CollisionResolution)]
fn solve_collisions(
    world: &mut World,
    system: &mut SystemState<(
        Query<(Entity, &Radius, &Verlet)>,
        Query<(&Radius, &Transform, Option<&Verlet>)>,
        Res<ColliderGrid>,
    )>,

    // (entity, translation, velocity)
    // Every entity should appear at most once in here.
    // TODO: We could store a component on each particle, that we mutate. Then we iter over all components and set verlet.translation to equal them.
    // I would prefer to avoid the extra memory if possible though.
    mut collision_resolutions: Local<Parallel<Vec<(Entity, Vec2, Vec2)>>>,
) {
    const COLLISION_SUBSTEPS: u8 = 3;
    let time = world.get_resource::<Time>().unwrap();
    let time_delta_seconds = time.delta_secs() / COLLISION_SUBSTEPS as f32;

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
                        velocity.abs() * velocity * 0.01 * time_delta_seconds as f32;

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

/// Sets Transform's translation to the particle's translation.
/// The transform is still not a source of truth, as during the non-fixed update it may use interpolation/extrapolation to keep everything looking pleasant.
#[system(Update::Physics::SyncPositions)]
fn verlet_sync_positions(mut particles: Query<(&Verlet, &mut Transform)>) {
    particles
        .par_iter_mut()
        .for_each(|(particle, mut transform)| {
            transform.translation.x = particle.translation.x;
            transform.translation.y = particle.translation.y;
        });
}

//MARK: AmbientFriction
/// Applies a small amount of friction every update.
#[derive(Component)]
pub struct AmbientFriction;

/// Applies the friction.
#[system(Update::Physics::BeforeUpdate)]
fn ambient_friction(mut particles: Query<&mut Verlet, With<AmbientFriction>>, time: Res<Time>) {
    let time_delta_seconds = time.delta_secs();
    particles.par_iter_mut().for_each(|mut particle| {
        let velocity_delta = particle.velocity_delta_from_friction(0.005, time_delta_seconds);
        particle.velocity -= velocity_delta;
    });
}

/// Applies a downward force to particles.
#[derive(Component)]
pub struct Gravity;

impl Gravity {
    // The force of gravity.
    const ACCELERATION: Vec2 = Vec2::new(0., -98.);
}

/// Accelerates every particle downwards.
#[system(Update::Physics::BeforeUpdate)]
fn update(mut particles: Query<&mut Verlet, With<Gravity>>) {
    particles.par_iter_mut().for_each(|mut particle| {
        particle.accelerate(Gravity::ACCELERATION);
    });
}

/// Extrapolates the transform's motion from Verlet.
#[derive(Component)]
pub struct Extrapolate;

#[system(Update::Early)]
fn extrapolate(
    mut particles: Query<(&Verlet, &Radius, &mut Transform), With<Extrapolate>>,
    colliders: Query<(&Radius, &Transform), Without<Extrapolate>>,
    time: Res<Time>,
    grid: Res<ColliderGrid>,
) {
    let time_delta_seconds = time.delta_secs();
    let halfed_after_squared_time_delta_seconds = time_delta_seconds * time_delta_seconds * 0.5;

    particles
        .par_iter_mut()
        .for_each(|(particle, radius, mut transform)| {
            let translation_delta = particle.velocity * time_delta_seconds
                + particle.acceleration * halfed_after_squared_time_delta_seconds;

            let translation = transform.translation.xy() + translation_delta;

            let collision = if let Some(grid_index) = grid.translation_to_index(translation) {
                // Prevent jittering.
                let radius = radius.0 * 1.1;
                grid.cells[grid_index].0.iter().any(|other_entity| {
                    let Ok((other_radius, other_transform)) = colliders.get(*other_entity) else {
                        return false;
                    };

                    check_collision(
                        radius,
                        particle.translation,
                        other_radius.0,
                        other_transform.translation.xy(),
                    )
                })
            } else {
                // There cannot be collisions outside of the collision grid.
                false
            };

            // If there is not a collision then we can update the transform.
            if !collision {
                transform.translation.x += translation_delta.x;
                transform.translation.y += translation_delta.y;
            }
        });
}

/// Chains 2 particles together.
/// Taken from https://www.youtube.com/watch?v=lS_qeBy3aQI
#[derive(Component)]
pub struct Chain {
    particle_1: Entity,
    particle_2: Entity,
    target_distance: f32,
}

#[system(Update::Physics::Chain)]
fn chain(
    chains: Query<&Chain>,
    mut particles: ParamSet<(Query<&Verlet>, Query<&mut Verlet>)>,
    mut translations: Local<Parallel<Vec<(Entity, Vec2, Entity, Vec2)>>>,
) {
    let particles_immutable = particles.p0();
    chains.par_iter().for_each(|chain| {
        let Ok([particle_1, particle_2]) =
            particles_immutable.get_many([chain.particle_1, chain.particle_2])
        else {
            return;
        };

        let axis = particle_1.translation - particle_2.translation;
        let distance = axis.length();
        let axis_normalised = axis / distance;
        let translation_delta = chain.target_distance - distance;

        let translations_tuple = (
            chain.particle_1,
            particle_1.translation + 0.5 * translation_delta * axis_normalised,
            chain.particle_2,
            particle_2.translation - 0.5 * translation_delta * axis_normalised,
        );

        translations.borrow_local_mut().push(translations_tuple);
    });

    // Get the mutable query.
    let mut particles = particles.p1();

    // Apply all translations in a singlethreaded fashion.
    translations.iter_mut().for_each(|translations| {
        translations.drain(..).for_each(
            |(particle_1, translation_1, particle_2, translation_2)| {
                let Ok([mut particle_1, mut particle_2]) =
                    particles.get_many_mut([particle_1, particle_2])
                else {
                    return;
                };

                particle_1.translation = translation_1;
                particle_2.translation = translation_2;
            },
        );
    });
}
