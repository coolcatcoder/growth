#![allow(clippy::type_complexity)]
#![warn(clippy::pedantic)]
// Not crimes.
#![allow(clippy::wildcard_imports)]
#![allow(clippy::needless_pass_by_value)]
// Crimes that are hard to fix.
// Sometimes crimes.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(incomplete_features)]
// Unstable features:
#![feature(generic_const_exprs)]

use std::ops::Range;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::system::EntityCommands,
};

mod collision;
mod events;
mod ground;
pub mod particle;
mod plant;
mod player;
mod sun;
mod time;
mod tree;

mod prelude {
    pub use super::{squared, Action, Grower};
    pub use crate::{
        collision::prelude::*, ground::prelude::*, particle, player::prelude::*, sun::prelude::*,
        time::prelude::*, tree::prelude::*,
    };
    pub use bevy::{
        ecs::{
            query::{QueryData, WorldQuery},
            system::SystemParam,
        },
        prelude::*,
    };
    pub use derive_more::{Deref, DerefMut};
    pub use leafwing_input_manager::prelude::*;
    pub use rand::prelude::*;
    pub use rayon::prelude::*;
    pub use std::time::Duration;
}
use prelude::*;
use rand::distributions::uniform::{SampleRange, SampleUniform};

// The world is dying. Save it. The sun will eventually hit the world. Hope they realise that sooner rather than later!
// Energy is area, roughly 1 energy for 700 area (30 diameter circle). You can only store as much energy as your area will allow.

// Big main world full of plants is cool. You see it during the beginning. Then suddenly instead of all the little sun, you see a huge chunk hit everything.
// It all goes to yellow.
// Remake the world, from the few dying plants surviving on small bits of floating rubble.

// Player weird floating orb thing, with smaller orbs orbiting. after the crash you have few, and must slowly collect them, and gather abilities
// Before you get a power upgrade, you are forced to deal with a battery which lasts only a few seconds. Your screen quickly getting darker and darker.
// You recharge using the sun. Too much hurts.

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
            InputManagerPlugin::<Action>::default(),
            RunEveryPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                //debug_move_camera,
                player::debug_collisions,
                move_players,
                plant::Boulder::update,
                Ground::grower,
                Tree::grower,
                Leaf::grower,
                Sun::update,
                (
                    particle::Ticker::update_time,
                    particle::AmbientFriction::motion,
                    particle::AmbientFriction::velocity,
                    particle::StopOnCollision::motion,
                    particle::StopOnCollision::velocity,
                    particle::StepUp::motion,
                    particle::Motion::system,
                    particle::Velocity::system,
                    particle::Ticker::finish,
                )
                    .chain_ignore_deferred(),
            ),
        )
        .add_systems(
            PostUpdate,
            camera_follow.before(TransformSystem::TransformPropagate),
        )
        .add_systems_that_run_every(
            Duration::from_secs_f64(1. / 30.),
            (
                orb_follow,
                plant::Boulder::absorb_sun,
                (ColliderGrid::update, collide).chain(),
            ),
        )
        // Maybe not...
        //.add_systems_that_run_every(Duration::from_secs_f64(1. / 5.), sync_player_transforms)
        //.add_systems_that_run_every(Duration::from_secs_f32(1.), || info!("blah"))
        .init_resource::<ActionState<Action>>()
        .insert_resource(Action::default_input_map())
        .insert_resource(ColliderGrid::new(GRID_ORIGIN))
        .run();
}

// This does not contain the translation, as that having a default does not make sense.
// This is only for properties that usually stay the same from one nodule to another.
#[derive(Clone)]
struct NoduleConfig {
    depth: f32,
    diameter: f32,
    colour: [f32; 3],
    collision: bool,
}

impl Default for NoduleConfig {
    fn default() -> Self {
        Self {
            depth: 0.,
            diameter: 30.,
            colour: [0.5, 0.5, 0.5],
            collision: true,
        }
    }
}

struct LineConfig {
    spacing: Vec2,

    // Offset y translates y and creeps randomly up and down.
    offset_y_bounds: Range<f32>,
    offset_y_change: Range<f32>,

    // Randomly translates y, to make the terrain look rough or smooth.
    roughness: Range<f32>,

    // The jitter of every nodule
    jitter_x: Range<f32>,
    jitter_y: Range<f32>,

    height: u32,
    depth: u32,
    // Idea: Easing
    // Slowly pulls in the clamp (perhaps via lerp?) so that the nodules finish exactly (or inexactly) at the end point!
}

impl Default for LineConfig {
    fn default() -> Self {
        Self {
            spacing: Vec2::splat(20.),

            offset_y_bounds: (30. * -5.)..(30. * 5.),
            offset_y_change: -20.0..20.0,

            roughness: 0.0..0.0,

            jitter_x: -5.0..5.0,
            jitter_y: -5.0..5.0,

            height: 0,
            depth: 0,
        }
    }
}

#[derive(Copy, Clone)]
struct LineEnd {
    translation: Vec2,
    offset_y: f32,
}

struct Terrain<'c, 'a, 'w, 's> {
    rng: ThreadRng,

    commands: &'c mut Commands<'w, 's>,
    asset_server: &'a AssetServer,
}

impl<'c, 'a, 'w, 's> Terrain<'c, 'a, 'w, 's> {
    const DEBUG: bool = false;

    fn create(&mut self, config: NoduleConfig, translation: Vec2) -> EntityCommands {
        let mut entity_commands = self.commands.spawn(SpriteBundle {
            texture: self.asset_server.load("nodule.png"),
            transform: Transform::from_translation(Vec3::new(
                translation.x,
                translation.y,
                config.depth,
            )),
            sprite: Sprite {
                color: Color::Srgba(Srgba::rgb(
                    config.colour[0],
                    config.colour[1],
                    config.colour[2],
                )),
                custom_size: Some(Vec2::splat(config.diameter)),
                ..default()
            },
            ..default()
        });

        if config.collision {
            entity_commands.insert(Collider {
                radius: config.diameter / 2.,
            });
        }

        entity_commands
    }

    fn new(from: Vec2, commands: &'c mut Commands<'w, 's>, asset_server: &'a AssetServer) -> Self {
        let mut terrain = Self {
            rng: thread_rng(),

            commands,
            asset_server,
        };

        if Self::DEBUG {
            terrain.create(
                NoduleConfig {
                    colour: [1., 0., 0.],
                    diameter: 40.,
                    ..default()
                },
                from,
            );
        }

        terrain
    }

    // Consider switching from nodule config to a closure that returns a nodule config with parameters for x and y and such like.
    fn line(
        &mut self,
        from: LineEnd,
        to: Vec2,
        config: LineConfig,
        nodule: NoduleConfig,
    ) -> LineEnd {
        let mut to = LineEnd {
            translation: to,
            offset_y: from.offset_y,
        };

        //let distance = self.previous_translation.distance(to);
        let distance_x = (from.translation.x - to.translation.x).abs();
        let gradient =
            (to.translation.y - from.translation.y) / (to.translation.x - from.translation.x);

        if Self::DEBUG {
            self.create(
                NoduleConfig {
                    colour: [1., 0., 0.],
                    diameter: 40.,
                    ..default()
                },
                to.translation,
            );
        }

        // Hang on a second, why is this using distance? Shouldn't it only be x distance, not real distance???
        // I've replaced it with distance_x, but we should make certain that this is now correct.
        for x in 0..((distance_x / config.spacing.x).round() as u32) {
            let x = x as f32 * config.spacing.x;
            let y = x * gradient;

            if Self::DEBUG {
                self.create(
                    NoduleConfig {
                        colour: [0., 0., 1.],
                        ..default()
                    },
                    Vec2::new(x, y) + from.translation,
                );
                self.create(
                    NoduleConfig {
                        colour: [0., 1., 0.],
                        ..default()
                    },
                    Vec2::new(x, y + config.offset_y_bounds.start) + from.translation,
                );
                self.create(
                    NoduleConfig {
                        colour: [0., 1., 0.],
                        ..default()
                    },
                    Vec2::new(x, y + config.offset_y_bounds.end) + from.translation,
                );
            }

            to.offset_y += self
                .rng
                .gen_range_allow_empty(config.offset_y_change.clone());
            to.offset_y = to
                .offset_y
                .clamp(config.offset_y_bounds.start, config.offset_y_bounds.end);

            let roughness = self.rng.gen_range_allow_empty(config.roughness.clone());

            let mut spawner = |translation: f32| {
                let jitter = Vec2::new(
                    self.rng.gen_range_allow_empty(config.jitter_x.clone()),
                    self.rng.gen_range_allow_empty(config.jitter_y.clone()),
                );

                self.create(
                    nodule.clone(),
                    Vec2::new(x, y + to.offset_y + translation + roughness)
                        + from.translation
                        + jitter,
                );
            };

            for height in 0..config.height {
                spawner((height + 1) as f32 * config.spacing.y);
            }

            spawner(0.0);

            for depth in 0..config.depth {
                spawner((depth + 1) as f32 * -config.spacing.y);
            }
        }

        to
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = thread_rng();

    let mut terrain = Terrain::new(Vec2::new(-25_000., 10000.0), &mut commands, &asset_server);

    //MARK: Mountain
    let mountain = terrain.line(
        LineEnd {
            translation: Vec2::new(-25_000., 10000.),
            offset_y: 0.,
        },
        Vec2::new(-20_000., 0.),
        LineConfig {
            depth: 100,

            roughness: -500.0..500.0,
            ..default()
        },
        NoduleConfig { ..default() },
    );

    //MARK: Forest?
    let forest = terrain.line(
        mountain,
        Vec2::new(-5_000., 0.),
        LineConfig {
            depth: 50,
            ..default()
        },
        NoduleConfig { ..default() },
    );

    //MARK: Lake
    //water
    let water_y = 30. * -6.;
    terrain.line(
        LineEnd {
            translation: Vec2::new(forest.translation.x, water_y),
            offset_y: 0.,
        },
        Vec2::new(500., water_y),
        LineConfig {
            offset_y_change: 0.0..0.0,
            ..default()
        },
        NoduleConfig {
            depth: -1.,
            colour: [0., 0., 1.],
            ..default()
        },
    );
    // Terrain::new(Vec2::new(-5_000., 0.), &mut commands, &asset_server).line(
    //     Vec2::new(-500., -1000.),
    //     LineConfig {
    //         depth: 50,
    //         ..default()
    //     },
    //     NoduleConfig { ..default() },
    // );

    let lake = terrain.line(
        forest,
        Vec2::new(-500., -1500.),
        LineConfig {
            depth: 50,
            ..default()
        },
        NoduleConfig { ..default() },
    );

    //MARK: Pancake Falls
    let start_x = -500.;
    let start_y = -1500.;
    let increment_x = 500.;
    let increment_y = 1500.;
    let rise = 300.;
    let thickness = 30;

    for y in 0..5 {
        for x in 0..4 {
            let y_to_from = match x {
                0 => (0., rise),
                1 => (rise, 0.),
                2 => (0., rise),
                3 => (rise, 0.),
                _ => unreachable!(),
            };

            terrain.line(
                LineEnd {
                    translation: Vec2::new(
                        start_x + x as f32 * increment_x + y as f32 * increment_x,
                        start_y + y as f32 * increment_y + y_to_from.0,
                    ),
                    offset_y: 0.,
                },
                Vec2::new(
                    start_x + (x + 1) as f32 * increment_x + y as f32 * increment_x,
                    start_y + y as f32 * increment_y + y_to_from.1,
                ),
                LineConfig {
                    spacing: Vec2::splat(30.),
                    depth: thickness,
                    offset_y_change: 0.0..0.0,
                    jitter_x: 0.0..0.0,
                    jitter_y: 0.0..0.0,
                    ..default()
                },
                NoduleConfig {
                    colour: [0.5, 0.5, 0.8],
                    ..default()
                },
            );
        }
    }

    let mut player_translation = Vec2::ZERO;

    info!("player {}", player_translation);

    let player = commands
        .spawn((
            Player,
            SpriteBundle {
                texture: asset_server.load("nodule.png"),
                sprite: Sprite {
                    color: Color::Srgba(Srgba::rgb(1.0, 0.0, 0.0)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    player_translation.x,
                    player_translation.y + 90.,
                    1.,
                )),
                ..default()
            },
            particle::Ticker(EveryTime::new(Duration::from_secs_f64(1. / 25.), default())),
            particle::Velocity(Vec2::new(0., -5.)),
            particle::StopOnCollision,
            Collider { radius: 15. },
            particle::StepUp(60.),
            particle::AmbientFriction(Vec2::splat(0.02)),
        ))
        .id();

    commands.spawn(Camera2dBundle { ..default() });

    for i in 1..=10 {
        commands.spawn((
            AbilityOrb {
                following: Some(player),
                distance: 60. * i as f32,
            },
            SpriteBundle {
                texture: asset_server.load("nodule.png"),
                sprite: Sprite {
                    color: Color::Srgba(Srgba::rgb(1.0, 1.0, 0.0)),
                    custom_size: Some(Vec2::splat(15.)),
                    ..default()
                },
                transform: Transform::from_translation(Vec3::new(
                    player_translation.x,
                    player_translation.y + 90. + (60. * i as f32),
                    1.,
                )),
                ..default()
            },
        ));
    }
}

pub trait Grower: Component + Sized {
    type SystemParameters<'w, 's>: SystemParam;
    type Components<'a>: QueryData;

    fn tick(
        &mut self,
        system_parameters: &mut Self::SystemParameters<'_, '_>,
        components: <<Self as Grower>::Components<'_> as WorldQuery>::Item<'_>,
    );

    fn grower(
        mut growers: Query<(&mut Self, Self::Components<'_>)>,
        mut system_parameters: Self::SystemParameters<'_, '_>,
    ) {
        for (mut grower, components) in &mut growers {
            grower.tick(&mut system_parameters, components);
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy, Debug, Reflect)]
pub enum Action {
    Move,
    Zoom,
}

impl Actionlike for Action {
    // Record what kind of inputs make sense for each action.
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            Self::Move => InputControlKind::DualAxis,
            Self::Zoom => InputControlKind::Axis,
            //_ => InputControlKind::Button,
        }
    }
}

impl Action {
    fn default_input_map() -> InputMap<Self> {
        InputMap::default()
            .with_dual_axis(Self::Move, KeyboardVirtualDPad::WASD)
            .with_axis(Self::Zoom, MouseScrollAxis::Y)
    }
}

fn debug_move_camera(
    time: Res<Time>,
    actions: Res<ActionState<Action>>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
) {
    const MOVE_SPEED: f32 = 600.;
    const ZOOM_SPEED: f32 = 10.;

    let (mut transform, mut camera) = camera.single_mut();

    let movement = actions.clamped_axis_pair(&Action::Move).xy()
        * MOVE_SPEED
        * time.delta_seconds()
        * camera.scale;

    transform.translation.x += movement.x;
    transform.translation.y += movement.y;

    camera.scale +=
        actions.axis_data(&Action::Zoom).unwrap().value * ZOOM_SPEED * time.delta_seconds();
}

pub fn squared(value: f32) -> f32 {
    value * value
}

trait RangeStartEnd<T> {
    fn start(&self) -> T;
    fn end(&self) -> T;
}

impl<T: Copy> RangeStartEnd<T> for Range<T> {
    fn start(&self) -> T {
        self.start
    }

    fn end(&self) -> T {
        self.end
    }
}

trait RngExtension {
    fn gen_range_allow_empty<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform + PartialEq,
        R: SampleRange<T> + RangeStartEnd<T>;
}

impl RngExtension for ThreadRng {
    fn gen_range_allow_empty<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform + PartialEq,
        R: SampleRange<T> + RangeStartEnd<T>,
    {
        if range.start() == range.end() {
            range.start()
        } else {
            range.sample_single(self)
        }
    }
}
