use crate::prelude::*;

pub mod prelude {
    pub use super::{FromMenu, Menu, MenuReader};
}

#[init]
#[derive(Event, Default, PartialEq, Eq, Clone)]
pub enum Menu {
    #[default]
    SelectProfile,
    LanguageSelection,
    ProfileName,
    Main,
    InGame,
}

/// Removes some of the boilerplate from reading menus.
#[derive(SystemParam)]
pub struct MenuReader<'w, 's> {
    event_reader: EventReader<'w, 's, Menu>,
    menu: Local<'s, Menu>,
}

impl<'w, 's> MenuReader<'w, 's> {
    /// Has the menu just switched to some menu. (This includes the previous menu.)
    pub fn switched_to(&mut self, menu: Menu) -> bool {
        let Some(switched_to) = self.event_reader.read().last() else {
            return false;
        };
        *self.menu = switched_to.clone();

        menu == *switched_to
    }

    /// Has the menu switched to any menu. (This includes the previous menu.)
    pub fn switched_to_any(&mut self) -> bool {
        let switched_to_any = self.event_reader.len() != 0;
        if let Some(switched_to) = self.event_reader.read().last() {
            *self.menu = switched_to.clone();
        }
        switched_to_any
    }

    // Is the menu a menu?
    pub fn is(&mut self, menu: Menu) -> bool {
        if let Some(switched_to) = self.event_reader.read().last() {
            *self.menu = switched_to.clone();
        }

        menu == *self.menu
    }
}

#[system(Startup)]
fn startup_menu(mut menu: EventWriter<Menu>) {
    menu.send(Menu::SelectProfile);
}

#[derive(Component)]
pub struct FromMenu;

/// Unloads all entities from menus when a new menu is requested.
#[system(Update::UnloadMenus)]
fn unload_menus(
    from_menu: Query<Entity, With<FromMenu>>,
    mut menu: MenuReader,
    mut commands: Commands,
) {
    if !menu.switched_to_any() {
        return;
    }

    from_menu
        .iter()
        .for_each(|entity| commands.entity(entity).despawn());

    info!("Despawned all menu entities.");
}
