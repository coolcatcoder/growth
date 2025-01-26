use crate::prelude::*;

pub mod prelude {
    pub use super::Editor;
}

/// Stores anything needed for the general editor, which is the right panel.
#[init]
#[derive(Resource)]
pub struct Editor {
    category_open: usize,
    /// Used for copying and pasting.
    /// This also is set when you select an item from the right panel.
    /// The &str is so we can only show the paste button when the &str is equal to the expected value.
    pub copied: (fn(&mut Commands, &AssetServer, Vec2), &'static str),

    pub selected_entities: Vec<Entity>,
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            category_open: 0,
            copied: (|_, _, _| {}, ""),
            selected_entities: vec![],
        }
    }
}

impl Editor {
    // I want a long list of buttons. When you click on one it selects it. From there you can click and it will spawn one wherever you click.
    pub fn ui(mut contexts: EguiContexts, mut editor: ResMut<Editor>) {
        egui::SidePanel::right("Editor").show(contexts.ctx_mut(), |ui| {
            let categories: &[(&str, &[(&str, fn(&mut Commands, &AssetServer, Vec2))])] = &[(
                "Plants",
                &[
                    ("Test Plant", |commands, asset_server, translation| {
                        commands.spawn((
                            Transform::from_translation(Vec3::new(
                                translation.x,
                                translation.y,
                                1.,
                            )),
                            PlantCell::new(
                                Arc::new(vec![PlantCellTemplate {
                                    grow_chance_every: Duration::from_secs(2),

                                    grow_chance: 0.5,

                                    grow_chance_change_after_success: -0.5,
                                    grow_chance_change_after_failure: 0.,

                                    grow_chance_clamp: 0.0..1.0,

                                    grow_into: vec![0],
                                }]),
                                0,
                            ),
                            Sprite {
                                image: asset_server.load("nodule.png"),
                                ..default()
                            },
                        ));
                    }),
                    ("Seed?", |commands, asset_server, translation| {
                        for x in -30..=30 {
                            for y in -10..=10 {
                                let translation =
                                    translation + Vec2::new(30. * x as f32, 30. * y as f32);
                                commands.spawn((
                                    Sprite {
                                        image: asset_server.load("nodule.png"),
                                        ..default()
                                    },
                                    Transform::from_translation(Vec3::new(
                                        translation.x,
                                        translation.y,
                                        1.,
                                    )),
                                    Verlet::from_translation(translation),
                                    Radius(15.),
                                    Gravity,
                                ));
                            }
                        }
                    }),
                ],
            )];

            categories.iter().for_each(|(category, _)| {
                if ui.button(*category).clicked() {}

                ui.add_space(10.);
            });
            ui.separator();

            let (_, entries) = categories[editor.category_open];

            entries.iter().for_each(|(name, spawner)| {
                ui.add_space(10.);
                let button = ui.button(*name);
                if button.clicked() {
                    editor.copied = (*spawner, *name);
                }
                if *name == editor.copied.1 {
                    button.highlight();
                }
            });
        });
    }

    // Creates whatever you have selected.
    pub fn create(
        editor: Res<Editor>,
        actions: Res<ActionState<Action>>,
        translation: Res<CursorWorldTranslation>,
        mut commands: Commands,
        asset_server: Res<AssetServer>,
    ) {
        if actions.just_pressed(&Action::EditorCreate) {
            (editor.copied.0)(&mut commands, &asset_server, translation.0);
        }
    }
}
