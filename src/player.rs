use bevy::color::palettes::css::RED;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{camera_follow, move_players, AbilityOrb, Player};
}

#[derive(Component)]
pub struct Player;

/// Move all players based on their inputs.
#[system(Update::Physics::BeforeUpdate)]
pub fn move_players(
    actions: Res<ActionState<Action>>,
    mut players: Query<&mut Verlet, With<Player>>,
) {
    const MOVE_SPEED: f32 = 1000.;

    players.iter_mut().for_each(|mut verlet| {
        let movement = actions
            .clamped_axis_pair(&Action::Move)
            .xy()
            .normalize_or_zero()
            * MOVE_SPEED;

        verlet.accelerate(movement);
    });
}

pub fn debug_action(actions: Res<ActionState<Action>>) {
    if actions.just_pressed(&Action::Debug) {
        error!("No debug action.")
    }
}

// Having the player framerate be slower than the camera was very unpleasant. Abandoned due to that.
// pub fn sync_player_transforms(mut players: Query<(&mut Transform, &Player)>) {
//     players.iter_mut().for_each(|(mut transform, player)| {
//         transform.translation.x = player.translation.x;
//         transform.translation.y = player.translation.y;
//     });
// }

/// Makes the camera follow the player.
pub fn camera_follow(
    time: Res<Time>,
    player: Option<Single<&Transform, With<Player>>>,
    mut camera_transform: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let Some(follow_transform) = player else {
        return;
    };

    let mut camera_transform = camera_transform.single_mut();

    let distance = follow_transform
        .translation
        .xy()
        .distance_squared(camera_transform.translation.xy());

    //println!("{distance}");

    let new_position = camera_transform.translation.xy().move_towards(
        follow_transform.translation.xy(),
        time.delta_secs() * distance,
    );

    camera_transform.translation.x = new_position.x;
    camera_transform.translation.y = new_position.y;
}

#[derive(Component)]
pub struct AbilityOrb {
    pub following: Option<Entity>,
    pub distance: f32,
}

// pub fn orb_follow(
//     mut orbs: Query<(&mut Transform, &AbilityOrb)>,
//     transforms: Query<&Transform, Without<AbilityOrb>>,
// ) {
//     orbs.par_iter_mut().for_each(|(mut transform, orb)| {
//         if let Some(following) = orb.following {
//             let other_transform = transforms.get(following).unwrap();
//             let translation = (transform.translation.xy() - other_transform.translation.xy())
//                 .normalize_or_zero()
//                 * orb.distance
//                 + other_transform.translation.xy();
//             transform.translation.x = translation.x;
//             transform.translation.y = translation.y;
//         }
//     });
// }

pub fn debug_collisions(
    mut gizmos: Gizmos,
    players: Query<&Transform, With<Player>>,
    colliders: Query<(&Transform, &Radius)>,
    collider_grid: Res<ColliderGrid>,
) {
    players.iter().for_each(|transform| {
        info!("{} translation", transform.translation.xy().round());
        if let Some(index) = collider_grid.translation_to_index(transform.translation.xy()) {
            info!("{} entities", collider_grid.cells[index].0.len());
            collider_grid.cells[index].0.iter().for_each(|entity| {
                let (transform, collider) = colliders.get(*entity).unwrap();

                gizmos.circle_2d(transform.translation.xy(), collider.0, RED);
            });
        }
    });
}

/// Allows zooming in and out.
/// Useful for when you are trying to see how a change affects a big area.
#[system(Update)]
fn debug_zoom(
    time: Res<Time>,
    actions: Res<ActionState<Action>>,
    camera: Option<Single<&mut OrthographicProjection, With<Camera2d>>>,
    mut menu: MenuReader,
) {
    if !menu.is(Menu::InGame) {
        return;
    }

    const ZOOM_SPEED: f32 = 10.;

    let Some(mut camera) = camera else {
        return;
    };

    camera.scale +=
        actions.axis_data(&Action::Zoom).unwrap().value * ZOOM_SPEED * time.delta_secs();
}
