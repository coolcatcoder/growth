use crate::prelude::*;

fn ui_background() -> Color {
    Srgba::rgb(0.5, 0.2, 0.5).into()
}

#[system(Update::LoadMenus)]
fn game(mut ui: Ui) {
    load!(ui, Menu::InGame);

    
}

#[system(Update)]
fn setup(mut menu: MenuReader, mut load: Load) {
    assert_return!(menu.switched_to(Menu::InGame));

    info!("Loading debug game.");

    // For testing purposes.
    return;

    load.path("./map");
}