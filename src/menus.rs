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

/*
menu! {
    Parameters {

    }

    Config {
        menu: Menu::ProfileName,
        background: Srgba::gray(0.1).into(),
        root: Node {},
    }

    Button
}
*/

macro_rules! menu_some {
    (
        Load
        Config($variable:ident)
        With($root:expr, $asset_server:expr, $index:expr)
    ) => {
        let Some($variable) = $variable else {
            return;
        };
    };

    ($($_:tt)*) => {};
}

macro_rules! menu_label {
    (
        Load
        Config($text:expr)
        With($root:expr, $asset_server:expr, $index:expr)
    ) => {
        $root.with_child((label($asset_server), Text::new($text), Label, FromMenu));
    };

    ($($_:tt)*) => {};
}

macro_rules! menu_text_input {
    (
        Load
        Config($config:expr)
        With($root:expr, $asset_server:expr, $index:expr)
    ) => {
        $root.with_child((text_input($asset_server), Label, FromMenu, $config));
    };

    ($($_:tt)*) => {};
}

macro_rules! menu_button {
    (
        Load
        Config(
            $text:expr,
            [$self_ident:ident = self]
            |$($($parameter_idents:ident)+: $parameter_type:ty),*| $function:block
        )
        With($root:expr, $asset_server:expr, $index:expr)
    ) => {
        bevy_registration::paste!{
            $root.with_child((
                button($asset_server),
                Text::new($text),
                Button,
                FromMenu,
                WidgetId($text.into()),
                BackgroundColor(button_background_colour(&Interaction::None)),
                [<Widget $index>]
            ));
        }
    };

    (
        Module
        Config(
            $text:expr,
            [$self_ident:ident = self]
            |$($($parameter_idents:ident)+: $parameter_type:ty),*| $function:block
        )
        With($index:expr)
    ) => {
        bevy_registration::paste! {
            #[derive(Component)]
            struct [<Widget $index>];

            #[system(Update)]
            fn [<widget_ $index>](
                $self_ident: Option<Single<(&Interaction, &mut BackgroundColor), With<[<Widget $index>]>>>,
                $(
                    $($parameter_idents)+: $parameter_type
                ),*
            ) {
                let Some(mut $self_ident) = $self_ident else {
                    return;
                };

                $self_ident.1.0 = button_background_colour($self_ident.0);

                $function
            }
        }
    };

    //($($_:tt)*) => {};
}

macro_rules! menu {
    (
        [Menu::$menu:ident]
        $(
            (
                $(
                    $parameter_ident:ident: $parameter_type:ty
                ),*$(,)?
            )
        )?

        $(
            $widget:ident($($config:tt)*)
        )*

    ) => {
        bevy_registration::paste! {
            mod [<menu_ $menu:snake>] {
                pub use super::*;

                $(
                    [<menu_ $widget:snake>]!(Module Config($($config)*) With(${index()}));
                )*

                #[system(Update::LoadMenus)]
                fn [<$menu:snake _load>](
                    mut menu: MenuReader,
                    mut clear_colour: ResMut<ClearColor>,
                    asset_server: Res<AssetServer>,
                    mut commands: Commands,
                    $(
                        $(
                            $parameter_ident: $parameter_type,
                        )*
                    )?
                ) {
                    if !menu.switched_to(Menu::$menu) {
                        return;
                    }

                    clear_colour.0 = background();

                    commands.spawn((Camera2d { ..default() }, FromMenu));

                    let mut root = commands.spawn((
                        root(),
                        FromMenu,
                    ));

                    $(
                        [<menu_ $widget:snake>]!(Load Config($($config)*) With(root, &asset_server, ${index()}));
                    )*
                }
            }
        }
    };
}

// menu! {
//     [Menu::ProfileName]
//     (
//         language: Option<Res<Language>>,
//     )

//     Some(language)

//     Label({
//         match *language {
//             Language::English => "Profile Name",
//         }
//     })

//     TextInput({TextInput::new(10)})

//     Button(
//         {
//             match *language {
//                 Language::English => "Accept",
//             }
//         },
//         [button = self]
//         |text_input: Option<Single<&TextInput>>, mut menu: EventWriter<Menu>| {
//             let Some(text_input) = text_input else {
//                 return;
//             };

//             if text_input.text.chars().count() == 0 {
//                 button.1.0 = button_background_colour(&Interaction::None);
//             } else {
//                 if matches!(button.0, Interaction::Pressed) {
//                     info!("foo");
//                     menu.send(Menu::SelectProfile);
//                 }
//             }
//         }
//     )
// }
