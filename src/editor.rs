use crate::{prelude::*, TerrainPoint};

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
    /// The TypeId is so we can only show the paste button when the TypeId is equal to the expected type.
    pub copied: (fn(&mut Commands, Vec2), TypeId),
}

impl Default for Editor {
    fn default() -> Self {
        Self {
            category_open: 0,
            copied: (|_, _| {}, TypeId::of::<()>()),
        }
    }
}

impl Editor {
    fn test_spawn(commands: &mut Commands) {}

    // I want a long list of buttons. When you click on one it selects it. From there you can click and it will spawn one wherever you click.
    pub fn ui(mut contexts: EguiContexts, mut editor: ResMut<Editor>) {
        egui::SidePanel::right("Editor").show(contexts.ctx_mut(), |ui| {
            let categories: &[(&str, &[(&str, TypeId, fn(&mut Commands, Vec2))])] = &[(
                "Misc",
                &[(
                    "Terrain",
                    TypeId::of::<TerrainPoint>(),
                    |commands, translation| {},
                )],
            )];

            categories.iter().for_each(|(category, _)| {
                if ui.button(*category).clicked() {

                }

                ui.add_space(10.);
            });
            ui.separator();

            let (category, entries) = categories[editor.category_open];

            entries.iter().for_each(|(name, type_id, spawner)| {
                ui.add_space(10.);
                if ui.button(*name).clicked() {
                    editor.copied = (*spawner, *type_id);
                }
            });
        });
    }
}
