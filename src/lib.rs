use bevy::{
    ecs::{
        entity::Entity,
        system::Resource,
        world::{unsafe_world_cell::UnsafeWorldCell, EntityWorldMut, World},
    },
    hierarchy::DespawnRecursiveExt,
};
use std::{
    cell::{Cell, RefCell},
    error, fmt,
    marker::PhantomData,
    ops,
    sync::Arc,
};

/// Fixed capacity entity pool - gives out temporary access to a fixed number of entities via handles.
///
/// Primitive that enables using ECS Worlds as procedural scratch space in async tasks. Intended
/// for use in single-threaded contexts with exclusive world access.
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
/// An entity pool allows us to allocate a fixed amount of scratch space that has
/// - entity garbage collection
/// -  
/// onto the main world and inspected.
pub struct EntityPool<'pool> {
    entities: Arc<[Entity]>,
    in_use: Box<[bool]>,
    free_cursor: usize,
    handles: Vec<&'pool RefCell<EntityHandle<'pool>>>,
}

impl EntityPool {
    /// Reserve `capacity` number of entities  
    pub fn new(capacity: usize, world: &mut World) -> Self {
        let entities = world.entities().reserve_entities(capacity).collect();

        Self {
            entities: Arc::new(world.entities().reserve_entities(capacity).collect()),
            in_use: Box::new([false; capacity]),
            free_cursor: 0,
            handles: Vec::new(),
        }
    }

    /// SAFETY:  
    ///
    /// Must be run only at world initialization. Must not clash with any other entities in address space.
    pub unsafe fn reserve_in_world(&self, world: &mut World) {
        world
            .insert_or_spawn_batch(self.0)
            .inspect_err(|e| panic!("Failed to reserve entity pool entities in world: {e}"));
    }

    /// Returns a handle to an entity
    ///
    /// # Panics
    ///
    /// Panics on pool exhaustion if there are no more free entities to hand out.
    pub fn get_handle(&mut self) -> EntityHandle {
        if self.in_use[self.free_cursor] {
            // entity at free_cursor is in use - look through pool for a free entity
            let mut iter = self.in_use.iter().enumerate();
            if iter.any(|(_, &in_use)| !in_use) {
                match iter.next() {
                    Some((idx, _)) => self.free_cursor = idx,
                    None => panic!("pool exhaustion - all entities in use"),
                }
                *in_use = false;
            }
        }

        let handle = EntityHandle {
            entity: self.entities[self.free_cursor],
            pool: RefCell::new(self),
            dropped: false,
        };
        self.free_cursor = (self.free_cursor + 1) % self.entities.len();

        handle
    }

    /// Removes all components from in use entities
    ///
    /// WARNING:   
    pub fn free_all(&mut self, world: &mut World) {
        for (idx, in_use) in self.in_use.iter().enumerate() {
            if in_use {
                // TODO: 0.14 make this clear
                // world.entity_mut(self.entities[idx]).clear();
                *in_use = false;
            }
        }

        for handle in self.handles {
            handle.borrow_mut().dropped.set(true);
        }
    }
}

// TODO: Entity pool is an ungly ugly non-send ds - we should create it inside the
// async task based on a list of reserved_entities.
//
// TODO: when we switch what snapshot we're looking at, we don't need to clear. We serialize
// all the empty entities, if they're empty treat them as nothing.
//

/// EntityPool
///
/// We're trying to take NonSend resources out of world.
impl Clone for EntityPool {
    fn clone(&self) -> Self {
        Self {
            entities: self.entities.clone(),
            in_use: Box::new([false; capacity]),
            free_cursor: 0,
            handles: Vec::new(),
        }
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
    dropped: Cell<bool>,
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
