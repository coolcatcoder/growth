use bevy::ecs::system::EntityCommands;

pub use crate::prelude::*;

pub mod prelude {
    //pub use super::{Leaf, Tree};
}

// Out of curiosity, could I have one Plant struct, that has lots of optional structs that work with it? To create behaviour by composition?
// I don't know. I may fall back to the ancient grow code for now.

// This just marks a nodule as a plant.
pub struct Plant;

// Do we need this?
pub struct Base;

// Entities with this just kinda exist in the void perhaps? Used to create offshoots when something asks them to?
// This is all really experimental...
pub struct OffShoot {
    length: usize,
    ends: Vec<Entity>,
}

#[derive(Component)]
pub struct WibblyGrass {}

impl WibblyGrass {
    const REST_Y: f32 = 120.;
    const DETECTION_DISTANCE: f32 = 100.;

    pub fn sway(
        grass: Query<(&particle::Chain, &Transform), With<WibblyGrass>>,
        players: Query<&Transform, With<Player>>,
        transforms: Query<&Transform>,
        time: Res<Time>,
        commands: ParallelCommands,
    ) {
        let delta = time.delta_secs();

        grass.par_iter().for_each(|(chain, anchor_transform)| {
            let mut closest_player: Option<(f32, Vec2)> = None;
            players.iter().for_each(|player_transform| {
                let distance_squared = player_transform
                    .translation
                    .xy()
                    .distance_squared(anchor_transform.translation.xy());

                if distance_squared < squared(Self::DETECTION_DISTANCE) {
                    if let Some(closest_player) = &mut closest_player {
                        if distance_squared < closest_player.0 {
                            *closest_player = (distance_squared, player_transform.translation.xy());
                        }
                    } else {
                        closest_player =
                            Some((distance_squared, player_transform.translation.xy()));
                    }
                }
            });

            if let Some(closest_player) = closest_player {
                commands.command_scope(move |mut commands| {
                    commands
                        .entity(chain.target.unwrap())
                        .entry::<Transform>()
                        .and_modify(move |mut transform| {
                            let mut translation = transform.translation.xy();
                            let direction = (closest_player.1 - translation).normalize_or_zero();
                            translation += direction * 100. * delta;
                            transform.translation.x = translation.x;
                            transform.translation.y = translation.y;
                        });
                });
            } else {
                let rest_translation =
                    anchor_transform.translation.xy() + Vec2::new(0., Self::REST_Y);
                commands.command_scope(move |mut commands| {
                    commands
                        .entity(chain.target.unwrap())
                        .entry::<Transform>()
                        .and_modify(move |mut transform| {
                            let mut translation = transform.translation.xy();
                            let direction = (rest_translation - translation).normalize_or_zero();
                            translation += direction * 100. * delta;
                            transform.translation.x = translation.x;
                            transform.translation.y = translation.y;
                        });
                });
            }
        });
    }

    // pub fn create<const X: i32, const WIDTH: u16>(info: LineCustomiserInfo) -> bool {
    //     if info.nodule_translation.y == 0 && (info.translation.x - X as f32).abs() < 60. {
    //         info.terrain
    //             .create(
    //                 NoduleConfig {
    //                     depth: 1.,
    //                     colour: [0.21, 0., 0.25],
    //                     collision: false,
    //                     ..default()
    //                 },
    //                 info.translation,
    //             )
    //             .entry::<Sprite>()
    //             .and_modify(move |mut sprite| {
    //                 sprite.custom_size = Some(Vec2::new(WIDTH as f32, 30.));
    //             });

    //         for x in 0..5 {
    //             let x = x as f32 * 10. - (WIDTH as f32 / 2.);

    //             let mut links = Vec::with_capacity(15);
    //             for y in 1..=15 {
    //                 let translation = info.translation + Vec2::new(x, y as f32 * 5.);

    //                 let link = info
    //                     .terrain
    //                     .create(
    //                         NoduleConfig {
    //                             depth: 1.,
    //                             colour: [0.21, 0., 0.25],
    //                             collision: false,
    //                             diameter: 5.,
    //                             ..default()
    //                         },
    //                         translation,
    //                     )
    //                     .id();

    //                 links.push((5., link, translation));
    //             }

    //             let target = info
    //                 .terrain
    //                 .commands
    //                 .spawn(TransformBundle::from_transform(
    //                     Transform::from_translation(Vec3::new(
    //                         info.translation.x,
    //                         info.translation.y + Self::REST_Y,
    //                         0.,
    //                     )),
    //                 ))
    //                 .id();

    //             let mut anchor = info.terrain.commands.spawn(TransformBundle::from_transform(
    //                 Transform::from_translation(Vec3::new(
    //                     info.translation.x + x,
    //                     info.translation.y,
    //                     0.,
    //                 )),
    //             ));

    //             let anchor_id = anchor.id();

    //             anchor.insert((
    //                 particle::Chain {
    //                     anchor: anchor_id,
    //                     links,
    //                     target: Some(target),
    //                 },
    //                 WibblyGrass {},
    //             ));
    //         }
    //         true
    //     } else {
    //         false
    //     }
    // }
}

// IMPORTANT: I think I might abandon this... I kinda want grass that looks more similar to grass in rainworld. It would also be more performant.
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

    // The current direction it wants to move.
    direction: bool,
}

// Since we deprecated collision questions, I think this can be removed.
// enum BoulderAction {
//     None,
//     Move(Entity),
//     Seed(Entity),
//     Grow(Entity),
// }

impl Boulder {
    const DEBUG: bool = false;

    pub fn create(translation: Vec2, commands: &mut Commands, asset_server: &AssetServer) {
        const RADIUS: f32 = 30.;

        let mut rng = thread_rng();

        let energy_capacity = circle_to_energy(RADIUS);
        if Self::DEBUG {
            info!(energy_capacity);
        }

        commands.spawn((
            Self {
                energy: 0.,
                energy_capacity,

                timer: EveryTime::new(
                    Duration::from_secs_f32(0.75),
                    Duration::from_secs_f32(rng.gen_range(0.0..0.75)),
                ),

                direction: rng.gen_bool(0.5),
            },
            Radius(RADIUS),
            Transform::from_translation(Vec3::new(translation.x, translation.y, 1.)),
            Sprite {
                image: asset_server.load("nodule.png"),
                color: Color::Srgba(Srgba::rgb(0., 1., 1.)),
                custom_size: Some(Vec2::splat(RADIUS * 2.)),
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
        gizmos: &GizmosLingering,
    ) {
        let mut rng = thread_rng();

        self.timer.full_run(time_delta, || {
            match self.energy / self.energy_capacity {
                0.0..0.7 => {
                    // Move
                    let motion_range_x = if self.direction {
                        1.0..radius
                    } else {
                        -radius..-1.0
                    };
                    let motion_range_y = -radius..(radius / 2.);

                    let search_radius = radius * 3.;

                    //TODO: This could subtract from the radius in the distance squared calculation?
                    let overlap = -5.;
                    let far = 5.;

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
                            let (other_radius, other_translation) = {
                                let (other_radius, other_transform) =
                                    colliders.get(*colliding_entity).unwrap();

                                (other_radius.0, other_transform.translation.xy())
                            };

                            let distance_between_edges = distance_between_edges(
                                radius,
                                translation + potential_motion,
                                other_radius,
                                other_translation,
                            );

                            if distance_between_edges == 0. {
                                if Self::DEBUG {
                                    //info!("Too close!");
                                    //info!(distance_squared);
                                }
                                gizmos.add(Duration::from_secs(1), move |gizmos| {
                                    gizmos.circle_2d(
                                        other_translation,
                                        other_radius * 0.8,
                                        Color::srgb(1., 0., 1.),
                                    );
                                });
                                not_too_close = false;
                                break;
                            } else if Self::DEBUG {
                                //info!(distance_between_edges);
                                gizmos.add(Duration::from_secs(1), move |gizmos| {
                                    gizmos.circle_2d(
                                        other_translation,
                                        other_radius,
                                        Color::srgb(1., 0., 0.),
                                    );
                                });
                            }

                            if distance_between_edges < far {
                                if Self::DEBUG {
                                    gizmos.add(Duration::from_secs(1), move |gizmos| {
                                        gizmos.circle_2d(
                                            other_translation,
                                            other_radius * 0.6,
                                            Color::srgb(1., 1., 0.),
                                        );
                                    });
                                }
                                not_too_far = true;
                            }
                        }

                        if not_too_close && not_too_far {
                            if Self::DEBUG {
                                //info!("Success!");
                            }

                            motion = Some(potential_motion);
                        } else {
                            attempts_left -= 1;
                        }
                    }

                    if let Some(motion) = motion {
                        commands.entity(entity).entry::<Transform>().and_modify(
                            move |mut transform| {
                                transform.translation.x += motion.x;
                                transform.translation.y += motion.y;
                            },
                        );
                    } else {
                        if Self::DEBUG {
                            info!("Failure!");
                        }
                        self.direction = !self.direction;
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
        gizmos: Res<GizmosLingering>,
    ) {
        //TODO: Remove gizmos and make this parallel.
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
                        &gizmos,
                    );
                });
            });
    }
}
