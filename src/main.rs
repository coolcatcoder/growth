#![allow(clippy::type_complexity)]
#![warn(clippy::pedantic)]
//#![deny(missing_docs)]
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
#![feature(maybe_uninit_array_assume_init)]

use std::ops::{Range, RangeInclusive};

use arrayvec::ArrayVec;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::query::QueryFilter,
    window::PrimaryWindow,
};
use bevy_egui::{
    egui::{self, color_picker, DragValue, Response, Ui},
    EguiContexts, EguiPlugin,
};

mod collision;
mod events;
mod ground;
pub mod particle;
mod plant;
mod player;
mod saving;
mod sun;
mod time;
mod tree;

mod prelude {
    pub use super::{squared, Action, GizmosLingering, Grower, NoduleConfig, Terrain};
    pub use crate::{
        collision::prelude::*, ground::prelude::*, particle, player::prelude::*,
        saving::prelude::*, sun::prelude::*, time::prelude::*, tree::prelude::*,
    };
    pub use bevy::{
        ecs::{
            query::{QueryData, WorldQuery},
            system::{EntityCommands, SystemParam},
        },
        prelude::*,
        utils::Parallel,
    };
    pub use derive_more::{Deref, DerefMut};
    pub use leafwing_input_manager::prelude::*;
    pub use rand::prelude::*;
    pub use rayon::prelude::*;
    pub use std::time::Duration;
}
use bevy_common_assets::json::JsonAssetPlugin;
use prelude::*;
use rand::distributions::uniform::{SampleRange, SampleUniform};
use serde::{Deserialize, Serialize};

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
            //FrameTimeDiagnosticsPlugin,
            //LogDiagnosticsPlugin::default(),
            InputManagerPlugin::<Action>::default(),
            RunEveryPlugin,
            JsonAssetPlugin::<Map>::new(&[".json"]),
            EguiPlugin,
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                LineSelected::ui,
                LoadMap::load,
                (
                    TerrainLine::generate,
                    TerrainLine::validate,
                    TerrainLine::debug,
                    TerrainPoint::select,
                    TerrainPoint::add,
                    TerrainPoint::remove,
                    TerrainPoint::debug,
                    TerrainPoint::translate,
                ),
                update_cursor_translation,
                particle::Chain::solve,
                plant::WibblyGrass::sway,
                particle::DistanceConstraint::solve,
                //player::debug_action,
                particle::Verlet::system,
                display_lingering_gizmos,
                debug_move_camera,
                //player::debug_collisions,
                //move_players,
                plant::Boulder::update_system,
                //Ground::grower,
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
        // .add_systems(
        //     PostUpdate,
        //     camera_follow.before(TransformSystem::TransformPropagate),
        // )
        .add_systems_that_run_every(Duration::from_secs_f64(1. / 5.), particle::Verlet::collide)
        .add_systems_that_run_every(Duration::from_secs_f64(1. / 5.), ColliderGrid::update)
        .add_systems_that_run_every(
            Duration::from_secs_f64(1. / 30.),
            // TODO: ColliderGrid::update is special, in the fact that an entity likely won't move more than a grid every frame,
            // so it doesn't have to update as often as we currently have it set.
            collide,
        )
        // Maybe not...
        //.add_systems_that_run_every(Duration::from_secs_f64(1. / 5.), sync_player_transforms)
        //.add_systems_that_run_every(Duration::from_secs_f32(1.), || info!("blah"))
        .init_resource::<ActionState<Action>>()
        .init_resource::<GizmosLingering>()
        .init_resource::<CursorWorldTranslation>()
        .init_resource::<CursorPreviousWorldTranslation>()
        .init_resource::<LoadMap>()
        .init_resource::<LineSelected>()
        .insert_resource(Action::default_input_map())
        .insert_resource(ColliderGrid::new(GRID_ORIGIN))
        .add_event::<Save>()
        .add_event::<Load>()
        .run();
}

// This does not contain the translation, as that having a default does not make sense.
// This is only for properties that usually stay the same from one nodule to another.
#[derive(Clone)]
pub struct NoduleConfig {
    pub depth: f32,
    pub diameter: f32,
    pub colour: [f32; 3],
    pub collision: bool,
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

// A stupid solution, but it works.
// TODO: Get rid of this eventually, as customisation becomes normal, and emptiness is weird.
type LineConfigDefault = LineConfig;

struct LineConfig<const L: usize = 0> {
    spacing: Vec2,

    // Offset y translates y and creeps randomly up and down.
    offset_y_bounds: Range<f32>,
    offset_y_change: Range<f32>,

    // Randomly translates y, to make the terrain look rough or smooth.
    roughness: Range<f32>,

    // The jitter of every nodule
    jitter_x: Range<f32>,
    jitter_y: Range<f32>,

    // How many nodules we shall spawn in a downwards direction.
    // 1 means there will be 1 nodule.
    depth: u32,
    // Offsets all nodules this amount upwards.
    upwards_offset: f32,

    // Idea: Easing
    // Slowly pulls in the clamp (perhaps via lerp?) so that the nodules finish exactly (or inexactly) at the end point!

    // Runs functions for every nodule spawned until they return true.
    customisers: [Option<fn(LineCustomiserInfo) -> bool>; L],
}

impl<const L: usize> Default for LineConfig<L> {
    fn default() -> Self {
        Self {
            spacing: Vec2::splat(20.),

            offset_y_bounds: (30. * -5.)..(30. * 5.),
            offset_y_change: -20.0..20.0,

            roughness: 0.0..0.0,

            jitter_x: -5.0..5.0,
            jitter_y: -5.0..5.0,

            depth: 1,
            upwards_offset: 0.,

            customisers: [None; L],
        }
    }
}

struct LineCustomiserInfo<'t, 'c, 'a, 'w, 's> {
    terrain: &'t mut Terrain<'c, 'a, 'w, 's>,

    // The mathematical point on the line we are currently at.
    // If you want this to be in world coordinates, make sure to add from to it.
    point_translation: Vec2,
    // How many nodules across are we, and how many nodules away from the top are we?
    nodule_translation: UVec2,
    // the actual current translation in world coordinates.
    translation: Vec2,
}

#[derive(Copy, Clone)]
struct LineEnd {
    translation: Vec2,
    offset_y: f32,
}

pub struct Terrain<'c, 'a, 'w, 's> {
    pub rng: ThreadRng,

    pub commands: &'c mut Commands<'w, 's>,
    pub asset_server: &'a AssetServer,
}

impl<'c, 'a, 'w, 's> Terrain<'c, 'a, 'w, 's> {
    const DEBUG: bool = false;

    pub fn create(&mut self, config: NoduleConfig, translation: Vec2) -> EntityCommands {
        let mut entity_commands = self.commands.spawn((
            Transform::from_translation(Vec3::new(translation.x, translation.y, config.depth)),
            Sprite {
                image: self.asset_server.load("nodule.png"),
                color: Color::srgb(config.colour[0], config.colour[1], config.colour[2]),
                custom_size: Some(Vec2::splat(config.diameter)),
                ..default()
            },
        ));

        if config.collision {
            entity_commands.insert(Radius {
                0: config.diameter / 2.,
            });
        }

        entity_commands
    }

    fn new(from: Vec2, commands: &'c mut Commands<'w, 's>, asset_server: &'a AssetServer) -> Self {
        let mut terrain = Self {
            rng: thread_rng(),
            //rng: StdRng::from_rng(thread_rng()).unwrap(),
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

        // 1970474327465874943
        // let seed = terrain.rng.next_u64();
        // info!("{seed}");

        terrain
    }

    // Consider switching from nodule config to a closure that returns a nodule config with parameters for x and y and such like.
    fn line<const L: usize>(
        &mut self,
        from: LineEnd,
        to: Vec2,
        mut config: LineConfig<L>,
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
        let end = (distance_x / config.spacing.x).round() as u32;

        for nodule_x in 0..end {
            let x = nodule_x as f32 * config.spacing.x;
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

            for depth in 0..config.depth {
                let jitter = Vec2::new(
                    self.rng.gen_range_allow_empty(config.jitter_x.clone()),
                    self.rng.gen_range_allow_empty(config.jitter_y.clone()),
                );

                let translation = Vec2::new(
                    x,
                    y + to.offset_y + (depth as f32 * -config.spacing.y) + roughness,
                ) + from.translation
                    + jitter;

                config.customisers.iter_mut().for_each(|customiser| {
                    if let Some(inner_customiser) = customiser.as_mut() {
                        if inner_customiser(LineCustomiserInfo {
                            terrain: self,

                            point_translation: Vec2::new(x, y),
                            nodule_translation: UVec2::new(nodule_x, depth),
                            translation,
                        }) {
                            *customiser = None;
                        }
                    }
                });

                self.create(nodule.clone(), translation);
            }
        }

        to
    }
}

#[derive(Default)]
struct LinePlacer(Vec<(f32, fn(LineCustomiserInfo))>);

impl LinePlacer {
    fn add(&mut self, x: f32, customiser: fn(LineCustomiserInfo)) {
        self.0.push((x, customiser));
    }

    fn tick(&mut self, info: LineCustomiserInfo) {
        for index in (0..self.0.len()).rev() {
            let (x, customiser) = &mut self.0[index];

            if info.nodule_translation.y == 0 && (info.translation.x - *x).abs() < 60. {
                customiser(LineCustomiserInfo {
                    terrain: info.terrain,
                    point_translation: info.point_translation,
                    nodule_translation: info.nodule_translation,
                    translation: info.translation,
                });
                self.0.swap_remove(index);
            }
        }
    }
}

// Wraps everything with an option.
// Hopefully safe.
fn customisers<const L: usize>(
    customisers: [fn(LineCustomiserInfo) -> bool; L],
) -> [Option<fn(LineCustomiserInfo) -> bool>; L] {
    let mut output =
        [const { std::mem::MaybeUninit::<Option<fn(LineCustomiserInfo) -> bool>>::uninit() }; L];
    customisers
        .into_iter()
        .enumerate()
        .for_each(|(index, customiser)| {
            output[index].write(Some(customiser));
        });
    // SAFETY: output and customisers are the same length.
    unsafe { std::mem::MaybeUninit::array_assume_init(output) }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut map: ResMut<LoadMap>) {
    map.0 = Some(asset_server.load::<Map>("map.json"));

    let mut rng: ThreadRng = thread_rng();

    let mut terrain = Terrain::new(Vec2::new(-25_000., 10000.0), &mut commands, &asset_server);

    //MARK: Mountain
    let mountain = terrain.line(
        LineEnd {
            translation: Vec2::new(-25_000., 10000.),
            offset_y: 0.,
        },
        Vec2::new(-20_000., 0.),
        LineConfigDefault {
            depth: 100,

            roughness: -500.0..500.0,
            ..default()
        },
        NoduleConfig { ..default() },
    );

    //MARK: Forest?
    // TODO: Add caves and other points of interest. Also plants!!!
    // let mut placer = LinePlacer::default();
    // placer.add(-5_500., plant::WibblyGrass::create_experimental::<50>);

    let forest = terrain.line(
        mountain,
        Vec2::new(-5_000., 0.),
        LineConfig {
            depth: 50,
            customisers: customisers([
                plant::WibblyGrass::create::<-5_500, 50>,
                plant::WibblyGrass::create::<-5_600, 50>,
            ]),
            ..default()
        },
        NoduleConfig { ..default() },
    );

    // for translation in forest_surface {
    //     if rng.gen_bool(0.1) {
    //         plant::Boulder::create(
    //             translation + Vec2::new(0., 40.),
    //             terrain.commands,
    //             &asset_server,
    //         );
    //     }
    // }

    //MARK: Lake
    //water
    let water_y = 30. * -6.;
    terrain.line(
        LineEnd {
            translation: Vec2::new(forest.translation.x, water_y),
            offset_y: 0.,
        },
        Vec2::new(500., water_y),
        LineConfigDefault {
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

    terrain.line(
        forest,
        Vec2::new(-500., -1500.),
        LineConfigDefault {
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
                LineConfigDefault {
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
            Transform::from_translation(Vec3::new(
                player_translation.x,
                player_translation.y + 90.,
                1.,
            )),
            Sprite {
                image: asset_server.load("nodule.png"),
                color: Color::Srgba(Srgba::rgb(1.0, 0.0, 0.0)),
                ..default()
            },
            particle::Ticker(EveryTime::new(Duration::from_secs_f64(1. / 25.), default())),
            particle::Velocity(Vec2::new(0., -5.)),
            particle::StopOnCollision,
            Radius { 0: 15. },
            particle::StepUp(60.),
            particle::AmbientFriction(Vec2::splat(0.02)),
        ))
        .id();

    commands.spawn(Camera2d);

    let mut target = player;
    for i in 1..=50 {
        target = commands
            .spawn((
                particle::DistanceConstraint {
                    distance: 5.,
                    target,
                },
                AbilityOrb {
                    following: Some(player),
                    distance: 60. * i as f32,
                },
                Sprite {
                    image: asset_server.load("nodule.png"),
                    color: Color::Srgba(Srgba::rgb(1.0, 1.0, 0.0)),
                    custom_size: Some(Vec2::splat(15.)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(
                    player_translation.x,
                    player_translation.y + 90. + (60. * i as f32),
                    1.,
                )),
            ))
            .id();
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
    Debug,

    AddPoint,
    RemovePoint,
    CreateOrSelectLineMode,
    Select,
    TranslatePoint,
}

impl Actionlike for Action {
    // Record what kind of inputs make sense for each action.
    fn input_control_kind(&self) -> InputControlKind {
        match self {
            Self::Move => InputControlKind::DualAxis,
            Self::Zoom => InputControlKind::Axis,
            _ => InputControlKind::Button,
        }
    }
}

impl Action {
    fn default_input_map() -> InputMap<Self> {
        InputMap::default()
            .with_dual_axis(Self::Move, VirtualDPad::wasd())
            .with_axis(Self::Zoom, MouseScrollAxis::Y)
            .with(Self::Debug, KeyCode::KeyF)
            .with(
                Self::AddPoint,
                ButtonlikeChord::from_single(MouseButton::Left).with(KeyCode::KeyQ),
            )
            .with(
                Self::RemovePoint,
                ButtonlikeChord::from_single(MouseButton::Left).with(KeyCode::KeyE),
            )
            .with(Self::CreateOrSelectLineMode, KeyCode::KeyR)
            .with(Self::Select, MouseButton::Left)
            .with(Self::TranslatePoint, MouseButton::Right)
    }
}

fn debug_move_camera(
    time: Res<Time>,
    actions: Res<ActionState<Action>>,
    mut camera: Query<(&mut Transform, &mut OrthographicProjection), With<Camera2d>>,
    mut player: Query<&mut Transform, (With<Player>, Without<Camera2d>)>,
) {
    const MOVE_SPEED: f32 = 600.;
    const ZOOM_SPEED: f32 = 10.;

    let (mut transform, mut camera) = camera.single_mut();

    let movement = actions.clamped_axis_pair(&Action::Move).xy()
        * MOVE_SPEED
        * time.delta_secs()
        * camera.scale;

    transform.translation.x += movement.x;
    transform.translation.y += movement.y;

    camera.scale +=
        actions.axis_data(&Action::Zoom).unwrap().value * ZOOM_SPEED * time.delta_secs();

    player.single_mut().translation.x = transform.translation.x;
    player.single_mut().translation.y = transform.translation.y;
}

#[derive(Resource, Default)]
pub struct CursorWorldTranslation(pub Vec2);

#[derive(Resource, Default)]
pub struct CursorPreviousWorldTranslation(pub Vec2);

pub fn update_cursor_translation(
    mut cursor_position: ResMut<CursorWorldTranslation>,
    mut previous_cursor_position: ResMut<CursorPreviousWorldTranslation>,
    window: Query<&Window, With<PrimaryWindow>>,
    camera: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = camera.get_single().unwrap();

    let window = window.get_single().unwrap();

    if let Some(world_position) = window
        .cursor_position()
        .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor).ok())
        .map(|ray| ray.origin.truncate())
    {
        previous_cursor_position.0 = cursor_position.0;
        cursor_position.0 = world_position;
        //println!("cursor coords: {},{}", world_position.x, world_position.y);
    }
}

/// A point that can be used in terrain lines.
#[derive(Component, Default)]
pub struct TerrainPoint {
    selected: bool,
}

impl TerrainPoint {
    const RADIUS: f32 = 50.;

    /// Adds a terrain point where the user clicked.
    fn add(
        actions: Res<ActionState<Action>>,
        translation: Res<CursorWorldTranslation>,
        mut commands: Commands,
    ) {
        if actions.just_pressed(&Action::AddPoint) {
            commands.spawn((
                TerrainPoint::default(),
                Transform::from_translation(Vec3::new(translation.0.x, translation.0.y, 0.)),
            ));
        }
    }

    /// Removes all terrain points the mouse is currently in, whenever the mouse button and key are held down.
    fn remove(
        points: Query<(Entity, &Transform), With<TerrainPoint>>,
        actions: Res<ActionState<Action>>,
        cursor_translation: Res<CursorWorldTranslation>,
        mut commands: Commands,
    ) {
        if actions.pressed(&Action::RemovePoint) {
            points.iter().for_each(|(entity, transform)| {
                if check_collision(
                    10.,
                    cursor_translation.0,
                    Self::RADIUS,
                    transform.translation.xy(),
                ) {
                    if let Some(mut entity) = commands.get_entity(entity) {
                        entity.despawn();
                    }
                }
            });
        }
    }

    /// Translates the points, based on the mouse's position.
    /// Also regenerates lines, so they are based on the new point position.
    fn translate(
        mut lines: Query<&mut TerrainLine>,
        mut points: Query<(Entity, &mut Transform), With<TerrainPoint>>,
        actions: Res<ActionState<Action>>,
        cursor_translation: Res<CursorWorldTranslation>,
        cursor_previous_translation: Res<CursorPreviousWorldTranslation>,
    ) {
        if actions.pressed(&Action::TranslatePoint) {
            points.iter_mut().for_each(|(entity, mut transform)| {
                if check_collision(
                    10.,
                    cursor_previous_translation.0,
                    Self::RADIUS,
                    transform.translation.xy(),
                ) {
                    transform.translation += Vec3::new(
                        cursor_translation.0.x - cursor_previous_translation.0.x,
                        cursor_translation.0.y - cursor_previous_translation.0.y,
                        0.,
                    );

                    lines.iter_mut().for_each(|mut line| {
                        if line.point_1 == entity || line.point_2 == entity {
                            line.generate = true;
                        }
                    });
                }
            });
        }
    }

    /// Selects points. While the key is held down:
    /// If you select 2 points, it will form a terrain line between them, if there is one already, it will select it.
    fn select(
        mut points: Query<(Entity, &Transform, &mut TerrainPoint)>,
        lines: Query<(Entity, &TerrainLine)>,
        actions: Res<ActionState<Action>>,
        cursor_translation: Res<CursorWorldTranslation>,
        mut line_selected: ResMut<LineSelected>,
        mut commands: Commands,
    ) {
        if actions.pressed(&Action::CreateOrSelectLineMode) {
            if !actions.just_pressed(&Action::Select) {
                return;
            }

            // Get all points selected. For any clicked on, and not selected, we select them.
            let mut points_selected = vec![];
            points
                .iter_mut()
                .for_each(|(entity, transform, mut point)| {
                    if point.selected
                        || check_collision(
                            10.,
                            cursor_translation.0,
                            Self::RADIUS,
                            transform.translation.xy(),
                        )
                    {
                        point.selected = true;
                        points_selected.push((entity, point));
                    }
                });

            if points_selected.len() == 2 {
                points_selected[0].1.selected = false;
                points_selected[1].1.selected = false;

                if let Some((line_entity, _)) = lines.iter().find(|(_, line)| {
                    // Both line points equal either of the selected points.
                    (line.point_1 == points_selected[0].0 || line.point_1 == points_selected[1].0)
                        && (line.point_2 == points_selected[0].0
                            || line.point_2 == points_selected[1].0)
                }) {
                    line_selected.0 = Some(line_entity);
                } else {
                    info!("Created a line!");
                    line_selected.0 = Some(
                        commands
                            .spawn(TerrainLine::new((
                                points_selected[0].0,
                                points_selected[1].0,
                            )))
                            .id(),
                    );
                }
            }
        } else {
            points.iter_mut().for_each(|(_, _, mut point)| {
                point.selected = false;
            });
        }
    }

    /// Highlights the terrain points in red, for easy debugging.
    fn debug(points: Query<(&TerrainPoint, &Transform)>, mut gizmos: Gizmos) {
        points.iter().for_each(|(point, transform)| {
            let colour = if point.selected {
                Color::srgb(0.5, 0., 0.)
            } else {
                Color::srgb(1., 0., 0.)
            };

            gizmos.circle_2d(transform.translation.xy(), Self::RADIUS, colour);
        });
    }
}

#[derive(Resource, Default)]
struct LoadMap(Option<Handle<Map>>);

impl LoadMap {
    // Loads the map!
    fn load(mut map: ResMut<Self>) {
        let Some(map) = map.0.take() else {
            return;
        };

        info!("Got it!");
    }
}

/// All lines and anything else on the map.
#[derive(Serialize, Deserialize, Asset, TypePath)]
struct Map {
    lines: Vec<SerializeableTerrainLine>,
}

impl Map {
    //Save
}

/// Anything generated by terrain.
#[derive(Component)]
struct BelongsToTerrain(Entity);

//MARK: TerrainLine
/// Allows only 1 line to be selected at a time.
#[derive(Resource, Default)]
struct LineSelected(Option<Entity>);

impl LineSelected {
    fn ui(
        mut contexts: EguiContexts,
        mut commands: Commands,
        mut line_selected: ResMut<LineSelected>,
        mut lines: Query<&mut TerrainLine>,
        generated: Query<(Entity, &BelongsToTerrain)>,
    ) {
        let Some(line_entity) = line_selected.0 else {
            return;
        };

        let Ok(mut line) = lines.get_mut(line_entity) else {
            return;
        };
        let line = line.into_inner();

        egui::SidePanel::left("Line Editor").show(contexts.ctx_mut(), |ui| {
            pub fn vec_ui<const N: usize, T: bevy_egui::egui::emath::Numeric>(
                ui: &mut Ui,
                vec_and_component_names: [(&mut T, &str); N],
                name: &str,
                range: RangeInclusive<T>,
                speed: f32,
            ) -> [Response; N] {
                let mut responses = ArrayVec::<_, N>::new();

                ui.vertical(|ui| {
                    ui.label(name);

                    ui.horizontal(|ui| {
                        for (component, name) in vec_and_component_names {
                            ui.vertical(|ui| {
                                ui.label(name);
                                responses.push(ui.add(
                                    DragValue::new(component).range(range.clone()).speed(speed),
                                ));
                            });
                        }
                    });
                });

                responses.into_inner().unwrap()
            }

            if ui.button("Delete.").clicked() {
                TerrainLine::delete(line_entity, &mut commands, &generated);
                line_selected.0 = None;
            }

            ui.add_space(10.);

            if ui
                .button(format!("Randomise Seed!\n{}", line.seed))
                .clicked()
            {
                line.generate = true;
                line.seed = thread_rng().next_u64();
            }

            ui.add_space(10.);

            if vec_ui(
                ui,
                [(&mut line.spacing.x, "x"), (&mut line.spacing.y, "y")],
                "spacing",
                1.0..=f32::INFINITY,
                1.,
            )
            .union()
            .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            if vec_ui(
                ui,
                [
                    (&mut line.offset_y_bounds.start, "min"),
                    (&mut line.offset_y_bounds.end, "max"),
                ],
                "offset_y_bounds",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            )
            .union()
            .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            if vec_ui(
                ui,
                [
                    (&mut line.offset_y_change.start, "min"),
                    (&mut line.offset_y_change.end, "max"),
                ],
                "offset_y_change",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            )
            .union()
            .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            if vec_ui(
                ui,
                [
                    (&mut line.roughness.start, "min"),
                    (&mut line.roughness.end, "max"),
                ],
                "roughness",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            )
            .union()
            .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            if vec_ui(
                ui,
                [
                    (&mut line.jitter_x.start, "min"),
                    (&mut line.jitter_x.end, "max"),
                ],
                "jitter_x",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            )
            .union()
            .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            if vec_ui(
                ui,
                [
                    (&mut line.jitter_y.start, "min"),
                    (&mut line.jitter_y.end, "max"),
                ],
                "jitter_y",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            )
            .union()
            .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            ui.label("depth");
            if ui
                .add(DragValue::new(&mut line.depth).speed(1).range(1..=u32::MAX))
                .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            ui.label("upwards_offset");
            if ui
                .add(DragValue::new(&mut line.upwards_offset).speed(1.))
                .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            ui.label("z");
            if ui.add(DragValue::new(&mut line.z).speed(1.)).changed() {
                line.generate = true;
            }

            ui.add_space(10.);

            ui.label("diameter");
            if ui
                .add(DragValue::new(&mut line.diameter).speed(1.))
                .changed()
            {
                line.generate = true;
            }

            ui.add_space(10.);

            ui.label("colour");
            if color_picker::color_edit_button_rgb(ui, &mut line.colour).changed() {
                line.generate = true;
            }

            ui.add_space(10.);

            if ui.checkbox(&mut line.collision, "collision").changed() {
                line.generate = true;
            }

            ui.add_space(10.);
        });
    }
}

/// Allows us to serialize the terrain lines.
#[derive(Serialize, Deserialize, Asset, TypePath)]
struct SerializeableTerrainLine {
    // Entity is opaque, so we just our own index, to keep track of everything.
    point_1: usize,
    point_2: usize,

    // Should the line (re)generate everything?
    generate: bool,

    // The seed that determines how the terrain will randomly generate.
    seed: u64,

    // How far to go forward in the x, and how far to go down in the y.
    spacing: Vec2,

    // Offset y translates y and creeps randomly up and down.
    offset_y_bounds: Range<f32>,
    offset_y_change: Range<f32>,

    // Randomly translates y, to make the terrain look rough or smooth.
    roughness: Range<f32>,

    // The jitter of every nodule
    jitter_x: Range<f32>,
    jitter_y: Range<f32>,

    // How many nodules we shall spawn in a downwards direction.
    // 1 means there will be 1 nodule.
    depth: u32,
    // Offsets all nodules this amount upwards.
    upwards_offset: f32,

    // Idea: Easing
    // Slowly pulls in the clamp (perhaps via lerp?) so that the nodules finish exactly (or inexactly) at the end point!

    // Depth to draw the nodules at.
    z: f32,

    // The diameter of the nodules.
    diameter: f32,

    // The colour of the nodules.
    colour: [f32; 3],

    // Whether we want to add colliders to the nodules.
    collision: bool,
}

/// A bunch of circles that look like terrain hopefully.
#[derive(Component)]
pub struct TerrainLine {
    point_1: Entity,
    point_2: Entity,

    // Should the line (re)generate everything?
    generate: bool,

    // The seed that determines how the terrain will randomly generate.
    seed: u64,

    // How far to go forward in the x, and how far to go down in the y.
    spacing: Vec2,

    // Offset y translates y and creeps randomly up and down.
    offset_y_bounds: Range<f32>,
    offset_y_change: Range<f32>,

    // Randomly translates y, to make the terrain look rough or smooth.
    roughness: Range<f32>,

    // The jitter of every nodule
    jitter_x: Range<f32>,
    jitter_y: Range<f32>,

    // How many nodules we shall spawn in a downwards direction.
    // 1 means there will be 1 nodule.
    depth: u32,
    // Offsets all nodules this amount upwards.
    upwards_offset: f32,

    // Idea: Easing
    // Slowly pulls in the clamp (perhaps via lerp?) so that the nodules finish exactly (or inexactly) at the end point!

    // Depth to draw the nodules at.
    z: f32,

    // The diameter of the nodules.
    diameter: f32,

    // The colour of the nodules.
    colour: [f32; 3],

    // Whether we want to add colliders to the nodules.
    collision: bool,
}

impl TerrainLine {
    /// Fills in all the defaults automatically. Only requires the points.
    fn new(points: (Entity, Entity)) -> Self {
        Self {
            point_1: points.0,
            point_2: points.1,

            generate: true,

            seed: thread_rng().next_u64(),

            spacing: Vec2::splat(20.),

            offset_y_bounds: (30. * -5.)..(30. * 5.),
            offset_y_change: -20.0..20.0,

            roughness: 0.0..0.0,

            jitter_x: -5.0..5.0,
            jitter_y: -5.0..5.0,

            depth: 1,
            upwards_offset: 0.,

            z: 0.,
            diameter: 30.,
            colour: [0.5, 0.5, 0.5],
            collision: true,
        }
    }

    /// Generates the terrain, and removes all previously generated terrain.
    fn generate(
        mut lines: Query<(Entity, &mut Self)>,
        points: Query<&Transform>,
        generated: Query<(Entity, &BelongsToTerrain)>,
        mut commands: Commands,
        asset_server: Res<AssetServer>,
    ) {
        lines.iter_mut().for_each(|(line_entity, mut line)| {
            if line.generate {
                line.generate = false;

                // Despawn all generated entities related to this line.
                generated.iter().for_each(|(generated_entity, generated)| {
                    if line_entity == generated.0 {
                        commands.entity(generated_entity).despawn();
                    }
                });

                let Ok(point_1_transform) = points.get(line.point_1) else {
                    return;
                };

                let Ok(point_2_transform) = points.get(line.point_2) else {
                    return;
                };

                // We need x ordering, due to a bug in the line function.
                let (point_1, point_2) =
                    if point_1_transform.translation.x < point_2_transform.translation.x {
                        (
                            point_1_transform.translation.xy(),
                            point_2_transform.translation.xy(),
                        )
                    } else {
                        (
                            point_2_transform.translation.xy(),
                            point_1_transform.translation.xy(),
                        )
                    };

                line.line(
                    line_entity,
                    LineEnd {
                        translation: point_1,
                        offset_y: 0.,
                    },
                    point_2,
                    &mut commands,
                    &asset_server,
                );

                info!("Generated a line!");
            }
        });
    }

    /// Deletes any lines that are missing points.
    fn validate(
        lines: Query<(Entity, &Self)>,
        generated: Query<(Entity, &BelongsToTerrain)>,
        mut commands: Commands,
    ) {
        lines.iter().for_each(|(entity, line)| {
            if commands.get_entity(line.point_1).is_none()
                || commands.get_entity(line.point_2).is_none()
            {
                Self::delete(entity, &mut commands, &generated);
            }
        });
    }

    fn delete(
        entity: Entity,
        commands: &mut Commands,
        generated: &Query<(Entity, &BelongsToTerrain)>,
    ) {
        commands.entity(entity).despawn();
        // Despawn all generated entities related to this line.
        generated.iter().for_each(|(generated_entity, generated)| {
            if entity == generated.0 {
                commands.entity(generated_entity).despawn();
            }
        });
        info!("Destroyed a line!");
    }

    /// Displays debug info about the line, such as the center, and what the limits are.
    fn debug(
        lines: Query<(Entity, &Self)>,
        points: Query<&Transform>,
        mut gizmos: Gizmos,
        line_selected: Res<LineSelected>,
    ) {
        lines.iter().for_each(|(line_entity, line)| {
            let Ok(point_1) = points.get(line.point_1) else {
                return;
            };

            let Ok(point_2) = points.get(line.point_2) else {
                return;
            };

            let colour = if line_selected
                .0
                .filter(|line_selected| *line_selected == line_entity)
                .is_some()
            {
                // Cheeky using this let statement to also show other debug info.
                gizmos.line_2d(
                    point_1.translation.xy() + Vec2::new(0., line.offset_y_bounds.start),
                    point_2.translation.xy() + Vec2::new(0., line.offset_y_bounds.start),
                    Color::srgb(0.0, 0.5, 0.0),
                );
                gizmos.line_2d(
                    point_1.translation.xy() + Vec2::new(0., line.offset_y_bounds.end),
                    point_2.translation.xy() + Vec2::new(0., line.offset_y_bounds.end),
                    Color::srgb(0.0, 0.5, 0.0),
                );

                Color::srgb(0.0, 0.0, 1.0)
            } else {
                Color::srgb(0.0, 1.0, 0.0)
            };

            gizmos.line_2d(point_1.translation.xy(), point_2.translation.xy(), colour);
        });
    }

    pub fn create<'a>(
        &self,
        entity: Entity,
        translation: Vec2,
        commands: &'a mut Commands,
        asset_server: &AssetServer,
    ) -> EntityCommands<'a> {
        let mut entity_commands = commands.spawn((
            Transform::from_translation(Vec3::new(translation.x, translation.y, self.z)),
            Sprite {
                image: asset_server.load("nodule.png"),
                color: Color::srgb(self.colour[0], self.colour[1], self.colour[2]),
                custom_size: Some(Vec2::splat(self.diameter)),
                ..default()
            },
            BelongsToTerrain(entity),
        ));

        if self.collision {
            entity_commands.insert(Radius {
                0: self.diameter / 2.,
            });
        }

        entity_commands
    }

    fn line(
        &self,
        entity: Entity,
        from: LineEnd,
        to: Vec2,
        commands: &mut Commands,
        asset_server: &AssetServer,
    ) -> LineEnd {
        //TODO: Each nodules should have its own rng seeded from the nodule x and y somehow, and then added to the main seed.
        // Otherwise we suffer huge rng changes from just increasing the depth.
        let mut rng = StdRng::seed_from_u64(self.seed);

        let mut to = LineEnd {
            translation: to,
            // TODO: Why???
            offset_y: from.offset_y,
        };

        //let distance = self.previous_translation.distance(to);
        let distance_x = (from.translation.x - to.translation.x).abs();
        let gradient =
            (to.translation.y - from.translation.y) / (to.translation.x - from.translation.x);

        // Hang on a second, why is this using distance? Shouldn't it only be x distance, not real distance???
        // I've replaced it with distance_x, but we should make certain that this is now correct.
        let end = (distance_x / self.spacing.x).round() as u32;

        for nodule_x in 0..end {
            let x = nodule_x as f32 * self.spacing.x;
            let y = x * gradient;

            //TODO: Remove and add a replacement in the debug system.
            // if Self::DEBUG {
            //     self.create(
            //         NoduleConfig {
            //             colour: [0., 0., 1.],
            //             ..default()
            //         },
            //         Vec2::new(x, y) + from.translation,
            //     );
            //     self.create(
            //         NoduleConfig {
            //             colour: [0., 1., 0.],
            //             ..default()
            //         },
            //         Vec2::new(x, y + config.offset_y_bounds.start) + from.translation,
            //     );
            //     self.create(
            //         NoduleConfig {
            //             colour: [0., 1., 0.],
            //             ..default()
            //         },
            //         Vec2::new(x, y + config.offset_y_bounds.end) + from.translation,
            //     );
            // }

            to.offset_y += rng.gen_range_allow_empty(self.offset_y_change.clone());
            to.offset_y = to
                .offset_y
                .clamp(self.offset_y_bounds.start, self.offset_y_bounds.end);

            let roughness = rng.gen_range_allow_empty(self.roughness.clone());

            for depth in 0..self.depth {
                let jitter = Vec2::new(
                    rng.gen_range_allow_empty(self.jitter_x.clone()),
                    rng.gen_range_allow_empty(self.jitter_y.clone()),
                );

                let translation = Vec2::new(
                    x,
                    y + to.offset_y + (depth as f32 * -self.spacing.y) + roughness,
                ) + from.translation
                    + jitter;

                self.create(entity, translation, commands, asset_server);
            }
        }

        to
    }
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

trait NotEmptyRange: Sized {
    fn not_empty_range() -> Range<Self>;
}

impl NotEmptyRange for f32 {
    fn not_empty_range() -> Range<Self> {
        0.0..1.0
    }
}

impl NotEmptyRange for f64 {
    fn not_empty_range() -> Range<Self> {
        0.0..1.0
    }
}

trait RngExtension {
    fn gen_range_allow_empty<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform + PartialEq + NotEmptyRange + PartialOrd,
        R: SampleRange<T> + RangeStartEnd<T>;
}

impl<Rng: RngCore> RngExtension for Rng {
    fn gen_range_allow_empty<T, R>(&mut self, range: R) -> T
    where
        T: SampleUniform + PartialEq + NotEmptyRange + PartialOrd,
        R: SampleRange<T> + RangeStartEnd<T>,
    {
        if range.start() == range.end() {
            // We still need to nudge the rng along, but we don't want the result, as it is unspecified nonsense.
            T::not_empty_range().sample_single(self);
            range.start()
        } else {
            range.sample_single(self)
        }
    }
}

#[derive(Resource, Default)]
pub struct GizmosLingering(
    Parallel<(
        Vec<usize>,
        Vec<(Duration, Box<dyn Fn(&mut Gizmos) + Send + Sync>)>,
    )>,
);

impl GizmosLingering {
    pub fn add(&self, duration: Duration, f: impl Fn(&mut Gizmos) + Send + Sync + 'static) {
        self.0.borrow_local_mut().1.push((duration, Box::new(f)));
    }
}

fn display_lingering_gizmos(
    mut gizmos_lingering: ResMut<GizmosLingering>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    gizmos_lingering
        .0
        .iter_mut()
        .for_each(|(gizmos_to_remove, gizmos_lingering)| {
            gizmos_lingering
                .iter_mut()
                .enumerate()
                .for_each(|(index, gizmos_lingering)| {
                    gizmos_lingering.0 = gizmos_lingering.0.saturating_sub(time.delta());
                    if gizmos_lingering.0.is_zero() {
                        gizmos_to_remove.push(index)
                    } else {
                        gizmos_lingering.1(&mut gizmos);
                    }
                });

            // Makes removing indices easy by removing them in the reverse order!
            gizmos_to_remove.sort_unstable();

            gizmos_to_remove.iter().rev().for_each(|index| {
                // The error is incorrect.
                #[allow(unused_must_use)]
                gizmos_lingering.swap_remove(*index);
            });

            gizmos_to_remove.clear();
        });
}

pub trait CompileTimeOption<T> {
    fn get(&self) -> Option<&T>;
}

impl<T> CompileTimeOption<T> for T {
    fn get(&self) -> Option<&T> {
        Some(self)
    }
}

// impl<T> CompileTimeOption<T> for () {

// }

//MARK: Response Ext
/// Makes response arrays a bit easier to work with.
/// Stolen from previous projects.
trait ResponseArray {
    fn union(self) -> Response;
}

impl<const N: usize> ResponseArray for [Response; N] {
    fn union(self) -> Response {
        let mut iter = self.into_iter();
        let mut response = iter.next().unwrap();
        // I feel a bit confused, but this should work?
        iter.for_each(|r| response = response.union(r));
        response
    }
}
