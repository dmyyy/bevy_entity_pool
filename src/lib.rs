use bevy::ecs::{
    entity::Entity,
    world::{EntityWorldMut, World, WorldId},
};
use std::{ops::Deref, sync::Arc};

/// Fixed capacity entity pool - gives out temporary access to a fixed number of entities via handles.
/// Handles can only be reclaimed by calling '[EntityPool::free]' - it is expected that only
/// locally relevant entities are used in the scratch-world and that entities are periodically freed.
///
/// Primitive that enables using ECS Worlds as procedural scratch space in async tasks. Intended
/// for use in long-running single-threaded contexts with exclusive world access.
///
/// This pattern can be summarized as:
/// - reserve a fixed entity address space in the main world
/// - spawn an async task that performs some long-running procedural/generative work that would either
///     - benefit from having a type-erased dynamic storage to work with to build abstractions, or
///     - is hard algorithmic work that we want to use later at runtime which will be stored in a fixed number of entities/resources
/// - spawn a task with the reserved entities and some optional configuration options
/// - perform hard work and store the results of said work in the scratch world
/// - copy that work back into main world using bevy_scene
///
/// # Panics
/// Panics on pool exhaustion.
pub struct EntityPool {
    world_id: WorldId,
    entities: Arc<[Entity]>,
    free_cursor: usize,
    handles: Vec<EntityHandle>,
}

impl EntityPool {
    /// Initializes an entity pool and reserves required entities.
    ///
    /// # Panics
    /// Panics if it isn't possible to spawn all entities.
    pub fn new(entities: Vec<Entity>, world: &mut World) -> Self {
        world
            .insert_or_spawn_batch(entities.iter().copied().map(|e| (e, ())))
            .inspect_err(|e| panic!("Failed to spawn all entities {e:?}"));

        Self {
            world_id: world.id(),
            entities: Arc::from(entities.as_slice()),
            free_cursor: 0,
            handles: Vec::new(),
        }
    }

    /// Returns a handle to an entity
    ///
    /// # Panics
    /// Panics on pool exhaustion
    pub fn get_handle(&mut self) -> &EntityHandle {
        if self.free_cursor > self.entities.len() - 1 {
            panic!("pool exhaustion - all entities in use");
        }

        let handle = EntityHandle {
            entity: self.entities[self.free_cursor],
            dropped: false,
        };
        self.handles.push(handle);
        self.free_cursor += 1;

        self.handles.last().unwrap()
    }

    /// Invalidates and reclaims all in use handles.  
    ///
    /// # Panics
    /// Panics if an entity handle is dereferenced after a
    pub fn free(&mut self, world: &mut World) {
        // make sure world we're freeing from is the same world we initialized with
        debug_assert_eq!(self.world_id, world.id());

        for entity in self.entities[0..self.free_cursor].iter().copied() {
            // TODO: 0.14 clear all components on entity
            // world.entity_mut(entity).clear();
        }

        for handle in &mut self.handles {
            handle.dropped = true;
        }
    }
}

/// Handle that points to an entity. Smart pointer that enforces memory safety of underlying entity, namely
/// - handle should not be droppe
///
/// # Panics   
/// Panics if handle goes out of scope without being explicitly freed.
pub struct EntityHandle {
    entity: Entity,
    dropped: bool,
}

impl EntityHandle {
    pub fn entity_mut<'w>(&'w self, world: &'w mut World) -> EntityWorldMut {
        world.entity_mut(self.entity)
    }
}

impl Drop for EntityHandle {
    fn drop(&mut self) {
        if !self.dropped {
            panic!(
                "entity handle went out of scope without being explicitly dropped - memory leak"
            );
        }
    }
}

impl Deref for EntityHandle {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        if self.dropped {
            panic!("dereferenced entity handle even though handle has already been dropped - use after free");
        }
        &self.entity
    }
}
