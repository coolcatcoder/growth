use std::fs::{self, File};

use bevy::utils::HashMap;
use serde::Serialize;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{EntityToIndex, IndexToEntity, Load, Save, StartSave};
}

const SAVE_PATH: &str = "./assets/saves/";

/// Called just before everything should save.
/// This will make sure that everything is cleared out before sending the Save event.
/// The string is the path relative to SAVE_PATH for safety.
#[derive(Event)]
pub struct StartSave(pub String);

impl StartSave {
    /// Gets rid of previous save data, and then sends out the Save event.
    pub fn clear_previous(mut start_save: EventReader<StartSave>, mut save: EventWriter<Save>) {
        start_save.read().for_each(|start_save| {
            let path = format!("{SAVE_PATH}{}", start_save.0);
            // If the folder doesn't exist, don't try delete it.
            // If it does exist, try delete it.
            match fs::exists(&path) {
                Ok(exists) => {
                    if exists {
                        if let Err(error) = fs::remove_dir_all(&path) {
                            error!(
                                "Tried to delete the folder containing the saved files. Got \
                                 error: {error}"
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

            if let Err(error) = fs::create_dir(format!("{SAVE_PATH}{}", start_save.0)) {
                error!(
                    "Tried to create the folder that would contain saved files. Got error: {error}"
                );
                return;
            };

            save.send(Save(start_save.0.clone()));
        });
    }
}

/// An event that is called whenever everything should save.
#[derive(Event)]
pub struct Save(String);

impl Save {
    pub fn new(path: String) -> Self {
        Self(path)
    }

    pub fn to_file<T: Serialize>(&self, file_name: String, value: &T) {
        let file = match File::create(format!("{SAVE_PATH}{}{}", self.0, file_name)) {
            Ok(file) => file,
            Err(error) => {
                error!(
                    "Tried to save file with name {} to {}. Got this error {}.",
                    file_name, self.0, error
                );
                return;
            }
        };

        if let Err(error) = serde_json::to_writer_pretty(file, &value) {
            error!(
                "Tried to save file with name {} to {}. During serialisation, we got this error \
                 {}.",
                file_name, self.0, error
            );
        }
    }
}

/// Uncertain.
#[derive(Event)]
pub struct Load(String);

/// Because Entity is opaque, we must convert it to something that will never change.
/// We will never need to reset this. If an entity uses it again, it will just get the index it is supposed to.
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
// TODO: Does this need to be reset on load? Do we need to despawn all previously loaded entities? Yes.
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
