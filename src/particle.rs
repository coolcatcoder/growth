pub use crate::prelude::*;

//MARK: ParticleTicker
#[derive(Component)]
pub struct Ticker(pub EveryTime);

impl Ticker {
    pub fn update_time(time: Res<Time>, mut particles: Query<&mut Ticker>) {
        particles
            .par_iter_mut()
            .for_each(|mut particle| particle.0.tick(time.delta()));
    }

    pub fn finish(mut particles: Query<&mut Ticker>) {
        particles
            .par_iter_mut()
            .for_each(|mut particle| particle.0.finish_running());
    }
}

//MARK: AirFriction
#[derive(Component)]
pub struct AmbientFriction(pub Vec2);

impl AmbientFriction {
    pub fn system(mut particles: Query<(&AmbientFriction, &mut Motion, &Ticker)>) {
        particles
            .par_iter_mut()
            .for_each(|(friction, mut motion, ticker)| {
                let motion = motion.into_inner();
                ticker.0.run(|| {
                    motion.amount -= motion.amount.abs() * motion.amount * friction.0;
                });
            });
    }
}

//MARK: StepUp
#[derive(Component)]
pub struct StepUp(pub f32);

impl StepUp {
    pub fn system(
        collider_grid: Res<ColliderGrid>,
        mut particles: Query<(
            Entity,
            &mut Transform,
            &mut Motion,
            &Collider,
            &StepUp,
            &Ticker,
        )>,
        colliders: Query<(&Collider, &Transform), Without<StepUp>>,
    ) {
        particles.par_iter_mut().for_each(
            |(entity, mut transform, mut motion, collider, step_up, ticker)| {
                ticker.0.run(|| {
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
            },
        );
    }
}

//MARK: Motion
#[derive(Component)]
pub struct Motion {
    pub amount: Vec2,
    pub enabled: [bool; 2],
}

impl Motion {
    pub fn new(amount: Vec2, enabled: [bool; 2]) -> Self {
        Self { amount, enabled }
    }

    pub fn system(mut particles: Query<(&mut Transform, &Motion, &Ticker)>) {
        particles
            .par_iter_mut()
            .for_each(|(mut transform, motion, particle_ticker)| {
                particle_ticker.0.run(|| {
                    if motion.enabled[0] {
                        transform.translation.x += motion.amount.x;
                    }
                    if motion.enabled[1] {
                        transform.translation.y += motion.amount.y;
                    }
                })
            });
    }
}

//MARK: MotionCollid
#[derive(Component)]
pub struct DisableMotionOnCollision;

impl DisableMotionOnCollision {
    pub fn system(
        collider_grid: Res<ColliderGrid>,
        mut particles: Query<
            (Entity, &Transform, &mut Motion, &Collider, &Ticker),
            With<DisableMotionOnCollision>,
        >,
        colliders: Query<(&Collider, &Transform)>,
    ) {
        particles
            .par_iter_mut()
            .for_each(|(entity, transform, mut motion, collider, ticker)| {
                ticker.0.run(|| {
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
            });
    }
}
