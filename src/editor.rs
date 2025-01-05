use crate::prelude::*;

pub mod prelude {
    pub use super::Editor;
}

#[init]
#[derive(Resource, Default)]
pub struct Editor {}
init_resource!(Editor);

impl Editor {
    fn test_spawn() {}

    pub fn ui(mut contexts: EguiContexts, mut editor: ResMut<Editor>) {
        egui::SidePanel::right("Editor").show(contexts.ctx_mut(), |ui| {
            let blah = [("Plants", [""])];
        });
    }
}
