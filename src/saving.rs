use std::{
    fs::{self, File},
    path::Path,
};

use bevy::utils::HashMap;
use serde::Serialize;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{
        EntityToIndex, IndexToEntity, Load, Save, Situation, StartLoad, StartSave,
    };
}

/// Anything saved will be relative to this path.
// Not a path, due to Paths not being allowed as constants.
const SAVE_PATH: &str = "./assets/saves/";

/// What we want to save/load.
// Does this actually make sense for both save and load?
// When would I ever want to just load lines??
#[derive(Copy, Clone)]
pub enum Situation {
    // String is relative save path.
    World,
    Lines,
}

/// Called just before everything should save.
/// This will make sure that everything is cleared out before sending the Save event.
/// The string is the path relative to SAVE_PATH for safety.
#[derive(Event)]
pub struct StartSave(pub Situation);

impl StartSave {
    /// Clears EntityToIndex and then sends of the save event.
    pub fn prepare(
        mut start_save: EventReader<Self>,
        mut save: EventWriter<Save>,
        mut entity_to_index: ResMut<EntityToIndex>,
    ) {
        start_save.read().for_each(|start_save| {
            entity_to_index.0.clear();
            entity_to_index.1 = 0;

            save.send(Save(start_save.0));
        });
    }
}

/// An event that is called whenever everything should save.
#[derive(Event)]
pub struct Save(pub Situation);

impl Save {
    /// Deletes (if it exists) and recreates the directory.
    pub fn prepare_path(path: impl AsRef<Path>) {
        let path = Path::new(SAVE_PATH).join(path);

        match fs::exists(&path) {
            Ok(exists) => {
                if exists {
                    if let Err(error) = fs::remove_dir_all(&path) {
                        error!(
                            "Tried to delete the folder containing the saved files. Got error: \
                             {error}"
                        );
                        return;
                    };
                }
            }
            Err(error) => {
                error!(
                    "Tried to check if the folder containing saved files exists. Got error: \
                     {error}"
                );
                return;
            }
        }

        if let Err(error) = fs::create_dir(&path) {
            error!("Tried to create the folder that would contain saved files. Got error: {error}");
            return;
        };
    }

    /// Saves a serialisiable value to a path.
    pub fn to_file<T: Serialize>(path: impl AsRef<Path>, value: &T) {
        let path = Path::new(SAVE_PATH).join(&path);
        let file = match File::create(&path) {
            Ok(file) => file,
            Err(error) => {
                error!(
                    "Tried to save file {}. Got this error {}.",
                    path.display(),
                    error
                );
                return;
            }
        };

        if let Err(error) = serde_json::to_writer_pretty(file, &value) {
            error!(
                "Tried to save file {}. During serialisation, we got this error {}.",
                path.display(),
                error
            );
        }
    }
}

/// Indicates to start loading from that path relative to SAVE_PATH.
#[derive(Event)]
pub struct StartLoad(pub Situation);

impl StartLoad {
    pub fn prepare(
        mut start_load: EventReader<StartLoad>,
        mut load: EventWriter<Load>,
        mut index_to_entity: ResMut<IndexToEntity>,
    ) {
        start_load.read().for_each(|start_load| {
            // So far, I'm thinking that we should leave deloading up to the components. They can deload themselves if they want. Or not.
            // We just clear this then.
            index_to_entity.0.clear();

            load.send(Load(start_load.0));
        });
    }
}

#[macro_export]
macro_rules! load_system {
    (serialised: $serialised:ty, spawn_parameters: $spawn_parameters:ty, despawn_parameters: $despawn_parameters:ty, spawner: $spawner:expr, despawner: $despawner:expr) => {
        fn load(
            mut load: EventReader<Load>,
            mut folders: ResMut<Assets<LoadedFolder>>,
            mut serialised: ResMut<Assets<$serialised>>,
            asset_server: Res<AssetServer>,
            mut folder_handle: Local<Option<Handle<LoadedFolder>>>,
            mut commands: Commands,
            mut spawn_parameters: $spawn_parameters,
            mut despawn_parameters: $despawn_parameters,
        ) {
            if let Some(handle) = folder_handle.as_ref() {
                let folder = some_or_return!(folders.get_mut(handle));

                if folder.handles.is_empty() {
                    *folder_handle = None;
                    info!("Finished loading folder.");
                } else {
                    // This while loop removes handles as they load.
                    let mut index = folder.handles.len();
                    while index != 0 {
                        index -= 1;
                        let line_id = ok_or_error_and_return!(
                            folder.handles[index].id().try_typed::<$serialised>(),
                            "Tried to load file. Got error:"
                        );
                        if let Some(serialised) = serialised.remove(line_id) {
                            // Iterating backwards, so this is fine.
                            folder.handles.swap_remove(index);

                            $spawner(serialised, &mut spawn_parameters, &mut commands);
                            info!("Loaded a file!");
                        }
                    }
                }
            } else {
                load.read().for_each(|load| {
                    if !matches!(load.0, Situation::World) {
                        return;
                    }

                    *folder_handle = Some(asset_server.load_folder("./saves/lines"));

                    $despawner(&mut despawn_parameters, &mut commands);

                    info!("Loading lines folder!");
                });
            }
        }
    };
}

/// Tells everything to load.
#[derive(Event)]
pub struct Load(pub Situation);

/// Because Entity is opaque, we must convert it to something that will never change.
#[derive(Resource, Default)]
pub struct EntityToIndex(HashMap<Entity, u32>, u32);

impl EntityToIndex {
    pub fn convert(&mut self, entity: Entity) -> u32 {
        // Get the index if it exists, else create the index and return it.
        if let Some(index) = self.0.get(&entity) {
            *index
        } else {
            let index = self.1;
            self.1 += 1;
            self.0.insert(entity, index);
            index
        }
    }
}

/// The inverse of the previous.
/// Converts indices to entities.
// TODO: Does this need to be reset on load? Do we need to despawn all previously loaded entities? Yes. Why?
#[derive(Resource, Default)]
pub struct IndexToEntity(HashMap<u32, Entity>);

impl IndexToEntity {
    // Infallible, because we create the entity if it doesn't exist.
    // Anything that needs to create an entity from an index must use this function.
    pub fn convert(&mut self, index: u32, commands: &mut Commands) -> Entity {
        // Get the index if it exists, else create the index and return it.
        if let Some(entity) = self.0.get(&index) {
            *entity
        } else {
            let entity = commands.spawn_empty().id();
            self.0.insert(index, entity);
            entity
        }
    }
}
