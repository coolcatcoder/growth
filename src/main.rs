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
#![feature(macro_metavar_expr)]
#![feature(macro_metavar_expr_concat)]

use std::ops::{Range, RangeInclusive};

use arrayvec::ArrayVec;
use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    ecs::query::QueryFilter,
    window::PrimaryWindow,
};

mod collision;
mod editor;
mod error_handling;
mod events;
mod game_menu;
mod ground;
mod input;
mod localisation;
mod main_menu;
mod menus;
pub mod particle;
mod plant;
mod player;
mod profile;
mod saving;
mod sun;
mod time;
mod tree;
mod ui;
mod verlet;

mod prelude {
    pub use super::{
        squared, CursorPreviousWorldTranslation, CursorWorldTranslation, GizmosLingering, Grower,
        PlantCell, PlantCellTemplate, QueryExtensions, WorldOrCommands,
    };
    pub use crate::{
        collision::prelude::*, editor::prelude::*, error_handling::prelude::*, ground::prelude::*,
        input::prelude::*, localisation::prelude::*, menus::prelude::*, ok_or_error_and_return,
        particle, player::prelude::*, profile::prelude::*, saving::prelude::*, some_or_return,
        sun::prelude::*, time::prelude::*, tree::prelude::*, ui::prelude::*, verlet::prelude::*,
    };
    pub use bevy::{
        asset::LoadedFolder,
        color::palettes::basic::*,
        ecs::{
            query::{QueryData, WorldQuery},
            system::{EntityCommands, SystemParam},
        },
        prelude::*,
        utils::{HashMap, Parallel},
    };
    pub use bevy_common_assets::json::JsonAssetPlugin;
    pub use bevy_egui::{
        egui::{self, color_picker, DragValue, Response},
        EguiContexts, EguiPlugin,
    };
    pub use bevy_registration::prelude::*;
    pub use bevy_text_edit::TextEditable;
    pub use derive_more::{Deref, DerefMut};
    pub use leafwing_input_manager::prelude::*;
    pub use procedural_macros::*;
    pub use rand::prelude::*;
    pub use rayon::prelude::*;
    pub use serde::{Deserialize, Serialize};
    pub use std::{
        any::TypeId,
        fs::{self, File},
        path::Path,
        sync::Arc,
        time::Duration,
    };
}

use bevy_text_edit::TextEditPluginNoState;
use prelude::*;

// The world is dying. Save it. The sun will eventually hit the world. Hope they realise that sooner rather than later!
// Energy is area, roughly 1 energy for 700 area (30 diameter circle). You can only store as much energy as your area will allow.

// Big main world full of plants is cool. You see it during the beginning. Then suddenly instead of all the little sun, you see a huge chunk hit everything.
// It all goes to yellow.
// Remake the world, from the few dying plants surviving on small bits of floating rubble.

// Player weird floating orb thing, with smaller orbs orbiting. after the crash you have few, and must slowly collect them, and gather abilities
// Before you get a power upgrade, you are forced to deal with a battery which lasts only a few seconds. Your screen quickly getting darker and darker.
// You recharge using the sun. Too much hurts.

schedule! {
    Update(
        Early,
        UnloadMenus,
        LoadMenus,
        [run_every(Duration::from_secs_f64(1. / 30.))]
        Physics(
            BeforeUpdate,
            Update,
            Chain,
            CollisionResolution,
            SyncPositions,
        ),
        SaveAndLoad,
    )
}

/*
.add_systems_that_run_every(
            Duration::from_secs_f64(verlet::TIME_DELTA_SECONDS),
            // TODO: How do we want to deal with deffered changes? Surely we would want it to apply deffered at the end of every loop. I think .chain() does this?
            (
                (AmbientFriction::update, Gravity::update, move_players),
                Verlet::update,
                (Grounded::update),
                (Verlet::solve_collisions),
                (Verlet::sync_position),
            )
                .chain(),
        ) */

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins,
            //FrameTimeDiagnosticsPlugin,
            //LogDiagnosticsPlugin::default(),
            InputManagerPlugin::<Action>::default(),
            RunEveryPlugin,
            EguiPlugin,
            RegistrationPlugin,
            TextEditPluginNoState,
        ))
        //.add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                (PlantCell::update),
                (
                    LineSelected::ui,
                    TerrainLine::on_load,
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
                player::debug_action,
                particle::Verlet::system,
                display_lingering_gizmos,
                //debug_move_camera,
                player::debug_collisions,
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
        .add_systems(
            PostUpdate,
            (
                camera_follow,
                //Verlet::sync_position.before(camera_follow)
            )
                .before(TransformSystem::TransformPropagate),
        )
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
        .init_resource::<CursorWorldTranslation>()
        .init_resource::<CursorPreviousWorldTranslation>()
        .init_resource::<LineSelected>()
        .insert_resource(ColliderGrid::new(GRID_ORIGIN))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>, mut load: Load) {
    // Temporary as for now we care only for the world.
    load.path("./map");

    let player_translation = Vec2::new(0., 500.);

    info!("player {}", player_translation);

    commands.spawn((
        Player,
        Transform::from_translation(Vec3::new(0., 0., 1.)),
        Sprite {
            image: asset_server.load("nodule.png"),
            color: Color::Srgba(Srgba::rgb(1.0, 0.0, 0.0)),
            ..default()
        },
        Radius { 0: 15. },
        Verlet::from_translation(player_translation),
        AmbientFriction,
        Gravity,
        Extrapolate,
    ));

    commands.spawn(Camera2d);
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
    window: Option<Single<&Window, With<PrimaryWindow>>>,
    camera: Option<Single<(&Camera, &GlobalTransform)>>,
) {
    let Some(camera) = camera else {
        return;
    };
    let (camera, camera_transform) = (camera.0, camera.1);

    let Some(window) = window else {
        return;
    };

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

//MARK: TerrainPoint
/// A point that can be used in terrain lines.
#[derive(Component, Default, SaveAndLoad)]
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
        if false {
            //if actions.just_pressed(&Action::AddPoint) {
            commands.spawn((
                SaveConfig {
                    path: "./map".into(),
                },
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
        if false {
            //if actions.pressed(&Action::RemovePoint) {
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
        if false {
            //if actions.pressed(&Action::TranslatePoint) {
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
        if false {
            //if actions.pressed(&Action::CreateOrSelectLineMode) {
            if false {
                //if !actions.just_pressed(&Action::Select) {
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
                            .spawn((
                                SaveConfig {
                                    path: "./map".into(),
                                },
                                TerrainLine::new((points_selected[0].0, points_selected[1].0)),
                            ))
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
        mut copied: Local<Option<TerrainLine>>,
    ) {
        let Some(line_entity) = line_selected.0 else {
            return;
        };

        let Ok(line) = lines.get_mut(line_entity) else {
            return;
        };
        let line = line.into_inner();

        egui::SidePanel::left("Line Editor").show(contexts.ctx_mut(), |ui| {
            struct LineUi<'a> {
                generate: bool,
                ui: &'a mut egui::Ui,
            }

            impl LineUi<'_> {
                fn gap(&mut self) {
                    self.ui.add_space(10.);
                }

                fn single<T: bevy_egui::egui::emath::Numeric>(
                    &mut self,
                    value: &mut T,
                    name: &str,
                    allowed_range: RangeInclusive<T>,
                    speed: f32,
                ) {
                    self.ui.label(name);
                    if self
                        .ui
                        .add(DragValue::new(value).speed(speed).range(allowed_range))
                        .changed()
                    {
                        self.generate = true;
                    }
                    self.gap();
                }

                fn vec<const N: usize, T: bevy_egui::egui::emath::Numeric>(
                    &mut self,
                    vec_and_component_names: [(&mut T, &str); N],
                    name: &str,
                    allowed_range: RangeInclusive<T>,
                    speed: f32,
                ) {
                    let mut responses = ArrayVec::<_, N>::new();

                    self.ui.vertical(|ui| {
                        ui.label(name);

                        ui.horizontal(|ui| {
                            for (component, name) in vec_and_component_names {
                                ui.vertical(|ui| {
                                    ui.label(name);
                                    responses.push(
                                        ui.add(
                                            DragValue::new(component)
                                                .range(allowed_range.clone())
                                                .speed(speed),
                                        ),
                                    );
                                });
                            }
                        });
                    });

                    if responses.into_inner().unwrap().union().changed() {
                        self.generate = true;
                    }

                    self.gap();
                }

                fn range<T: bevy_egui::egui::emath::Numeric>(
                    &mut self,
                    range: &mut RangeInclusive<T>,
                    name: &str,
                    allowed_range: RangeInclusive<T>,
                    speed: f32,
                ) {
                    let mut range_start = *range.start();
                    let mut range_end = *range.end();

                    let vec_and_component_names =
                        [(&mut range_start, "min"), (&mut range_end, "max")];

                    let mut responses = ArrayVec::<_, 2>::new();

                    self.ui.vertical(|ui| {
                        ui.label(name);

                        ui.horizontal(|ui| {
                            for (component, name) in vec_and_component_names {
                                ui.vertical(|ui| {
                                    ui.label(name);
                                    responses.push(
                                        ui.add(
                                            DragValue::new(component)
                                                .range(allowed_range.clone())
                                                .speed(speed),
                                        ),
                                    );
                                });
                            }
                        });
                    });

                    if responses.into_inner().unwrap().union().changed() {
                        *range = range_start..=range_end;
                        self.generate = true;
                    }

                    self.gap();
                }
            }

            let mut ui = LineUi {
                generate: false,
                ui,
            };

            if ui.ui.button("Delete.").clicked() {
                TerrainLine::delete(line_entity, &mut commands, &generated);
                line_selected.0 = None;
            }

            ui.gap();

            if ui
                .ui
                .button(format!("Randomise Seed!\n{}", line.seed))
                .clicked()
            {
                ui.generate = true;
                line.seed = thread_rng().next_u64();
            }

            ui.gap();

            if ui.ui.button("copy").clicked() {
                *copied = Some(line.clone());
            }

            if let Some(copied) = &mut *copied {
                if ui.ui.button("paste").clicked() {
                    copied.point_1 = line.point_1;
                    copied.point_2 = line.point_2;
                    copied.seed = line.seed;

                    *line = copied.clone();
                    line.generate = true;
                }
            }

            ui.gap();

            ui.vec(
                [(&mut line.spacing.x, "x"), (&mut line.spacing.y, "y")],
                "spacing",
                1.0..=f32::INFINITY,
                1.,
            );

            ui.range(
                &mut line.offset_y_bounds,
                "offset_y_bounds",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            );

            ui.range(
                &mut line.offset_y_change,
                "offset_y_change",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            );

            ui.range(
                &mut line.roughness,
                "roughness",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            );

            ui.range(
                &mut line.jitter_x,
                "jitter_x",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            );

            ui.range(
                &mut line.jitter_y,
                "jitter_y",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            );

            ui.single(&mut line.depth, "depth", 1..=u32::MAX, 1.);

            ui.single(
                &mut line.upwards_offset,
                "upwards_offset",
                f32::NEG_INFINITY..=f32::INFINITY,
                1.,
            );

            ui.single(&mut line.z, "z", f32::NEG_INFINITY..=f32::INFINITY, 1.);

            ui.single(&mut line.diameter, "diameter", 0.0..=f32::INFINITY, 1.);

            ui.ui.label("colour");
            if color_picker::color_edit_button_rgb(ui.ui, &mut line.colour).changed() {
                ui.generate = true;
            }

            ui.gap();

            if ui.ui.checkbox(&mut line.collision, "collision").changed() {
                ui.generate = true;
            }

            ui.gap();

            if ui.generate {
                line.generate = true;
            }
        });
    }
}

/// A bunch of circles that look like terrain hopefully.
#[derive(Component, SaveAndLoad, Clone)]
struct TerrainLine {
    point_1: Entity,
    point_2: Entity,

    // Should the line (re)generate everything?
    generate: bool,

    // The seed that determines how the terrain will randomly generate.
    seed: u64,

    // How far to go forward in the x, and how far to go down in the y.
    spacing: Vec2,

    // Offset y translates y and creeps randomly up and down.
    offset_y_bounds: RangeInclusive<f32>,
    offset_y_change: RangeInclusive<f32>,

    // Randomly translates y, to make the terrain look rough or smooth.
    roughness: RangeInclusive<f32>,

    // The jitter of every nodule
    jitter_x: RangeInclusive<f32>,
    jitter_y: RangeInclusive<f32>,

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

            offset_y_bounds: (30. * -5.)..=(30. * 5.),
            offset_y_change: -20.0..=20.0,

            roughness: 0.0..=0.0,

            jitter_x: -5.0..=5.0,
            jitter_y: -5.0..=5.0,

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

                // Must go after we know all points have loaded properly.
                line.generate = false;

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

                line.line(line_entity, point_1, point_2, &mut commands, &asset_server);
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
            // We only care if the entity exists. Even if it does not have the required components, it may still be loading.
            if commands.get_entity(line.point_1).is_none()
                || commands.get_entity(line.point_2).is_none()
            {
                Self::delete(entity, &mut commands, &generated);
            }
        });
    }

    /// Generates the lines when they load.
    fn on_load(mut load_finish: EventReader<LoadFinish>, mut commands: Commands) {
        load_finish.read().for_each(|load_finish| {
            if !load_finish.is_component::<Self>() {
                return;
            }

            commands
                .entity(load_finish.entity)
                .entry::<Self>()
                .and_modify(|mut line| {
                    line.generate = true;
                });
        });
    }

    /// Deletes itself properly. Makes sure to delete every generated entity aswell.
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
                    point_1.translation.xy()
                        + Vec2::new(0., line.offset_y_bounds.start() + line.upwards_offset),
                    point_2.translation.xy()
                        + Vec2::new(0., line.offset_y_bounds.start() + line.upwards_offset),
                    Color::srgb(0.0, 0.5, 0.0),
                );
                gizmos.line_2d(
                    point_1.translation.xy()
                        + Vec2::new(0., line.offset_y_bounds.end() + line.upwards_offset),
                    point_2.translation.xy()
                        + Vec2::new(0., line.offset_y_bounds.end() + line.upwards_offset),
                    Color::srgb(0.0, 0.5, 0.0),
                );

                Color::srgb(0.0, 0.0, 1.0)
            } else {
                Color::srgb(0.0, 1.0, 0.0)
            };

            gizmos.line_2d(
                point_1.translation.xy() + Vec2::new(0., line.upwards_offset),
                point_2.translation.xy() + Vec2::new(0., line.upwards_offset),
                colour,
            );
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
        from: Vec2,
        to: Vec2,
        commands: &mut Commands,
        asset_server: &AssetServer,
    ) {
        //TODO: Each nodules should have its own rng seeded from the nodule x and y somehow, and then added to the main seed.
        // Otherwise we suffer huge rng changes from just increasing the depth.
        let mut rng = StdRng::seed_from_u64(self.seed);

        let mut offset_y = 0.;

        //let distance = self.previous_translation.distance(to);
        let distance_x = (from.x - to.x).abs();
        let gradient = (to.y - from.y) / (to.x - from.x);

        // Hang on a second, why is this using distance? Shouldn't it only be x distance, not real distance???
        // I've replaced it with distance_x, but we should make certain that this is now correct.
        let end = (distance_x / self.spacing.x).round() as u32;

        for nodule_x in 0..end {
            let x = nodule_x as f32 * self.spacing.x;
            let y = x * gradient;

            offset_y += rng.gen_range(self.offset_y_change.clone());
            offset_y = offset_y.clamp(*self.offset_y_bounds.start(), *self.offset_y_bounds.end());

            let roughness = rng.gen_range(self.roughness.clone());

            for depth in 0..self.depth {
                let jitter = Vec2::new(
                    rng.gen_range(self.jitter_x.clone()),
                    rng.gen_range(self.jitter_y.clone()),
                );

                let translation = Vec2::new(
                    x,
                    y + offset_y
                        + (depth as f32 * -self.spacing.y)
                        + roughness
                        + self.upwards_offset,
                ) + from
                    + jitter;

                self.create(entity, translation, commands, asset_server);
            }
        }
    }
}

pub fn squared(value: f32) -> f32 {
    value * value
}

#[init]
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

//MARK: Query Ext
pub trait QueryExtensions {
    // owned query or borrow query??? Or generic get query from world???
    fn despawn_all(&self, commands: &mut Commands);
}

impl<F: QueryFilter> QueryExtensions for Query<'_, '_, Entity, F> {
    fn despawn_all(&self, commands: &mut Commands) {
        self.iter().for_each(|entity| {
            if let Some(mut entity) = commands.get_entity(entity) {
                entity.despawn()
            }
        });
    }
}

//MARK: World or Commands
pub trait WorldOrCommands {
    fn spawn_empty_and_get_id(&mut self) -> Entity;
}

impl WorldOrCommands for Commands<'_, '_> {
    fn spawn_empty_and_get_id(&mut self) -> Entity {
        self.spawn_empty().id()
    }
}

impl WorldOrCommands for World {
    fn spawn_empty_and_get_id(&mut self) -> Entity {
        self.spawn_empty().id()
    }
}

// erase_idents! {
// fn blah() {
//     let mut silly: u32 = 5;
//     // Comment that explains everything.
//     silly = 10;
//     {
//         let grah = 3;
//     }
//     error!("oh no");
// }
// }

//MARK: unwrap
/// Either some, or it returns optionally with a value.
#[macro_export]
macro_rules! some_or_return {
    ($option:expr) => {
        if let Some(value) = $option {
            value
        } else {
            return;
        }
    };

    ($option:expr, $return:expr) => {
        if let Some(value) = $option {
            value
        } else {
            return $return;
        }
    };
}

#[macro_export]
macro_rules! ok_or_error_and_return {
    ($result:expr) => {
        match $result {
            Ok(result) => result,
            Err(error) => {
                error!("{error}");
                return;
            }
        }
    };

    // Instead of an expression at the end, accept any tokens, and do format!("{} {error}", format!(tokens)).
    ($result:expr, $message:expr) => {
        match $result {
            Ok(result) => result,
            Err(error) => {
                error!("{} {error}", $message);
                return;
            }
        }
    };
}

#[derive(Clone)]
pub struct PlantCellTemplate {
    pub grow_chance_every: Duration,

    pub grow_chance: f32,

    pub grow_chance_change_after_success: f32,
    pub grow_chance_change_after_failure: f32,

    pub grow_chance_clamp: Range<f32>,

    pub grow_into: Vec<usize>,
}

#[derive(Component)]
pub struct PlantCell {
    pub time_passed: Duration,
    // Grow into grow_into[grow_into_pointer] and then add 1 to grow_into_pointer, and mod it by the length.
    pub grow_into_pointer: usize,

    base: PlantCellTemplate,

    // Templates are per plant, and read only. Once the plant entirely dies out, this will be deallocated.
    templates: Arc<Vec<PlantCellTemplate>>,
}

impl PlantCell {
    /// Constructs a new PlantCell from a vec of templates and fills in its own fields by indexing the vec with index.
    pub fn new(templates: Arc<Vec<PlantCellTemplate>>, index: usize) -> Self {
        let mut rng = thread_rng();

        Self {
            time_passed: rng.gen_range(Duration::ZERO..(templates[index].grow_chance_every)),
            grow_into_pointer: 0,

            base: templates[index].clone(),
            templates,
        }
    }

    /// Updates each cell, and grows new ones occasionally.
    fn update(
        mut cells: Query<(&mut PlantCell, &Transform)>,
        time: Res<Time>,
        commands: ParallelCommands,
        asset_server: Res<AssetServer>,
    ) {
        cells.par_iter_mut().for_each(|(cell, transform)| {
            let cell = cell.into_inner();

            //TODO: Do I want deterministic growing? Perhaps eventually.
            // For now this will work well enough.
            let mut rng = thread_rng();

            cell.time_passed += time.delta();

            while cell.time_passed >= cell.base.grow_chance_every {
                cell.time_passed -= cell.base.grow_chance_every;

                // If it over 1 or under 0 then we can obviously know whether it will grow or not.
                let grow = if cell.base.grow_chance >= 1. {
                    true
                } else if cell.base.grow_chance <= 0. {
                    false
                } else {
                    rng.gen_bool(cell.base.grow_chance as f64)
                };

                if grow {
                    info!("Grow");
                    cell.base.grow_chance += cell.base.grow_chance_change_after_success;

                    let new_cell = (
                        Self::new(cell.templates.clone(), cell.grow_into_pointer),
                        Transform::from_translation(transform.translation + Vec3::new(0., 30., 0.)),
                        Sprite {
                            image: asset_server.load("nodule.png"),
                            ..default()
                        },
                    );

                    commands.command_scope(|mut commands| {
                        commands.spawn(new_cell);
                    });

                    // Wrap around, so the pointer is always valid.
                    cell.grow_into_pointer += 1 % cell.templates.len();
                } else {
                    info!("No");
                    cell.base.grow_chance += cell.base.grow_chance_change_after_failure;
                }

                cell.base.grow_chance = cell.base.grow_chance.clamp(
                    cell.base.grow_chance_clamp.start,
                    cell.base.grow_chance_clamp.end,
                );
            }
        });
    }
}

struct Profile {
    identifier: String,
}
