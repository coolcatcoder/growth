use crate::prelude::*;

#[system(Update::LoadMenus)]
fn main_menu(mut ui: Ui) {
    load!(ui, Menu::Main);

    label!(ui, "Game Title!");

    button!(ui, "Debug Play!", |mut menu: EventWriter<Menu>| {
        menu.send(Menu::InGame);
    });
}
