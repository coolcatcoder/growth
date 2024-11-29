use bevy::ecs::system::EntityCommands;

pub use crate::prelude::*;

pub mod prelude {
    //pub use super::{Leaf, Tree};
}

// Gets larger the more energy it has, so it can store more energy?
// Also so dang simple that it can't really get scrambled lol. Other than the question stuff, but it doesn't matter really. It will just try again next frame.
// Does energy have to be taken? Can you just take a bit, and let the rest pass?
// I'm starting to have doubt on this whole field scrambling...
#[derive(Component)]
pub struct Boulder {
    energy: f32,
    energy_capacity: f32,
    // Justification for scramble setting this to None. You could just check if the entity has the question component, to check for scramble.
    //action: BoulderAction,
    timer: EveryTime,
}

// Since we deprecated collision questions, I think this can be removed.
// enum BoulderAction {
//     None,
//     Move(Entity),
//     Seed(Entity),
//     Grow(Entity),
// }

impl Boulder {
    pub fn create(translation: Vec2, commands: &mut Commands, asset_server: &AssetServer) {
        const RADIUS: f32 = 30.;

        let mut rng = thread_rng();

        let energy_capacity = circle_to_energy(RADIUS);
        info!(energy_capacity);

        commands.spawn((
            Self {
                energy: 0.,
                energy_capacity,

                timer: EveryTime::new(
                    Duration::from_secs_f32(0.75),
                    Duration::from_secs_f32(rng.gen_range(0.0..0.75)),
                ),
            },
            Radius(RADIUS),
            SpriteBundle {
                texture: asset_server.load("nodule.png"),
                transform: Transform::from_translation(Vec3::new(translation.x, translation.y, 1.)),
                sprite: Sprite {
                    color: Color::Srgba(Srgba::rgb(0., 1., 1.)),
                    custom_size: Some(Vec2::splat(RADIUS * 2.)),
                    ..default()
                },
                ..default()
            },
        ));
    }

    //TODO: Rewrite to be from the suns perspective! That way they can just check for collisions whenever they move.
    // pub fn absorb_sun(
    //     mut boulders: Query<(Entity, &mut Boulder, &Collider)>,
    //     suns: Query<(Entity, &Sun)>,
    //     mut commands: Commands,
    // ) {
    //     boulders
    //         .iter_mut()
    //         .for_each(|(boulder_entity, mut boulder, sensor)| {
    //             sensor.collisions.iter().for_each(|collision| {
    //                 if let Ok((sun_entity, sun)) = suns.get(*collision) {
    //                     boulder.energy += sun.energy;
    //                     commands.entity(sun_entity).despawn();

    //                     // Scramble
    //                     boulder.action = BoulderAction::None;

    //                     if boulder.energy > boulder.energy_capacity {
    //                         commands.entity(boulder_entity).despawn();
    //                     }
    //                 }
    //             });
    //         });
    // }

    fn update(
        &mut self,
        entity: Entity,
        radius: f32,
        translation: Vec2,
        collider_grid: &ColliderGrid,
        colliders: &Query<(&Radius, &Transform)>,
        time_delta: Duration,
        mut commands: Commands,
    ) {
        let mut rng = thread_rng();

        self.timer.full_run(time_delta, || {
            match self.energy / self.energy_capacity {
                0.0..0.7 => {
                    // Move
                    let motion_range_x = -radius..radius;
                    let motion_range_y = -radius..(radius / 2.);

                    let search_radius = radius * 3.;

                    //TODO: This could subtract from the radius in the distance squared calculation?
                    let overlap = -5.;
                    let far = 100.;

                    let mut motion = None;
                    let mut attempts_left = 50;

                    while motion.is_none() && attempts_left != 0 {
                        let potential_motion = Vec2::new(
                            rng.gen_range(motion_range_x.clone()),
                            rng.gen_range(motion_range_y.clone()),
                        );
                        let collisions = collider_grid.get_collisions(
                            translation + potential_motion,
                            search_radius,
                            Some(entity),
                            colliders,
                        );

                        let mut not_too_close = true;
                        let mut not_too_far = false;
                        for colliding_entity in collisions.iter() {
                            let (other_radius, other_transform) =
                                colliders.get(*colliding_entity).unwrap();
                            let distance_squared = distance_squared_between_edges(
                                radius,
                                translation + potential_motion,
                                other_radius.0,
                                other_transform.translation.xy(),
                            );

                            if distance_squared == 0. {
                                //info!("Too close!");
                                //info!(distance_squared);
                                not_too_close = false;
                                break;
                            } else {
                                info!(distance_squared);
                            }

                            //info!("Does this happen?");

                            if distance_squared < squared(far) {
                                not_too_far = true;
                            }
                        }

                        if not_too_close && not_too_far {
                            info!("Success!");
                            motion = Some(potential_motion);
                        } else {
                            //info!("{}", question.translation - transform.translation.xy());
                            attempts_left -= 1;
                        }
                    }

                    if let Some(motion) = motion {
                        commands.entity(entity).mutate_component::<Transform>(
                            move |mut transform| {
                                transform.translation.x += motion.x;
                                transform.translation.y += motion.y;
                            },
                        );
                    }
                }
                0.7..1.0 => {
                    todo!("seed or grow")
                }
                _ => unreachable!(),
            }
        });

        // match boulder.action {
        //             BoulderAction::None => {
        //                 match boulder.energy / boulder.energy_capacity {
        //                     0.0..0.7 => {
        //                         boulder.action = BoulderAction::Move(
        //                             commands
        //                                 .spawn(boulder.generate_move_question(
        //                                     transform.translation.xy(),
        //                                     sprite.custom_size.unwrap().x / 2.,
        //                                 ))
        //                                 .id(),
        //                         );
        //                     }
        //                     0.7..0.9 => {
        //                         todo!();
        //                     }
        //                     0.9..1.0 => {
        //                         todo!();
        //                     }
        //                     _ => {
        //                         unreachable!("We should have matched from 0% to 100%");
        //                     }
        //                 }
        //                 // if boulder.energy / boulder.energy_capacity >= 0.7 {
        //                 //     if boulder.energy_capacity > 10. {
        //                 //         if rng.gen_bool(0.5) {
        //                 //             todo!("Get bigger.");
        //                 //         } else {
        //                 //             todo!("Make more.");
        //                 //         }
        //                 //     } else {
        //                 //         todo!("Get bigger.");
        //                 //         commands.spawn(CollisionQuestion::new(todo!(), todo!()));
        //                 //     }
        //                 // } else {}
        //             }

        //             BoulderAction::Grow(question) => {}

        //             BoulderAction::Move(question) => {
        //                 let mut question = distance_questions.get_mut(question).unwrap();
        //                 let Some(answer) = question.answer() else {
        //                     return;
        //                 };

        //                 boulder.action = BoulderAction::None;

        //                 let mut not_to_close = true;
        //                 let mut not_to_far = false;
        //                 for (colliding_entity, distance_squared) in answer.iter() {
        //                     if *colliding_entity == entity {
        //                         continue;
        //                     }

        //                     if *distance_squared < -squared(5.) {
        //                         //info!("Too close!");
        //                         //info!(distance_squared);
        //                         not_to_close = false;
        //                         break;
        //                     }

        //                     //info!("Does this happen?");

        //                     if *distance_squared < squared(5.) {
        //                         not_to_far = true;
        //                     }
        //                 }

        //                 if not_to_close && not_to_far {
        //                     //info!("Success!");
        //                     transform.translation.x = question.translation.x;
        //                     transform.translation.y = question.translation.y;
        //                 } else {
        //                     //info!("{}", question.translation - transform.translation.xy());
        //                 }
        //             }

        //             BoulderAction::Seed(question) => {}
        //         }
    }

    pub fn update_system(
        mut boulders: Query<(Entity, &mut Boulder, &Transform, &Radius)>,
        collider_grid: Res<ColliderGrid>,
        colliders: Query<(&Radius, &Transform)>,
        time: Res<Time>,
        mut commands: ParallelCommands,
    ) {
        boulders
            .par_iter_mut()
            .for_each(|(entity, mut boulder, mut transform, radius)| {
                commands.command_scope(|commands| {
                    boulder.update(
                        entity,
                        radius.0,
                        transform.translation.xy(),
                        &collider_grid,
                        &colliders,
                        time.delta(),
                        commands,
                    );
                });
            });
    }
}
