use crate::prelude::*;

mod text_edit;

pub mod prelude {
    pub use super::{text_edit::prelude::*, Ui, UiLoaded};
    pub use crate::{button, label, load, text_input};
}

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

#[macro_export]
macro_rules! load {
    ($ui:ident, $menu:expr) => {
        let mut $ui = {
            if !$ui.menu.switched_to($menu) {
                return;
            }

            $ui.clear_colour.0 = background();

            $ui.commands.spawn((Camera2d { ..default() }, FromMenu));

            let root = $ui.commands.spawn((root(), FromMenu));

            UiLoaded {
                asset_server: $ui.asset_server,
                root,
            }
        };
    };
}

#[macro_export]
macro_rules! label {
    ($ui:ident, $text:expr) => {{
        let bundle = (label(&mut $ui), Text::new($text), Label, FromMenu);
        $ui.root.with_child(bundle)
    }};
}

#[macro_export]
macro_rules! text_input {
    ($ui:ident, $width_minimum:expr) => {{
        let bundle = (
            text_input(&mut $ui),
            TextInput::new($width_minimum),
            Label,
            FromMenu,
        );
        $ui.root.with_child(bundle)
    }};
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

            button.1.0 = button_background_colour(button.0);

            if matches!(button.0, Interaction::Pressed) {
                $function
            }
        }

        let bundle = (button(&mut $ui), Text::new($text), Button, FromMenu, ThisButton);
        $ui.root.with_child(bundle)
    }};
}
