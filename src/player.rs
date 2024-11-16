pub use crate::prelude::*;

pub mod prelude {
    pub use super::{camera_follow, move_players, orb_follow, AbilityOrb, Player};
}

#[derive(Component)]
pub struct Player;

pub fn move_players(
    time: Res<Time>,
    actions: Res<ActionState<Action>>,
    mut players: Query<&mut particle::Velocity, With<Player>>,
) {
    const MOVE_SPEED: f32 = 50.;

    players.iter_mut().for_each(|mut velocity| {
        let movement =
            actions.clamped_axis_pair(&Action::Move).xy() * MOVE_SPEED * time.delta_seconds();

        **velocity += movement;
        velocity.y -= 0.5;

        //info!("{}", motion.amount);
    });
}

// Having the player framerate be slower than the camera was very unpleasant. Abandoned due to that.
// pub fn sync_player_transforms(mut players: Query<(&mut Transform, &Player)>) {
//     players.iter_mut().for_each(|(mut transform, player)| {
//         transform.translation.x = player.translation.x;
//         transform.translation.y = player.translation.y;
//     });
// }

pub fn camera_follow(
    time: Res<Time>,
    players: Query<&Transform, With<Player>>,
    mut camera_transform: Query<&mut Transform, (With<Camera>, Without<Player>)>,
) {
    let follow_transform = players.single();

    let mut camera_transform = camera_transform.single_mut();

    let distance = follow_transform
        .translation
        .xy()
        .distance_squared(camera_transform.translation.xy());

    //println!("{distance}");

    let new_position = camera_transform.translation.xy().move_towards(
        follow_transform.translation.xy(),
        time.delta_seconds() * distance,
    );

    camera_transform.translation.x = new_position.x;
    camera_transform.translation.y = new_position.y;
}

#[derive(Component)]
pub struct AbilityOrb {
    pub following: Option<Entity>,
    pub distance: f32,
}

pub fn orb_follow(
    mut orbs: Query<(&mut Transform, &AbilityOrb)>,
    transforms: Query<&Transform, Without<AbilityOrb>>,
) {
    orbs.par_iter_mut().for_each(|(mut transform, orb)| {
        if let Some(following) = orb.following {
            let other_transform = transforms.get(following).unwrap();
            let translation = (transform.translation.xy() - other_transform.translation.xy())
                .normalize_or_zero()
                * orb.distance
                + other_transform.translation.xy();
            transform.translation.x = translation.x;
            transform.translation.y = translation.y;
        }
    });
}
