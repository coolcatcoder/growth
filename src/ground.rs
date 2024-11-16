use bevy::ecs::system::EntityCommands;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{create_rubble, Ground};
}

#[derive(Component)]
pub struct Ground {
    timer: Timer,
    distance_from_last_plant: u16,
}

impl Grower for Ground {
    type SystemParameters<'w, 's> = (Res<'w, Time>, Res<'w, AssetServer>, Commands<'w, 's>);
    type Components<'a> = &'a Transform;

    fn tick(
        &mut self,
        system_parameters: &mut Self::SystemParameters<'_, '_>,
        components: <Self::Components<'_> as WorldQuery>::Item<'_>,
    ) {
        let (time, asset_server, commands) = system_parameters;
        let transform = components;

        self.timer.tick(time.delta());

        if self.timer.just_finished() {
            Self::create(
                self.distance_from_last_plant,
                transform.translation.xy(),
                commands,
                asset_server,
            );
        }
    }
}

impl Ground {
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

pub fn create_rubble<'a>(
    translation: Vec2,
    commands: &'a mut Commands,
    asset_server: &AssetServer,
) -> EntityCommands<'a> {
    commands.spawn(SpriteBundle {
        texture: asset_server.load("nodule.png"),
        transform: Transform::from_translation(Vec3::new(translation.x, translation.y, 0.)),
        sprite: Sprite {
            color: Color::Srgba(Srgba::rgb(0.5, 0.5, 0.5)),
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
