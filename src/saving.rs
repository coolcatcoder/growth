use serde::de::DeserializeOwned;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{
        DeserialiseEntity, Load, LoadFinish, LoadStart, Save, SaveAndLoad, SaveConfig,
        SerialiseEntity, SerialisedEntity, StartSave,
    };
}

/// Anything saved will be relative to this path.
// Not a path, due to Paths not being allowed as constants.
const SAVE_PATH: &str = "./assets/saves/";
/// Same as above, but relative to ./assets instead.
const SAVE_PATH_RELATIVE_TO_ASSETS: &str = "./saves";

/// Called just before everything should save.
/// This will make sure that everything is cleared out before sending the Save event.
/// The string is the path relative to SAVE_PATH for safety.
#[derive(Event)]
pub struct StartSave(pub String);

impl StartSave {
    /// Clears EntityToIndex and then sends of the save event.
    pub fn prepare(
        mut start_save: EventReader<Self>,
        mut save: EventWriter<Save>,
        mut serialise_entity: ResMut<SerialiseEntity>,
    ) {
        start_save.read().for_each(|start_save| {
            serialise_entity.0.clear();
            serialise_entity.1 = 0;

            //TODO: Backup everything first.

            let path = Path::new(SAVE_PATH).join(&start_save.0);

            let exists = ok_or_error_and_return!(
                fs::exists(&path),
                "Tried to check if a folder existed and got this error:"
            );

            if exists {
                ok_or_error_and_return!(
                    fs::remove_dir_all(&path),
                    "Tried to remove a folder and got this error:"
                );
            }

            ok_or_error_and_return!(
                fs::create_dir(&path),
                "Tried to create a folder and got this error:"
            );

            save.send(Save(start_save.0.clone()));
        });
    }
}

/// An event that is called whenever everything should save.
#[derive(Event)]
pub struct Save(pub String);

impl Save {
    pub fn to_serialised_entity<T: Serialize>(
        value: &T,
        serialised_entity: SerialisedEntity,
        path: impl AsRef<Path>,
        file_name: &str,
    ) {
        let folder_path = Path::new(SAVE_PATH)
            .join(path)
            .join(format!("./{}", serialised_entity.0));

        let exists = ok_or_error_and_return!(
            fs::exists(&folder_path),
            "Tried to check if a folder existed and got this error:"
        );
        if !exists {
            ok_or_error_and_return!(
                fs::create_dir(&folder_path),
                "Tried to create a folder. Got this error:"
            );
        }

        let file_path = folder_path.join(format!("./component.{}.json", file_name));

        let exists = ok_or_error_and_return!(
            fs::exists(&file_path),
            "Tried to check if a file existed and got this error:"
        );
        if exists {
            ok_or_error_and_return!(
                fs::remove_file(&file_path),
                "Tried to remove a file. Got error:"
            );
        }

        let file = ok_or_error_and_return!(
            File::create(&file_path),
            "Tried to create a file. Got this error:"
        );
        ok_or_error_and_return!(
            serde_json::to_writer_pretty(file, value),
            "Tried to save a file. During serialisation got this error:"
        );
    }
}

/// Indicates to start loading from that path relative to SAVE_PATH.
#[derive(Event)]
pub struct LoadStart(pub String);

impl LoadStart {
    pub fn prepare(
        mut start_load: EventReader<LoadStart>,
        mut load: EventWriter<Load>,
        mut deserialise_entity: ResMut<DeserialiseEntity>,
        loaded_entities: Query<(Entity, &SaveConfig)>,
        mut commands: Commands,
    ) {
        start_load.read().for_each(|start_load| {
            // Deload any entities in the path we want to load.
            loaded_entities.iter().for_each(|(entity, save_config)| {
                if save_config.path == start_load.0 {
                    commands.entity(entity).despawn();
                }
            });

            // Clear this, to assure us that no nonsense shall occur. There is more than 1 save path, so this is required.
            deserialise_entity.0.clear();

            load.send(Load(start_load.0.clone()));
        });
    }
}

/// Tells everything to load.
#[derive(Event)]
pub struct Load(pub String);

#[derive(Event)]
pub struct LoadFinish(pub Entity);

/// Because Entity is opaque, we must convert it to something that will never change.
#[derive(Resource, Default)]
pub struct SerialiseEntity(HashMap<Entity, u32>, u32);

impl SerialiseEntity {
    pub fn convert(&mut self, entity: Entity) -> SerialisedEntity {
        // Get the index if it exists, else create the index and return it.
        if let Some(index) = self.0.get(&entity) {
            SerialisedEntity(*index)
        } else {
            let index = self.1;
            self.1 += 1;
            self.0.insert(entity, index);
            SerialisedEntity(index)
        }
    }
}

/// The inverse of the previous.
/// Converts indices to entities.
// TODO: Does this need to be reset on load? Do we need to despawn all previously loaded entities? Yes. Why?
#[derive(Resource, Default)]
pub struct DeserialiseEntity(HashMap<u32, Entity>);

impl DeserialiseEntity {
    // Infallible, because we create the entity if it doesn't exist.
    // Anything that needs to create an entity from an index must use this function.
    pub fn convert(
        &mut self,
        serialised_entity: SerialisedEntity,
        commands: &mut Commands,
    ) -> Entity {
        // Get the index if it exists, else create the index and return it.
        if let Some(entity) = self.0.get(&serialised_entity.0) {
            *entity
        } else {
            let entity = commands.spawn_empty().id();
            self.0.insert(serialised_entity.0, entity);
            entity
        }
    }
}

/// Entity is opaque and ethereal. We as such serialise and deserialise from u32.
/// Each entity gets assigned a number starting from 0 and ascending. If you want to reference that entity, you use that number.
#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct SerialisedEntity(pub u32);

/// New idea. Save config.
/// We still have decentralised saving, but for per entity information, it can be gotten from here.
/// This may include:
/// - path to save at
/// - TODO: Any more?
#[derive(Component, SaveAndLoad)]
pub struct SaveConfig {
    pub path: String,
}

#[derive(Default)]
pub enum LoadingStage<T: Asset> {
    #[default]
    WaitingForEvent,
    GotFolderHandle(Handle<LoadedFolder>),
    GotComponentHandles(Vec<Handle<T>>),
}

pub trait SaveAndLoad: Sized + Component {
    type Serialised: Serialize + DeserializeOwned + Asset;

    const STRUCT_IDENT_LOWERCASE: &str;
    const FILE_EXTENSION: &str;

    fn serialise(&self, serialise_entity: &mut SerialiseEntity) -> Self::Serialised;
    fn deserialise(
        serialised: &Self::Serialised,
        deserialise_entity: &mut DeserialiseEntity,
        commands: &mut Commands,
    ) -> Self;

    fn save(
        values: Query<(Entity, &SaveConfig, &Self)>,
        mut save: EventReader<Save>,
        mut serialise_entity: ResMut<SerialiseEntity>,
    ) {
        save.read().for_each(|save| {
            values.iter().for_each(|(entity, save_config, value)| {
                // find the save configs whose paths match the path you want to save
                // get or create entity folder at the path
                // create component file in it

                if save_config.path != save.0 {
                    return;
                }

                let entity = serialise_entity.convert(entity);

                let serialised = value.serialise(&mut serialise_entity);

                // Each entity should have only 1 of each component, so the file is unique.
                Save::to_serialised_entity(
                    &serialised,
                    entity,
                    &save_config.path,
                    Self::STRUCT_IDENT_LOWERCASE,
                );
            });
        });
    }

    fn load(
        mut commands: Commands,
        mut deserialise_entity: ResMut<DeserialiseEntity>,

        mut load: EventReader<Load>,
        mut load_finish: EventWriter<LoadFinish>,

        asset_server: Res<AssetServer>,
        serialised: Res<Assets<Self::Serialised>>,
        folders: Res<Assets<LoadedFolder>>,

        mut loading_stage: Local<LoadingStage<Self::Serialised>>,
    ) {
        match &mut *loading_stage {
            LoadingStage::WaitingForEvent => {
                load.read().for_each(|load| {
                    // If multiple systems load the same folder, they will all be given the same handle.
                    // This therefore does not waste effort loading the folder multiple times.
                    *loading_stage = LoadingStage::GotFolderHandle(
                        asset_server
                            .load_folder(Path::new(SAVE_PATH_RELATIVE_TO_ASSETS).join(&load.0)),
                    );

                    info!("Loading folder!");
                });
            }
            LoadingStage::GotFolderHandle(folder_handle) => {
                let folder = some_or_return!(folders.get(folder_handle));

                let mut component_handles = vec![];

                folder.handles.iter().for_each(|handle| {
                    if let Ok(handle) = handle.clone().try_typed::<Self::Serialised>() {
                        component_handles.push(handle);
                    }
                });

                *loading_stage = LoadingStage::GotComponentHandles(component_handles);
            }
            LoadingStage::GotComponentHandles(component_handles) => {
                let mut index = component_handles.len();

                while index != 0 {
                    index -= 1;
                    let component_handle = &component_handles[index];

                    let Some(path) = component_handle.path() else {
                        error!(
                            "Tried to get the path of a component from its handle. Something went \
                             wrong though. This error should never happen."
                        );
                        continue;
                    };

                    if let Some(serialised) = serialised.get(component_handle) {
                        let Some(path) = path.parent() else {
                            error!(
                                "Tried to get the parent folder of a serialised component, and \
                                 for some reason it does not have a parent."
                            );
                            continue;
                        };

                        let Some(entity_folder_name) = path.path().file_name() else {
                            error!(
                                "Tried to get the name of the parent folder of a serialised \
                                 component, and for some reason we failed."
                            );
                            continue;
                        };
                        let Some(entity_folder_name) = entity_folder_name.to_str() else {
                            error!("Tried to get a folder name but found it was invalid.");
                            continue;
                        };

                        let Ok(serialised_entity) = entity_folder_name.parse() else {
                            error!("An entity's folder name could not be parsed as a number.");
                            continue;
                        };
                        let serialised_entity = SerialisedEntity(serialised_entity);

                        let deserialised =
                            Self::deserialise(serialised, &mut deserialise_entity, &mut commands);

                        let entity = deserialise_entity.convert(serialised_entity, &mut commands);
                        commands.entity(entity).insert(deserialised);

                        load_finish.send(LoadFinish(entity));

                        // Iterating backwards, so this is safe.
                        component_handles.swap_remove(index);
                    }
                }

                if component_handles.len() == 0 {
                    *loading_stage = LoadingStage::WaitingForEvent;
                }
            }
        }
    }
}

pub trait AppSaveAndLoad {
    fn save_and_load<T: SaveAndLoad>(&mut self) -> &mut Self;
}

impl AppSaveAndLoad for App {
    fn save_and_load<T: SaveAndLoad>(&mut self) -> &mut Self {
        self.add_plugins(JsonAssetPlugin::<T::Serialised>::new(&[T::FILE_EXTENSION]));
        self.add_systems(Update, (T::save, T::load));
        self
    }
}

save_and_load_external! {
    pub struct Transform {
        pub translation: Vec3,
        pub rotation: Quat,
        pub scale: Vec3,
    }
}
