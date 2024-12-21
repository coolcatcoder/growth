use std::{
    fs::{self, File},
    path::Path,
};

use bevy::utils::HashMap;
use serde::Serialize;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{EntityToIndex, IndexToEntity, Load, Save, Situation, StartSave};
}

/// Anything saved will be relative to this path.
// Not a path, due to Paths not being allowed as constants.
const SAVE_PATH: &str = "./assets/saves/";

/// What we want to save/load.
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
pub struct StartLoad(String);

impl StartLoad {
    pub fn clear_and_get_handles(
        mut start_load: EventReader<StartLoad>,
        mut load: EventWriter<Load>,
        mut index_to_entity: ResMut<IndexToEntity>,
    ) {
        start_load.read().for_each(|start_load| {
            // I need to think carefully here.
            // Do we want to just clear it, and have a seperate Deload event?
            // Do we want to interate and remove all previously loaded entities?
            index_to_entity.0.clear();
        });
    }
}

/// Tells everything to load. Only load assets from the handles provided.
#[derive(Event)]
pub struct Load(Vec<UntypedHandle>);

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
