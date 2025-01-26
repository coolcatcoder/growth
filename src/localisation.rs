use crate::prelude::*;

pub mod prelude {
    pub use super::Language;
}

#[derive(Resource, Clone)]
pub enum Language {
    English,
    // Idea: AutomaticTranslation(something idk),
}

#[derive(Component)]
struct LanguageButton(Language);

#[system(Update::LoadMenus)]
fn load(
    mut menu: MenuReader,
    mut clear_colour: ResMut<ClearColor>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
) {
    if !menu.switched_to(Menu::LanguageSelection) {
        return;
    }

    clear_colour.0 = Srgba::gray(0.1).into();

    commands.spawn((Camera2d { ..default() }, FromMenu));

    let languages = commands.spawn((
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

    struct Add<'a, 'b>(EntityCommands<'a>, &'b AssetServer);

    impl<'a, 'b> Add<'a, 'b> {
        fn language(&mut self, language_name: &str, language: Language) {
            self.0.with_child((
                Text::new(language_name),
                TextColor(WHITE.into()),
                TextFont {
                    font: self.1.load("fonts/AzeretMono.ttf"),
                    font_size: 100.,
                    ..default()
                },
                Button,
                FromMenu,
                BackgroundColor(Srgba::gray(0.3).into()),
                LanguageButton(language),
            ));
        }
    }

    let mut add = Add(languages, &asset_server);

    add.language("English", Language::English);

    info!("Loaded language menu.");
}

#[system(Update)]
fn menu(
    mut language_buttons: Query<(&LanguageButton, &Interaction, &mut BackgroundColor)>,
    mut menu: EventWriter<Menu>,
    mut commands: Commands,
) {
    language_buttons
        .iter_mut()
        .for_each(
            |(language, interaction, mut background_colour)| match *interaction {
                Interaction::None => {
                    background_colour.0 = Srgba::gray(0.3).into();
                }
                Interaction::Hovered => {
                    background_colour.0 = Srgba::gray(0.2).into();
                }
                Interaction::Pressed => {
                    commands.insert_resource(language.0.clone());
                    menu.send(Menu::ProfileName);
                }
            },
        );
}
