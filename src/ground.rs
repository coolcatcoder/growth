use bevy::ecs::system::EntityCommands;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::create_terrain;
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
    commands.spawn((
        Transform::from_translation(Vec3::new(translation.x, translation.y, depth)),
        Sprite {
            image: asset_server.load("nodule.png"),
            color: Color::Srgba(Srgba::rgb(colour[0], colour[1], colour[2])),
            ..default()
        },
    ))
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
