pub use crate::prelude::*;

pub mod prelude {
    pub use super::{
        air_friction, disable_motion_on_collide, particle_motion, step_up, AirFriction, Motion,
        StepUp, ParticleTicker, tick_particles, finish_running_particles,
    };
}

//MARK: ParticleTicker
#[derive(Component)]
pub struct ParticleTicker(pub EveryTime);

pub fn tick_particles(time: Res<Time>, mut particles: Query<&mut ParticleTicker>) {
    particles.par_iter_mut().for_each(|mut particle| particle.0.tick(time.delta()));
}

pub fn finish_running_particles(mut particles: Query<&mut ParticleTicker>) {
    particles.par_iter_mut().for_each(|mut particle| particle.0.finish_running());
}

//MARK: AirFriction
#[derive(Component)]
pub struct AirFriction(Vec2, usize);

impl AirFriction {
    pub fn new(amount: Vec2) -> Self {
        Self(amount, 0)
    }
}

pub fn air_friction(mut particles: Query<(&mut AirFriction, &mut Motion)>) {
    particles
        .par_iter_mut()
        .for_each(|(mut friction, mut motion)| {});
}

//MARK: StepUp
#[derive(Component)]
pub struct StepUp(pub f32);

#[derive(Component)]
pub struct Motion {
    pub amount: Vec2,
    pub every: EveryTime,
    pub enabled: [bool; 2],
}

impl Motion {
    pub fn new(amount: Vec2, every: EveryTime, enabled: [bool; 2]) -> Self {
        Self {
            amount,
            every,
            enabled,
        }
    }
}

pub fn disable_motion_on_collide(
    collider_grid: Res<ColliderGrid>,
    mut particles: Query<(Entity, &Transform, &mut Motion, &Collider)>,
    colliders: Query<(&Collider, &Transform)>,
) {
    particles
        .par_iter_mut()
        .for_each(|(entity, transform, mut motion, collider)| {
            if motion.amount == Vec2::ZERO {
                return;
            }

            motion.enabled = [true, true];

            let all_axes_colliding = collider_grid.collides_with_any(
                transform.translation.xy() + motion.amount,
                collider.radius,
                Some(entity),
                &colliders,
            );

            if all_axes_colliding {
                motion.enabled[0] = !collider_grid.collides_with_any(
                    transform.translation.xy() + Vec2::new(motion.amount.x, 0.),
                    collider.radius,
                    Some(entity),
                    &colliders,
                );

                motion.enabled[1] = !collider_grid.collides_with_any(
                    transform.translation.xy() + Vec2::new(0., motion.amount.y),
                    collider.radius,
                    Some(entity),
                    &colliders,
                );
            }
        });
}

pub fn step_up(
    collider_grid: Res<ColliderGrid>,
    mut particles: Query<(Entity, &mut Transform, &mut Motion, &Collider, &StepUp)>,
    colliders: Query<(&Collider, &Transform), Without<StepUp>>,
) {
    particles
        .par_iter_mut()
        .for_each(|(entity, mut transform, mut motion, collider, step_up)| {
            if !motion.enabled[0] {
                // Should we take the y motion into account, so we don't accidentally fall through the floor perhaps?
                let minimum_y_translation = collider_grid
                    .no_collisions_minimum_y_translation_with_limit(
                        transform.translation.xy() + Vec2::new(motion.amount.x, 0.),
                        collider.radius,
                        step_up.0,
                        Some(entity),
                        &colliders,
                    );

                if minimum_y_translation <= step_up.0 {
                    motion.enabled[0] = true;
                    // Shouldn't this only happen when things move, which might not be very often? Or is this ok?
                    transform.translation.y += minimum_y_translation;
                }
            }
        });
}

pub fn particle_motion(time: Res<Time>, mut particles: Query<(&mut Transform, &mut Motion)>) {
    particles
        .par_iter_mut()
        .for_each(|(mut transform, motion)| {
            let motion = motion.into_inner();

            motion.every.full_run(time.delta(), || {
                if motion.enabled[0] {
                    transform.translation.x += motion.amount.x;
                }
                if motion.enabled[1] {
                    transform.translation.y += motion.amount.y;
                }
            });
        });
}

// Fun experiment, unique tuples. A replacement for query lens stuff.

// (B, G) from (B, F, G)
pub trait FromUniqueTuple<T> {
    fn from_unique_tuple(value: T) -> Self;
}

pub trait ToUniqueTuple<T> {
    fn to_unique_tuple(self) -> T;
}

impl<T, U> ToUniqueTuple<U> for T
where
    U: FromUniqueTuple<T>,
{
    fn to_unique_tuple(self) -> U {
        U::from_unique_tuple(self)
    }
}

impl<A, B> FromUniqueTuple<(A, B)> for A {
    fn from_unique_tuple(value: (A, B)) -> Self {
        value.0
    }
}
