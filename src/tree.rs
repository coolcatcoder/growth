pub use crate::prelude::*;

pub mod prelude {
    pub use super::{Leaf, Tree};
}

//MARK: Tree
#[derive(Component)]
pub struct Tree {
    timer: Timer,
    max_height: u16,
    height: u16,
}

impl Grower for Tree {
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
            if self.height == self.max_height {
                return;
            }

            if self.height > 10 && self.height % 3 == 0 {
                Leaf::create(
                    10.0_f32
                        .lerp(1.0, f32::from(self.height) / f32::from(self.max_height))
                        .round() as u8,
                    1.,
                    transform.translation.xy(),
                    commands,
                    asset_server,
                );

                Leaf::create(
                    10.0_f32
                        .lerp(1.0, f32::from(self.height) / f32::from(self.max_height))
                        .round() as u8,
                    -1.,
                    transform.translation.xy(),
                    commands,
                    asset_server,
                );
            }

            Self::create(
                self.height + 1,
                self.max_height,
                transform.translation.xy(),
                commands,
                asset_server,
            );
        }
    }
}

impl Tree {
    pub fn create(
        height: u16,
        max_height: u16,
        mut translation: Vec2,
        commands: &mut Commands,
        asset_server: &AssetServer,
    ) {
        const SIZE: Vec2 = Vec2::new(50., 15.);
        const GROW_TIME: f32 = 0.1;

        const MIN_TRANSLATION: Vec2 = Vec2::new(-5., 14.);
        const MAX_TRANSLATION: Vec2 = Vec2::new(5., 17.);

        // Maybe slow? Consider caching.
        let mut rng = thread_rng();

        translation.x += rng.gen_range(MIN_TRANSLATION.x..MAX_TRANSLATION.x);
        translation.y += rng.gen_range(MIN_TRANSLATION.y..MAX_TRANSLATION.y);

        commands.spawn((
            Self {
                timer: Timer::from_seconds(GROW_TIME, TimerMode::Once),
                height,
                max_height,
            },
            SpriteBundle {
                texture: asset_server.load("nodule.png"),
                transform: Transform::from_translation(Vec3::new(translation.x, translation.y, 0.)),
                sprite: Sprite {
                    color: Color::Srgba(Srgba::rgb(1.0, 0.0, 0.0)),
                    custom_size: Some(SIZE),
                    ..default()
                },
                ..default()
            },
        ));
    }
}

//MARK: Leaf
#[derive(Component)]
pub struct Leaf {
    timer: Timer,
    growth_remaining: u8,
    direction: f32,
}

impl Grower for Leaf {
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
            if self.growth_remaining == 0 {
                return;
            }

            Self::create(
                self.growth_remaining - 1,
                self.direction,
                transform.translation.xy(),
                commands,
                asset_server,
            );
        }
    }
}

impl Leaf {
    fn create(
        growth_remaining: u8,
        direction: f32,
        mut translation: Vec2,
        commands: &mut Commands,
        asset_server: &AssetServer,
    ) {
        const SIZE: Vec2 = Vec2::new(30., 15.);
        const GROW_TIME: f32 = 0.5;

        const MIN_TRANSLATION: Vec2 = Vec2::new(30., -5.);
        const MAX_TRANSLATION: Vec2 = Vec2::new(40., 0.);

        // Maybe slow? Consider caching.
        let mut rng = thread_rng();

        translation.x += rng.gen_range(MIN_TRANSLATION.x..MAX_TRANSLATION.x) * direction;
        translation.y += rng.gen_range(MIN_TRANSLATION.y..MAX_TRANSLATION.y);

        commands.spawn((
            Leaf {
                timer: Timer::from_seconds(GROW_TIME, TimerMode::Once),
                growth_remaining,
                direction,
            },
            SpriteBundle {
                texture: asset_server.load("nodule.png"),
                transform: Transform::from_translation(Vec3::new(translation.x, translation.y, 0.)),
                sprite: Sprite {
                    color: Color::Srgba(Srgba::rgb(0.0, 0.0, 1.0)),
                    custom_size: Some(SIZE),
                    ..default()
                },
                ..default()
            },
        ));
    }
}
