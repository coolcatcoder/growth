use core::f32;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{circle_to_energy, ellipse_to_energy, AbsorbSun, Energy, Sun};
}

pub fn ellipse_to_energy(x_radius: f32, y_radius: f32) -> f32 {
    //f32::consts::PI * x * y / (f32::consts::PI * 15. * 15.)
    // Classpad simplifies down to:
    (x_radius * y_radius) / 225.
}

pub fn circle_to_energy(radius: f32) -> f32 {
    // See ellipse_to_energy.
    (radius * radius) / 225.
}

// The entity should have the Energy component.
// If the entity somehow doesn't exist, or doesn't have the component, then Idk what should happen.
#[derive(Component)]
pub struct AbsorbSun(pub Entity);

// It is up to the plant to have an energy limit.
#[derive(Component)]
pub struct Energy(pub f32);

#[derive(Component)]
pub struct Sun {
    time_passed: f32,
    pub energy: f32,
}

impl Sun {
    pub fn create(
        energy: f32,
        translation: Vec2,
        commands: &mut Commands,
        asset_server: &AssetServer,
    ) {
        let mut rng = thread_rng();

        let diameter = f32::from(energy * 5.) + rng.gen_range(-3.0..10.0);

        commands.spawn((
            Self {
                time_passed: rng.gen_range(0.0..0.2),
                energy,
            },
            Radius(diameter / 2.),
            Transform::from_translation(Vec3::new(translation.x, translation.y, -1.)),
            Sprite {
                image: asset_server.load("nodule.png"),
                color: Color::Srgba(Srgba::rgb(1., 1., 0.)),
                custom_size: Some(Vec2::splat(diameter)),
                ..default()
            },
        ));
    }

    pub fn update(
        time: Res<Time>,
        mut suns: Query<(Entity, &mut Sun, &mut Transform)>,
        mut commands: Commands,
    ) {
        const MOVE_TIME: f32 = 0.2;

        suns.iter_mut()
            .for_each(|(entity, mut sun, mut transform)| {
                sun.time_passed += time.delta_secs();

                while sun.time_passed >= MOVE_TIME {
                    sun.time_passed -= MOVE_TIME;

                    transform.translation.y -= 5.;
                }

                if transform.translation.y < -1000. {
                    commands.entity(entity).despawn();
                }
            });
    }
}
