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

use std::ops::Range;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    window::PrimaryWindow,
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
    pub use super::{squared, Action, GizmosLingering, Grower, NoduleConfig, Terrain};
    pub use crate::{
        collision::prelude::*, ground::prelude::*, particle, player::prelude::*, sun::prelude::*,
        time::prelude::*, tree::prelude::*,
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
        .insert_resource(Action::default_input_map())
        .insert_resource(ColliderGrid::new(GRID_ORIGIN))
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
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
            .with(Self::AddPoint, ButtonlikeChord::from_single(MouseButton::Left).with(KeyCode::KeyQ))
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

pub fn update_cursor_translation(
    mut cursor_position: ResMut<CursorWorldTranslation>,
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
        //previous_cursor_position.0 = cursor_position.0;
        cursor_position.0 = world_position;
        //println!("cursor coords: {},{}", world_position.x, world_position.y);
    }
}

//MARK: TerrainLine
#[derive(Component)]
pub struct TerrainLine {
    point_1: Entity,
    point_2: Entity,
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

impl<Rng: RngCore> RngExtension for Rng {
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
