use crate::prelude::*;

mod text_edit;

pub mod prelude {
    pub use super::{
        text_edit::prelude::*, ui_background, ui_button, ui_button_background_colour, ui_label,
        ui_root, ui_text_input, Root, Ui, UiLoaded,
    };
    pub use crate::{button, label, load, text_input};
}

/// The base node.
#[derive(Component)]
pub struct Root;

#[derive(SystemParam)]
pub struct Ui<'w, 's> {
    pub menu: MenuReader<'w, 's>,
    pub clear_colour: ResMut<'w, ClearColor>,
    pub asset_server: Res<'w, AssetServer>,
    pub commands: Commands<'w, 's>,
}

pub struct UiLoaded<'a, 'r> {
    pub asset_server: Res<'a, AssetServer>,
    pub root: EntityCommands<'r>,
}

pub fn ui_background() -> Color {
    Srgba::gray(0.1).into()
}

pub fn ui_root() -> impl Bundle {
    Node {
        display: Display::Flex,
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::SpaceAround,
        width: Val::Percent(100.),
        height: Val::Percent(100.),
        ..default()
    }
}

#[macro_export]
macro_rules! load {
    ($ui:ident, $menu:expr) => {
        let mut $ui = {
            if !$ui.menu.switched_to($menu) {
                return;
            }

            $ui.clear_colour.0 = ui_background();

            $ui.commands.spawn((Camera2d { ..default() }, FromMenu));

            let root = $ui.commands.spawn((ui_root(), FromMenu, Root));

            UiLoaded {
                asset_server: $ui.asset_server,
                root,
            }
        };
    };
}

pub fn ui_label(ui: &mut UiLoaded) -> impl Bundle {
    (
        TextColor(WHITE.into()),
        TextFont {
            font: ui.asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
    )
}

#[macro_export]
macro_rules! label {
    ($ui:ident, $text:expr) => {{
        let bundle = (ui_label(&mut $ui), Text::new($text), Label, FromMenu);
        $ui.root.with_child(bundle)
    }};
}

pub fn ui_text_input(ui: &mut UiLoaded) -> impl Bundle {
    (
        TextColor(WHITE.into()),
        TextFont {
            font: ui.asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
        BackgroundColor(Srgba::gray(0.3).into()),
    )
}

#[macro_export]
macro_rules! text_input {
    ($ui:ident, $width_minimum:expr) => {{
        let bundle = (
            ui_text_input(&mut $ui),
            TextInput::new($width_minimum),
            Label,
            FromMenu,
        );
        $ui.root.with_child(bundle)
    }};
}

pub fn ui_button(ui: &mut UiLoaded) -> impl Bundle {
    (
        TextColor(WHITE.into()),
        TextFont {
            font: ui.asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
    )
}

pub fn ui_button_background_colour(interaction: &Interaction) -> Color {
    Srgba::gray(match interaction {
        Interaction::None => 0.3,
        Interaction::Hovered => 0.2,
        Interaction::Pressed => 0.1,
    })
    .into()
}

#[macro_export]
macro_rules! button {
    ($ui:ident, $text:expr, |$($($parameter_idents:ident)+: $parameter_type:ty),*| $function:block) => {{
        // Unique component for this button.
        // This does mean we have 1 archetype per entity, which is nasty.
        #[derive(Component)]
        struct ThisButton;

        #[system(Update)]
        fn on_click(
            button: Option<Single<(&Interaction, &mut BackgroundColor), With<ThisButton>>>,
            $(
                $($parameter_idents)+: $parameter_type
            ),*
        ) {
            let Some(mut button) = button else {
                return;
            };

            button.1.0 = ui_button_background_colour(button.0);

            if matches!(button.0, Interaction::Pressed) {
                $function
            }
        }

        let bundle = (ui_button(&mut $ui), Text::new($text), Button, FromMenu, ThisButton);
        $ui.root.with_child(bundle)
    }};
}
