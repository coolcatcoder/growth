use crate::prelude::*;

pub mod prelude {
    pub use super::ActiveProfile;
}

/// A profile. This is a component for easier saving and loading.
#[derive(Component, SaveAndLoad)]
struct Profile {
    name: String,
}

/// Used to identify which profile is the active one.
#[derive(Component)]
pub struct ActiveProfile;

fn ui_background() -> Color {
    Srgba::gray(0.1).into()
}

fn ui_root() -> impl Bundle {
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

fn ui_label(ui: &mut UiLoaded) -> impl Bundle {
    (
        TextColor(WHITE.into()),
        TextFont {
            font: ui.asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
    )
}

fn ui_text_input(ui: &mut UiLoaded) -> impl Bundle {
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

fn ui_button(ui: &mut UiLoaded) -> impl Bundle {
    (
        TextColor(WHITE.into()),
        TextFont {
            font: ui.asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
    )
}

fn ui_button_background_colour(interaction: &Interaction) -> Color {
    Srgba::gray(match interaction {
        Interaction::None => 0.3,
        Interaction::Hovered => 0.2,
        Interaction::Pressed => 0.1,
    })
    .into()
}

#[system(Update::LoadMenus)]
fn profile_name_load(mut ui: Ui, language: Option<Res<Language>>) {
    load!(ui, Menu::ProfileName);

    some!(language);

    label!(
        ui,
        match *language {
            Language::English => "Profile Name",
        }
    );

    text_input!(ui, 10);

    button!(
        ui,
        match *language {
            Language::English => "Accept",
        },
        |text_input: Option<Single<&TextInput>>,
         mut menu: EventWriter<Menu>,
         mut commands: Commands,
         mut save: Save| {
            let Some(text_input) = text_input else {
                return;
            };
            if text_input.text.chars().count() == 0 {
                return;
            };

            commands.spawn((
                Profile {
                    name: text_input.text.clone(),
                },
                SaveConfig {
                    path: String::from("./profiles"),
                },
                ActiveProfile,
            ));
            menu.send(Menu::Main);
            save.path("./profiles");
        }
    );
}

/// Either some profile's button, or the add new profile button.
#[derive(Component)]
struct ProfileButton(Option<Entity>);

#[system(Update::LoadMenus)]
fn selection_menu_load(
    mut menu: MenuReader,
    mut clear_colour: ResMut<ClearColor>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut load: Load,
) {
    if !menu.switched_to(Menu::SelectProfile) {
        return;
    }

    load.path("./profiles");

    clear_colour.0 = Srgba::gray(0.1).into();

    commands.spawn((Camera2d { ..default() }, FromMenu));

    let mut profiles = commands.spawn((
        Node {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::SpaceAround,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            ..default()
        },
        FromMenu,
        Root,
    ));

    profiles.with_child((
        Text::new("+"),
        TextColor(WHITE.into()),
        TextFont {
            font: asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
        Button,
        FromMenu,
        BackgroundColor(Srgba::gray(0.3).into()),
        ProfileButton(None),
    ));

    info!("Loaded profile menu.");
}

#[system(Update::LoadMenus)]
fn profile_buttons_load(
    profiles: Query<&Profile>,
    root: Option<Single<Entity, With<Root>>>,
    mut loaded: EventReader<LoadFinish>,
    mut menu: MenuReader,
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    if !menu.is(Menu::SelectProfile) {
        return;
    }

    // This will fail for 1 frame. I don't think that loses us any profiles? I hope.
    // Keeping the error, to remind myself that this isn't a great solution.
    some_err!(root);
    let mut root = commands.entity(*root);

    loaded.read().for_each(|loaded| {
        assert_return!(loaded.is_component::<Profile>());

        let profile = ok_err!(profiles.get(loaded.entity));

        info!("Loaded profile.");

        root.with_child((
            Text::new(&profile.name),
            TextColor(WHITE.into()),
            TextFont {
                font: asset_server.load("fonts/AzeretMono.ttf"),
                font_size: 100.,
                ..default()
            },
            Button,
            FromMenu,
            BackgroundColor(Srgba::gray(0.3).into()),
            ProfileButton(Some(loaded.entity)),
        ));
    });
}

#[system(Update)]
fn selection_menu(
    mut profile_buttons: Query<(&ProfileButton, &Interaction, &mut BackgroundColor)>,
    mut menu: EventWriter<Menu>,
    mut commands: Commands,
) {
    profile_buttons
        .iter_mut()
        .for_each(
            |(profile, interaction, mut background_colour)| match *interaction {
                Interaction::None => {
                    background_colour.0 = Srgba::gray(0.3).into();
                }
                Interaction::Hovered => {
                    background_colour.0 = Srgba::gray(0.2).into();
                }
                Interaction::Pressed => {
                    if let Some(entity) = profile.0 {
                        commands.entity(entity).insert(ActiveProfile);
                        menu.send(Menu::Main);
                    } else {
                        menu.send(Menu::LanguageSelection);
                    }
                }
            },
        );
}
