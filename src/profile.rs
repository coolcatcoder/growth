use crate::prelude::*;

/// The current user's profile.
/// This should store, name, settings, language, etc.
#[derive(Component, SaveAndLoad)]
pub struct Profile {
    name: String,
}

#[derive(Component)]
struct Active;

fn background() -> Color {
    Srgba::gray(0.1).into()
}

fn root() -> impl Bundle {
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

fn label(ui: &mut UiLoaded) -> impl Bundle {
    (
        TextColor(WHITE.into()),
        TextFont {
            font: ui.asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
    )
}

fn text_input(ui: &mut UiLoaded) -> impl Bundle {
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

fn button(ui: &mut UiLoaded) -> impl Bundle {
    (
        TextColor(WHITE.into()),
        TextFont {
            font: ui.asset_server.load("fonts/AzeretMono.ttf"),
            font_size: 100.,
            ..default()
        },
    )
}

fn button_background_colour(interaction: &Interaction) -> Color {
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
                Active,
                SaveConfig {
                    path: String::from("./profiles"),
                }
            ));
            menu.send(Menu::Main);
            save.path("./profiles");
        }
    );
}

// TODO: Replace () with a profile handle or something like that.
#[derive(Component)]
struct ProfileButton(Option<()>);

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

#[system(Update)]
fn selection_menu(
    mut profile_buttons: Query<(&ProfileButton, &Interaction, &mut BackgroundColor)>,
    mut menu: EventWriter<Menu>,
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
                    if let Some(_) = profile.0 {
                        todo!();
                    } else {
                        menu.send(Menu::LanguageSelection);
                    }
                }
            },
        );
}