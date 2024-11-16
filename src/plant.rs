pub use crate::prelude::*;

pub mod prelude {
    //pub use super::{Leaf, Tree};
}

// Gets larger the more energy it has, so it can store more energy?
// Also so dang simple that it can't really get scrambled lol. Other than the question stuff, but it doesn't matter really. It will just try again next frame.
// Does energy have to be taken? Can you just take a bit, and let the rest pass?
#[derive(Component)]
pub struct Boulder {
    energy: f32,
    energy_capacity: f32,

    // Justification for scramble setting this to None. You could just check if the entity has the question component, to check for scramble.
    action: BoulderAction,
}

enum BoulderAction {
    None,
    Move(Entity),
    Seed(Entity),
    Grow(Entity),
}

impl Boulder {
    pub fn create(translation: Vec2, commands: &mut Commands, asset_server: &AssetServer) {
        const RADIUS: f32 = 30.;

        let energy_capacity = circle_to_energy(RADIUS);
        info!(energy_capacity);

        commands.spawn((
            Self {
                energy: 0.,
                energy_capacity,
                action: BoulderAction::None,
            },
            CollisionSensor::default(),
            Collider::radius(RADIUS),
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

    pub fn absorb_sun(
        mut boulders: Query<(Entity, &mut Boulder, &CollisionSensor)>,
        suns: Query<(Entity, &Sun)>,
        mut commands: Commands,
    ) {
        boulders
            .iter_mut()
            .for_each(|(boulder_entity, mut boulder, sensor)| {
                sensor.collisions.iter().for_each(|collision| {
                    if let Ok((sun_entity, sun)) = suns.get(*collision) {
                        boulder.energy += sun.energy;
                        commands.entity(sun_entity).despawn();

                        // Scramble
                        boulder.action = BoulderAction::None;

                        if boulder.energy > boulder.energy_capacity {
                            commands.entity(boulder_entity).despawn();
                        }
                    }
                });
            });
    }

    pub fn update(
        mut boulders: Query<(Entity, &mut Boulder, &mut Sprite, &mut Transform)>,
        suns: Query<&Sun>,
        mut distance_questions: Query<&mut DistanceSquaredBetweenEdgesQuestion>,
        mut commands: Commands,
    ) {
        let mut rng = thread_rng();

        boulders
            .iter_mut()
            .for_each(|(entity, mut boulder, mut sprite, mut transform)| {
                match boulder.action {
                    BoulderAction::None => {
                        match boulder.energy / boulder.energy_capacity {
                            0.0..0.7 => {
                                boulder.action = BoulderAction::Move(
                                    commands
                                        .spawn(boulder.generate_move_question(
                                            transform.translation.xy(),
                                            sprite.custom_size.unwrap().x / 2.,
                                        ))
                                        .id(),
                                );
                            }
                            0.7..0.9 => {
                                todo!();
                            }
                            0.9..1.0 => {
                                todo!();
                            }
                            _ => {
                                unreachable!("We should have matched from 0% to 100%");
                            }
                        }
                        // if boulder.energy / boulder.energy_capacity >= 0.7 {
                        //     if boulder.energy_capacity > 10. {
                        //         if rng.gen_bool(0.5) {
                        //             todo!("Get bigger.");
                        //         } else {
                        //             todo!("Make more.");
                        //         }
                        //     } else {
                        //         todo!("Get bigger.");
                        //         commands.spawn(CollisionQuestion::new(todo!(), todo!()));
                        //     }
                        // } else {}
                    }

                    BoulderAction::Grow(question) => {}

                    BoulderAction::Move(question) => {
                        let mut question = distance_questions.get_mut(question).unwrap();
                        let Some(answer) = question.answer() else {
                            return;
                        };

                        boulder.action = BoulderAction::None;

                        let mut not_to_close = true;
                        let mut not_to_far = false;
                        for (colliding_entity, distance_squared) in answer.iter() {
                            if *colliding_entity == entity {
                                continue;
                            }

                            if *distance_squared < -squared(5.) {
                                //info!("Too close!");
                                //info!(distance_squared);
                                not_to_close = false;
                                break;
                            }

                            //info!("Does this happen?");

                            if *distance_squared < squared(5.) {
                                not_to_far = true;
                            }
                        }

                        if not_to_close && not_to_far {
                            //info!("Success!");
                            transform.translation.x = question.translation.x;
                            transform.translation.y = question.translation.y;
                        } else {
                            //info!("{}", question.translation - transform.translation.xy());
                        }
                    }

                    BoulderAction::Seed(question) => {}
                }
            });
    }

    fn generate_move_question(
        &self,
        translation: Vec2,
        radius: f32,
    ) -> DistanceSquaredBetweenEdgesQuestion {
        let mut rng = thread_rng();

        DistanceSquaredBetweenEdgesQuestion::new(
            translation
                + Vec2::new(
                    rng.gen_range(-radius..radius),
                    rng.gen_range(-radius..radius),
                ),
            radius,
        )
    }
}
