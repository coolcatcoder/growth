use crate::prelude::*;

fn ui_background() -> Color {
    Srgba::rgb(0.5, 0.2, 0.5).into()
}

#[system(Update::LoadMenus)]
fn game(mut ui: Ui) {
    load!(ui, Menu::InGame);
}

#[system(Update)]
fn setup(mut menu: MenuReader, mut load: Load, mut commands: Commands) {
    assert_return!(menu.switched_to(Menu::InGame));

    info!("Loading debug game.");

    commands.spawn((
        Transform::from_translation(Vec3::ZERO),
        Plant::new(vec![
            Stage {
                minimum_seconds: 3.,
                schematic: Schematic {
                    instructions: vec![
                        Instruction {
                            index: 0,
                            length: 1,
                        },
                        Instruction {
                            index: 1,
                            length: 2,
                        },
                    ],
                    lines: vec![
                        Line::new(Vec2::ZERO, Vec2::new(0., 100.), Vec2::ZERO),
                        Line::new(
                            Vec2::new(0., 100.),
                            Vec2::new(-50., 75.),
                            Vec2::new(0., 100.),
                        ),
                        Line::new(
                            Vec2::new(0., 100.),
                            Vec2::new(50., 75.),
                            Vec2::new(0., 100.),
                        ),
                    ],
                },
            },
            Stage {
                minimum_seconds: 20.,
                schematic: Schematic {
                    instructions: vec![
                        Instruction {
                            index: 0,
                            length: 1,
                        },
                        Instruction {
                            index: 1,
                            length: 2,
                        },
                    ],
                    lines: vec![
                        Line::new(Vec2::ZERO, Vec2::new(0., 200.), Vec2::ZERO),
                        Line::new(
                            Vec2::new(0., 100.),
                            Vec2::new(-25., 100.),
                            Vec2::new(0., 100.),
                        ),
                        Line::new(
                            Vec2::new(0., 100.),
                            Vec2::new(25., 100.),
                            Vec2::new(0., 100.),
                        ),
                    ],
                },
            },
            Stage {
                minimum_seconds: 60.,
                schematic: Schematic {
                    instructions: vec![
                        Instruction {
                            index: 0,
                            length: 1,
                        },
                        Instruction {
                            index: 1,
                            length: 2,
                        },
                    ],
                    lines: vec![
                        Line::new(Vec2::ZERO, Vec2::new(0., 100.), Vec2::ZERO),
                        Line::new(
                            Vec2::new(0., 100.),
                            Vec2::new(-50., 75.),
                            Vec2::new(0., 100.),
                        ),
                        Line::new(
                            Vec2::new(0., 100.),
                            Vec2::new(50., 75.),
                            Vec2::new(0., 100.),
                        ),
                    ],
                },
            },
        ]),
        PlantTester { lines: vec![] },
    ));

    // For testing purposes.
    return;

    load.path("./map");
}

#[system(Update)]
fn debug_plants(
    mut plants: Query<(&Transform, &mut Plant, &mut PlantTester)>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    const SPEED: f32 = 10.;
    let time_delta_seconds = time.delta_secs();

    plants
        .iter_mut()
        .for_each(|(transform, mut plant, mut plant_tester)| {
            let stage = some!(plant.stage_get_and_update(time_delta_seconds));

            plant_tester
                .lines
                .iter()
                .for_each(|(translation_1, translation_2)| {
                    gizmos.line_2d(
                        *translation_1 + transform.translation.xy(),
                        *translation_2 + transform.translation.xy(),
                        Srgba::RED,
                    );
                });

            // Using .any so we can only continue to the next instructions if the net distance incorrect is low enough.
            stage.schematic.instructions.iter().any(|instruction| {
                // the summed distance of every point away from where it should be.
                let mut net_distance_away = 0.;

                // This pseudo for loop allows us to deal with missing lines.
                let mut index = instruction.index;
                while index < instruction.index + instruction.length {
                    let desired_line = &stage.schematic.lines[index];
                    let line = match plant_tester.lines.get_mut(index) {
                        Some(line) => line,
                        None => {
                            // Adds the starting line, as desired.
                            plant_tester.lines.push((
                                desired_line.translation_1_start,
                                desired_line.translation_2_start,
                            ));
                            // Won't panic, as we just added a line.
                            plant_tester.lines.last_mut().unwrap()
                        }
                    };

                    let (desired_translation_1, desired_translation_2) =
                        (desired_line.translation_1, desired_line.translation_2);
                    let (translation_1, translation_2) = (&mut line.0, &mut line.1);

                    // This distance can also be used to get us the direction.
                    let distance_1 = desired_translation_1.distance(*translation_1);
                    let distance_2 = desired_translation_2.distance(*translation_2);

                    net_distance_away += distance_1 + distance_2;

                    // The normalised direction that we want to move in.
                    let mut direction_1 = (desired_translation_1 - *translation_1) / distance_1;
                    if direction_1.x.is_nan() {
                        direction_1.x = 0.;
                    }
                    if direction_1.y.is_nan() {
                        direction_1.y = 0.;
                    }

                    let mut direction_2 = (desired_translation_2 - *translation_2) / distance_2;
                    if direction_2.x.is_nan() {
                        direction_2.x = 0.;
                    }
                    if direction_2.y.is_nan() {
                        direction_2.y = 0.;
                    }

                    // Move the translations in the direction. Accounting for time's delta.
                    *translation_1 += direction_1 * time_delta_seconds * SPEED;
                    *translation_2 += direction_2 * time_delta_seconds * SPEED;

                    index += 1;
                }

                if net_distance_away < 1. {
                    // Close enough to continue.
                    false
                } else {
                    // Too far away. Stop.
                    true
                }
            });
        });
}

#[derive(Component, Default)]
struct PlantTester {
    lines: Vec<(Vec2, Vec2)>,
}

#[derive(Component)]
struct Plant {
    // How long has the plant lived for.
    seconds_passed: f32,
    // Consider having entities instead, for reuse? Perhaps Arc?
    stages: Vec<Stage>,
}

impl Plant {
    fn new(stages: Vec<Stage>) -> Self {
        Self {
            seconds_passed: 0.,
            stages,
        }
    }

    /// What stage is the plant currently?
    fn stage_get_and_update(&mut self, time_delta_seconds: f32) -> Option<&Stage> {
        self.seconds_passed += time_delta_seconds;

        let mut stage = None;

        self.stages.iter().any(|potential_stage| {
            if self.seconds_passed > potential_stage.minimum_seconds {
                stage = Some(potential_stage);
                false
            } else {
                true
            }
        });

        stage
    }
}

/// A stage of a plant. A plant can be transitioned from 1 stage to another smoothly. I hope
struct Stage {
    /// Only start this stage if at least this many seconds have passed from the start of the plant's life.
    minimum_seconds: f32,
    schematic: Schematic,
}

struct Schematic {
    instructions: Vec<Instruction>,
    /// Positions are relative.
    lines: Vec<Line>,
}

/// Grows plants perhaps?
/// Try it yourself.
/// This will grow a seedling:
///line((0, 0), (0, 0.5)),
///split(
///    line((0, 0.5), (0.25, 0.25)),
///    line((0, 0.5), (0.75, 0.25)),
///),
pub struct Instruction {
    index: usize,
    // Never zero?
    length: usize,
}

pub struct Line {
    translation_1: Vec2,
    translation_1_start: Vec2,

    translation_2: Vec2,
    translation_2_start: Vec2,
}

impl Line {
    fn new(translation_1: Vec2, translation_2: Vec2, start: Vec2) -> Self {
        Self {
            translation_1,
            translation_1_start: start,

            translation_2,
            translation_2_start: start,
        }
    }
}
