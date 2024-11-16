#![allow(clippy::type_complexity)]
#![warn(clippy::pedantic)]
// Not crimes.
#![allow(clippy::wildcard_imports)]
#![allow(clippy::needless_pass_by_value)]
// Crimes that are hard to fix.
// Sometimes crimes.
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_possible_truncation)]
// Unstable features:
#![feature(generic_const_exprs)]

use std::array;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

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
    pub use leafwing_input_manager::prelude::*;
    pub use rand::prelude::*;
    pub use rayon::prelude::*;
    pub use std::time::Duration;
}
use particle::DisableMotionOnCollision;
use prelude::*;

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
        ))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                //debug_move_camera,
                move_players,
                plant::Boulder::update,
                Ground::grower,
                Tree::grower,
                Leaf::grower,
                Sun::update,
                (
                    particle::Ticker::update_time,
                    ((
                        particle::DisableMotionOnCollision::system,
                        particle::StepUp::system,
                        particle::AmbientFriction::system,
                    )
                        .chain_ignore_deferred()),
                    particle::Motion::system,
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

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let mut rng = thread_rng();

    let mut player_translation = Vec2::ZERO;
    let mut previous_translation = Vec2::new(0.0, 0.0);
    for x in 0..120 {
        if x == 61 {
            player_translation = previous_translation
        }

        let mut depth = previous_translation.y;
        for y in 0..100 {
            if y == 50 {
                depth -= 1000.;
            }

            let mut rubble = create_rubble(
                Vec2::new(previous_translation.x + rng.gen_range(20.0..40.0), depth),
                &mut commands,
                &asset_server,
            );

            if y == 0 {
                rubble.insert(Collider { radius: 15. });
            }

            depth -= 30.;
        }

        previous_translation.x += 30.;

        if x <= 30 {
            previous_translation.y += rng.gen_range(10.0..20.0);
        } else if x <= 60 {
            previous_translation.y -= rng.gen_range(10.0..20.0);
        } else if x <= 90 {
            previous_translation.y += rng.gen_range(10.0..20.0);
        } else {
            previous_translation.y -= rng.gen_range(10.0..20.0)
        }
    }

    info!("player {}", player_translation);

    commands.spawn((
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
        particle::Motion::new(Vec2::new(0., -5.), [true, true]),
        particle::DisableMotionOnCollision,
        Collider { radius: 15. },
        particle::StepUp(60.),
        particle::AmbientFriction(Vec2::splat(0.05)),
    ));

    commands.spawn(Camera2dBundle { ..default() });
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
    const MOVE_SPEED: f32 = 300.;
    const ZOOM_SPEED: f32 = 5.;

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
