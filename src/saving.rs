use bevy::utils::HashMap;

pub use crate::prelude::*;

pub mod prelude {
    pub use super::{EntityToIndex, IndexToEntity, Load, Save};
}

/// An event that is called whenever everything should save.
#[derive(Event)]
pub struct Save;

/// Uncertain.
#[derive(Event)]
pub enum Load {
    // Store resource path?
    Save(String),
}

/// Because Entity is opaque, we must convert it to something that will never change.
/// We will never need to reset this. If an entity uses it again, it will just get the index it is supposed to.
#[derive(Resource, Default)]
pub struct EntityToIndex(HashMap<Entity, usize>, usize);

impl EntityToIndex {
    pub fn convert(&mut self, entity: Entity) -> usize {
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
// TODO: Does this need to be reset on load? Do we need to despawn all previously loaded entities?
#[derive(Resource, Default)]
pub struct IndexToEntity(HashMap<usize, Entity>);

impl IndexToEntity {
    // Infallible, because we create the entity if it doesn't exist.
    // Anything that needs to create an entity from an index must use this function.
    pub fn convert(&mut self, index: usize, commands: &mut Commands) -> Entity {
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
