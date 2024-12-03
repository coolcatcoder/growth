use bevy::ecs::system::EntityCommands;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{create_terrain, Ground};
}

pub struct GrowTimer {
    timer: EveryTime,
}

#[derive(Component)]
pub struct Ground {
    timer: Timer,
    distance_from_last_plant: u16,
}

impl Ground {
    pub fn grow(
        mut grounds: Query<(&mut Ground, &Transform)>,
        time: Res<Time>,
        asset_server: Res<AssetServer>,
        mut commands: Commands,
    ) {
        grounds.iter_mut().for_each(|(mut ground, transform)| {
            ground.timer.tick(time.delta());

            if ground.timer.just_finished() {
                Self::create(
                    ground.distance_from_last_plant,
                    transform.translation.xy(),
                    &mut commands,
                    &asset_server,
                );
            }
        });
    }

    pub fn create(
        mut distance_from_last_plant: u16,
        mut translation: Vec2,
        commands: &mut Commands,
        asset_server: &AssetServer,
    ) {
        const MIN_TRANSLATION: Vec2 = Vec2::new(25., -30.);
        const MAX_TRANSLATION: Vec2 = Vec2::new(30., 30.);

        let mut rng = thread_rng();

        translation.x += rng.gen_range(MIN_TRANSLATION.x..MAX_TRANSLATION.x);
        translation.y += rng.gen_range(MIN_TRANSLATION.y..MAX_TRANSLATION.y);

        if distance_from_last_plant > 15 && rng.gen_bool(0.05) {
            distance_from_last_plant = 0;
            Tree::create(
                0,
                rng.gen_range(30..500),
                translation,
                commands,
                asset_server,
            );
        }

        commands.spawn((
            Self {
                timer: Timer::from_seconds(0.2, TimerMode::Once),
                distance_from_last_plant: distance_from_last_plant + 1,
            },
            SpriteBundle {
                texture: asset_server.load("nodule.png"),
                transform: Transform::from_translation(Vec3::new(
                    translation.x,
                    translation.y.max(-1000.),
                    0.,
                )),
                sprite: Sprite {
                    color: Color::Srgba(Srgba::rgb(0.0, 1.0, 0.0)),
                    ..default()
                },
                ..default()
            },
        ));
    }
}

#[derive(Component)]
pub struct Rock {}

impl Rock {
    //pub fn create()
}

pub fn create_terrain<'a>(
    translation: Vec2,
    depth: f32,
    colour: [f32; 3],
    commands: &'a mut Commands,
    asset_server: &AssetServer,
) -> EntityCommands<'a> {
    commands.spawn(SpriteBundle {
        texture: asset_server.load("nodule.png"),
        transform: Transform::from_translation(Vec3::new(translation.x, translation.y, depth)),
        sprite: Sprite {
            color: Color::Srgba(Srgba::rgb(colour[0], colour[1], colour[2])),
            ..default()
        },
        ..default()
    })
}

// fn create_rubble(min_offset: Vec2, max_offset: Vec2, min_translation: Vec2, max_translation: Vec2, width: u16, height: u16, commands: &mut Commands, asset_server: &AssetServer) {
//     let mut rng = thread_rng();

//     let mut previous_translation = Vec2::new(-27530., 0.);
//     for x in 0..width {
//         let mut downward_translation = previous_translation;

//         for y in 0..height {
//             let mut rock = commands.spawn(SpriteBundle {
//                 texture: asset_server.load("nodule.png"),
//                 transform: Transform::from_translation(Vec3::new(
//                     downward_translation.x + rng.gen_range(min_offset.x..max_offset.x),
//                     downward_translation.y + rng.gen_range(min_offset.y..max_offset.y),
//                     0.,
//                 )),
//                 sprite: Sprite {
//                     color: Color::Srgba(Srgba::rgb(0.5, 0.5, 0.5)),
//                     ..default()
//                 },
//                 ..default()
//             });

//             // 2, set to 5 for optimisation fun
//             if y <= 2 {
//                 rock.insert(Collider::radius(15.));
//             }

//             downward_translation.y -= 30.;
//         }

//         previous_translation.x += rng.gen_range(min_translation.x..max_translation.x);
//         previous_translation.y += rng.gen_range(min_translation.y..max_translation.y);
//     }
// }
