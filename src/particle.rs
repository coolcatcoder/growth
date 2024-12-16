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

#[derive(Default, Component)]
pub struct Verlet {
    previous_translation: Vec2,

    acceleration: Vec2,
}

impl Verlet {
    pub fn new(translation: Vec2) -> Self {
        Self {
            previous_translation: translation,
            acceleration: Vec2::ZERO,
        }
    }

    pub fn accelerate(&mut self, acceleration: Vec2) {
        self.acceleration += acceleration;
    }

    /// This has been passed from code base to codebase. No documentation has survived.
    fn update(&mut self, delta_time: f32, translation: &mut Vec2) {
        let acceleration = self.acceleration * (delta_time * delta_time);
        *translation = *translation + (*translation - self.previous_translation) + acceleration;

        self.previous_translation = *translation;
        self.acceleration = Vec2::ZERO;
    }

    pub fn system(mut particles: Query<(&mut Verlet, &mut Transform)>, time: Res<Time>) {
        particles
            .par_iter_mut()
            .for_each(|(mut particle, mut transform)| {
                particle.accelerate(Vec2::new(0., -10000.));

                let mut translation = transform.translation.xy();
                particle.update(time.delta_secs(), &mut translation);
                transform.translation.x = translation.x;
                transform.translation.y = translation.y;
            });
    }

    pub fn collide(
        commands: ParallelCommands,
        mut particles: Query<(Entity, &mut Verlet, &Transform)>,
        //time: Res<Time>,
        collider_grid: Res<ColliderGrid>,
        colliders: Query<(&Radius, &Transform)>,
    ) {
        particles
            .par_iter_mut()
            .for_each(|(entity, mut particle, transform)| {
                // TODO: Profile adding an if previous translation equals translation, then perhaps don't check for collisions?

                if collider_grid.collides_with_any(
                    transform.translation.xy(),
                    15.,
                    Some(entity),
                    &colliders,
                ) {
                    info!("!");
                    let previous_translation = particle.previous_translation;
                    particle.previous_translation = transform.translation.xy();
                    particle.accelerate(Vec2::new(0., 100000.));

                    commands.command_scope(|mut commands| {
                        commands
                            .get_entity(entity)
                            .unwrap()
                            .entry::<Transform>()
                            .and_modify(move |mut transform| {
                                transform.translation.x = previous_translation.x;
                                transform.translation.y = previous_translation.y;
                            });
                    });
                }
            });
    }
}

#[derive(Component)]
pub struct DistanceConstraint {
    pub distance: f32,
    pub target: Entity,
}

// impl DistanceConstraint {
//     pub fn solve(constraints: Query<(Entity, &DistanceConstraint, &Transform)>, transforms: Query<&Transform>, commands: ParallelCommands, time: Res<Time>) {
//         constraints.par_iter().for_each(|(entity, constraint, transform)| {
//             let mut translation = transform.translation.xy();
//             let target_translation = transforms.get(constraint.target).unwrap().translation.xy();
//             let direction = (translation - target_translation).normalize_or_zero();
//             let distance = translation.distance(target_translation);

//             // We move in distance inthe direction so we arrive at the tranform, we then move back to be at the desired distance.
//             translation += direction * (distance - constraint.distance);

//             commands.command_scope(|mut commands| {
//                 commands.entity(entity).mutate_component::<Transform>(move |mut transform| {
//                     transform.translation.x = translation.x;
//                     transform.translation.y = translation.y;
//                 });
//             });
//         });
//     }
// }

impl DistanceConstraint {
    pub fn solve(
        constraints: Query<(Entity, &DistanceConstraint, &Transform)>,
        transforms: Query<&Transform>,
        commands: ParallelCommands,
        time: Res<Time>,
    ) {
        // TODO: Work out if order matters.
        constraints
            .par_iter()
            .for_each(|(entity, constraint, transform)| {
                let mut translation = transform.translation.xy();
                let target_translation =
                    transforms.get(constraint.target).unwrap().translation.xy();

                // Because I don't know math, and I don't know how to search the internet, this is written by chatgpt.
                let distance = translation.distance(target_translation);
                let distance_x = translation.x - target_translation.x;
                let distance_y = translation.y - target_translation.y;
                let unit_distance_x = distance_x / distance;
                let unit_distance_y = distance_y / distance;
                let scaled_distance_x = unit_distance_x * constraint.distance;
                let scaled_distance_y = unit_distance_y * constraint.distance;

                // We move in distance inthe direction so we arrive at the tranform, we then move back to be at the desired distance.
                translation = Vec2::new(
                    target_translation.x + scaled_distance_x,
                    target_translation.y + scaled_distance_y,
                );

                commands.command_scope(|mut commands| {
                    commands.entity(entity).entry::<Transform>().and_modify(
                        move |mut transform| {
                            transform.translation.x = translation.x;
                            transform.translation.y = translation.y;
                        },
                    );
                });
            });
    }
}

// Moves returns translation moves so that it is the desired distance away from target translation.
fn distance_constraint(distance_desired: f32, translation: Vec2, target_translation: Vec2) -> Vec2 {
    // TODO: Work out why this is required.
    if distance_desired == 0. {
        return target_translation;
    }

    // Because I don't know math, and I don't know how to search the internet, this is written by chatgpt.
    let distance = translation.distance(target_translation);
    let distance_x = translation.x - target_translation.x;
    let distance_y = translation.y - target_translation.y;
    let unit_distance_x = distance_x / distance;
    let unit_distance_y = distance_y / distance;
    let scaled_distance_x = unit_distance_x * distance_desired;
    let scaled_distance_y = unit_distance_y * distance_desired;

    // We move in distance inthe direction so we arrive at the tranform, we then move back to be at the desired distance.
    Vec2::new(
        target_translation.x + scaled_distance_x,
        target_translation.y + scaled_distance_y,
    )
}

/// Chains entities between anchor and target.
/// If target cannot be reached, the chain will still remain anchored.
#[derive(Component)]
pub struct Chain {
    // Anchor is a seperate entity because we may want multiple chains on one anchor.
    pub anchor: Entity,
    /// The links of the chain.
    /// In order of closest to anchor to closest to target. Roughly.
    /// The translation is the source of truth, I think. Transform's translation's xy will be set to it.
    /// (distance_to_previous, entity, translation)
    // TODO: How does the last link interact with target?
    pub links: Vec<(f32, Entity, Vec2)>,
    pub target: Option<Entity>,
}

impl Chain {
    pub fn solve(
        mut chains: Query<&mut Chain>,
        transforms: Query<&Transform>,
        commands: ParallelCommands,
    ) {
        chains.par_iter_mut().for_each(|mut chain| {
            let anchor_translation = transforms.get(chain.anchor).unwrap().translation.xy();
            if let Some(target) = chain.target {
                let target_translation = transforms.get(target).unwrap().translation.xy();
                // Because deferred mutation can occur inside this for loop, we can NEVER query a link's transform inside of it.
                // We just change the source of truth to the chain to deal with it.
                // TODO: Five steps is arbitrary. We may wish to actually detect a convergence? But that may not happen if target is too far away.
                for _ in 0..1 {
                    // Towards target.
                    // Solving it from target back towards anchor, because I think that makes the most sense, so that both go outwards towards the other.

                    let mut previous_translation = target_translation;
                    // Does the last link exactly match the target? I think so, but I'm not sure.
                    let mut distance = 0.;

                    chain.links.iter_mut().rev().for_each(
                        |(next_distance, entity, translation)| {
                            *translation =
                                distance_constraint(distance, *translation, previous_translation);
                            previous_translation = *translation;
                            distance = *next_distance;
                        },
                    );

                    // Back towards anchor.
                    // Must be last, so we are always connected to anchor.

                    let mut previous_translation = anchor_translation;

                    chain
                        .links
                        .iter_mut()
                        .for_each(|(distance, entity, translation)| {
                            *translation =
                                distance_constraint(*distance, *translation, previous_translation);
                            previous_translation = *translation;
                        });
                }
            } else {
                // All we need to do is make sure that the distance constraints anchored to anchor are solved.
                let mut previous_translation = anchor_translation;

                chain
                    .links
                    .iter_mut()
                    .for_each(|(distance, entity, translation)| {
                        *translation =
                            distance_constraint(*distance, *translation, previous_translation);
                        previous_translation = *translation;
                    });
            }

            chain.links.iter().for_each(|(_, entity, translation)| {
                // Borrow checker manipulation.
                let entity = *entity;
                let translation = *translation;
                commands.command_scope(move |mut commands| {
                    commands.entity(entity).entry::<Transform>().and_modify(
                        move |mut transform| {
                            transform.translation.x = translation.x;
                            transform.translation.y = translation.y;
                        },
                    );
                });
            });
        });
    }
}
