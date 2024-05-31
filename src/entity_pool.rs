use bevy::{
    ecs::{
        entity::Entity,
        system::Resource,
        world::{unsafe_world_cell::UnsafeWorldCell, EntityWorldMut, World},
    },
    hierarchy::DespawnRecursiveExt,
};
use std::{cell::RefCell, error, fmt, marker::PhantomData, ops};

/// Fixed capacity entity pool that
pub struct EntityPool(Vec<Entity>);

impl EntityPool {
    /// Reserve `capacity` number of entities  
    pub fn new(capacity: usize, world: &mut World) -> Self {
        let entities = world.entities().reserve_entities(capacity).collect();

        Self(entities)
    }

    /// SAFETY: Must be run only when world is being initialized before any other entities have been
    /// spawned to avoid clobbering the entity address space.
    pub unsafe fn reserve_in_world(&self, world: &mut World) {
        let mut world_entities = world.entities_mut();
        self.0.iter().copied().for_each(|e| {
            world_entities.alloc_at(e);
        })
    }

    pub fn get_handle(&mut self) -> EntityHandle {
        EntityHandle {
            entity: entity.pop(),
            pool: RefCell::new(self),
            dropped: false,
        }
    }
}

/// Handle that points to an entity.  
///
/// It is the callers responsibility to call '[EntityHandle::drop]' before the handle goes out of scope.  
///
/// # Panics   
///
/// Panics if handle goes out of scope without being explicitly dropped the owner.
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

impl<'a, T> ops::Deref for PoolHandle<'a, T> {
    type Target = T;

    fn deref(&self) -> &T {
        //  No Panic:
        //  -  `self.item` is initialized with `Some`, and never replaced.
        self.item.as_ref().unwrap()
    }
}

impl<'a, T> ops::DerefMut for PoolHandle<'a, T> {
    fn deref_mut(&mut self) -> &mut T {
        //  No Panic:
        //  -  `self.item` is initialized with `Some`, and never replaced.
        self.item.as_mut().unwrap()
    }
}

//
//  PoolExhausted
//

impl fmt::Display for PoolExhausted {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{self:?}")
    }
}

impl error::Error for PoolExhausted {}
