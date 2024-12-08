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
    pub fn motion(mut particles: Query<(&AmbientFriction, &mut Motion, &Ticker)>) {
        particles
            .par_iter_mut()
            .for_each(|(friction, motion, ticker)| {
                let motion = motion.into_inner();
                ticker.0.run(|| {
                    motion.amount -= motion.amount.abs() * motion.amount * friction.0;
                });
            });
    }

    pub fn velocity(mut particles: Query<(&AmbientFriction, &mut Velocity, &Ticker)>) {
        particles
            .par_iter_mut()
            .for_each(|(friction, velocity, ticker)| {
                let velocity = velocity.into_inner();
                ticker.0.run(|| {
                    velocity.0 -= velocity.abs() * velocity.0 * friction.0;
                });
            });
    }
}

//MARK: StepUp
#[derive(Component)]
pub struct StepUp(pub f32);

impl StepUp {
    pub fn motion(
        collider_grid: Res<ColliderGrid>,
        mut particles: Query<(
            Entity,
            &mut Transform,
            &mut Motion,
            &Radius,
            &StepUp,
            &Ticker,
        )>,
        colliders: Query<(&Radius, &Transform), Without<StepUp>>,
    ) {
        particles.par_iter_mut().for_each(
            |(entity, mut transform, mut motion, collider, step_up, ticker)| {
                ticker.0.run(|| {
                    if !motion.enabled[0] {
                        // Should we take the y motion into account, so we don't accidentally fall through the floor perhaps?
                        let minimum_y_translation = collider_grid
                            .no_collisions_minimum_y_translation_with_limit(
                                transform.translation.xy() + Vec2::new(motion.amount.x, 0.),
                                collider.0,
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

//MARK: Velocity
// Different from motion only slightly, as it can't be enabled or disabled, and should instead be set to 0.
#[derive(Component, Deref, DerefMut)]
pub struct Velocity(pub Vec2);

impl Velocity {
    pub fn system(mut particles: Query<(&mut Transform, &Velocity, &Ticker)>) {
        particles
            .par_iter_mut()
            .for_each(|(mut transform, velocity, particle_ticker)| {
                particle_ticker.0.run(|| {
                    transform.translation.x += velocity.x;
                    transform.translation.y += velocity.y;
                })
            });
    }
}

//MARK:StopCollision
#[derive(Component)]
pub struct StopOnCollision;

impl StopOnCollision {
    pub fn motion(
        collider_grid: Res<ColliderGrid>,
        mut particles: Query<
            (Entity, &Transform, &mut Motion, &Radius, &Ticker),
            With<StopOnCollision>,
        >,
        colliders: Query<(&Radius, &Transform)>,
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
                        collider.0,
                        Some(entity),
                        &colliders,
                    );

                    if all_axes_colliding {
                        motion.enabled[0] = !collider_grid.collides_with_any(
                            transform.translation.xy() + Vec2::new(motion.amount.x, 0.),
                            collider.0,
                            Some(entity),
                            &colliders,
                        );

                        motion.enabled[1] = !collider_grid.collides_with_any(
                            transform.translation.xy() + Vec2::new(0., motion.amount.y),
                            collider.0,
                            Some(entity),
                            &colliders,
                        );
                    }
                });
            });
    }

    // TODO: Currently we are using a janky hack to get step up working with velocity.
    pub fn velocity(
        collider_grid: Res<ColliderGrid>,
        mut particles: Query<
            (
                Entity,
                &Transform,
                &mut Velocity,
                &Radius,
                Option<&StepUp>,
                &Ticker,
            ),
            With<StopOnCollision>,
        >,
        colliders: Query<(&Radius, &Transform)>,
    ) {
        particles.par_iter_mut().for_each(
            |(entity, transform, mut velocity, collider, step_up, ticker)| {
                ticker.0.run(|| {
                    if **velocity == Vec2::ZERO {
                        return;
                    }

                    let all_axes_colliding = collider_grid.collides_with_any(
                        transform.translation.xy() + **velocity,
                        collider.0,
                        Some(entity),
                        &colliders,
                    );

                    if all_axes_colliding {
                        if collider_grid.collides_with_any(
                            transform.translation.xy() + Vec2::new(0., velocity.y),
                            collider.0,
                            Some(entity),
                            &colliders,
                        ) {
                            velocity.y = 0.;
                        }

                        if collider_grid.collides_with_any(
                            transform.translation.xy() + Vec2::new(velocity.x, 0.),
                            collider.0,
                            Some(entity),
                            &colliders,
                        ) {
                            if let Some(step_up) = step_up {
                                // Should we take the y velocity into account, so we don't accidentally fall through the floor perhaps?
                                // let minimum_y_translation = collider_grid
                                //     .no_collisions_minimum_y_translation_with_limit(
                                //         transform.translation.xy() + Vec2::new(velocity.x, 0.),
                                //         collider.0,
                                //         step_up.0,
                                //         Some(entity),
                                //         &colliders,
                                //     );

                                // if minimum_y_translation <= step_up.0 {
                                //     velocity.y += minimum_y_translation;
                                // } else {
                                //     velocity.x = 0.;
                                // }

                                //TODO: The above was so horrible, I've resorted to this...
                                velocity.y += 5.;
                                velocity.x = 0.;
                            } else {
                                velocity.x = 0.;
                            }
                        }
                    }
                });
            },
        );
    }
}
