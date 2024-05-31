use bevy::{
    ecs::{
        entity::Entity,
        system::Resource,
        world::{unsafe_world_cell::UnsafeWorldCell, EntityWorldMut, World},
    },
    hierarchy::DespawnRecursiveExt,
};
use std::{cell::RefCell, error, fmt, marker::PhantomData, ops};

/// Fixed capacity entity pool
///
/// Primitive that enables using ECS Worlds as procedural scratch space in async tasks. Intended
/// for use with exclusive world access.
///
/// This pattern can be summarized as:
/// - reserve a fixed entity address space in the main world
/// - spawn an async task that performs some long-running procedural/generative work that would either
///     - benefit from having a type-erased dynamic storage to work with to build abstractions, or
///     - is hard algorithmic work that we want to use later at runtime which will be stored in a fixed number of entities/resources
/// - spawn a task with the reserved entities and some optional configuration options
/// - perform hard work and store the results of said work in the scratch world
/// - copy that scratch work back into the main world using bevy_scene
///
/// An entity pool allows us to allocate a fixed amount of scratch space that can be naively cloned
/// onto the main world and inspected.
pub struct EntityPool(Box<[Entity]>);

impl EntityPool {
    /// Reserve `capacity` number of entities  
    pub fn new(capacity: usize, world: &mut World) -> Self {
        let entities = world.entities().reserve_entities(capacity).collect();

        Self(entities)
    }

    /// SAFETY:  
    ///
    /// Must be run only at world initialization. Must not clash with any other entities.
    pub unsafe fn reserve_in_world(&self, world: &mut World) {
        world
            .insert_or_spawn_batch(self.0)
            .inspect_err(|e| "Failed to reserve entity pool entities in world: {e}");
    }

    pub fn get_handle(&mut self) -> EntityHandle {
        EntityHandle {
            entity: entity.pop(),
            pool: RefCell::new(self),
            dropped: false,
        }
    }

    /// NOTE: Must be called between switching scenes.  
    pub fn clear(&self, world: &mut World) {
        // TODO:
        // want two things
        // - something that tells us what all of the entities are in the pool. This should be immutable
        // - something that tells us if an entity is being used.
        // no notion of hierarchy
    }
}

/// Handle that points to an entity.  
///
/// It is the callers responsibility to call '[EntityHandle::drop]' before the handle goes out of scope.  
///
/// # Panics   
///
/// Panics if handle goes out of scope without being explicitly dropped by the owner.
pub struct EntityHandle<'a> {
    entity: Entity,
    pool: &'a RefCell<EntityPool>,
    dropped: bool,
}

impl<'a> EntityHandle<'a> {
    pub fn entity_mut(&self, world: &mut World) -> EntityWorldMut {
        world.entity_mut(self.entity)
    }

    /// Removes components associated with the handles entity in world. Must be called before handle
    /// goes out of scope.
    pub fn drop(mut self, world: &mut World) {
        if !self.dropped {
            let mut entity_mut = world
                .get_entity_mut(self.entity)
                .expect("entity should live as long as handle is alive");
            // TODO: 0.14
            // clear all components on the entity
            // entity_mut.clear();

            self.pool.get_mut().0.push(self.entity);
            self.dropped = true;
        }
    }
}

impl Drop for EntityHandle {
    fn drop(&mut self) {
        if !self.dropped {
            panic!("entity handle went out of scope without being explicitly dropped");
        }
    }
}
