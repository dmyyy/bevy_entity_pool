use bevy::{
    ecs::{
        entity::Entity,
        system::Resource,
        world::{unsafe_world_cell::UnsafeWorldCell, World},
    },
    hierarchy::DespawnRecursiveExt,
};
use std::{cell::RefCell, error, fmt, marker::PhantomData, ops};

// PROBLEM: what world are we keeping track of?
// Do we remove from world on drop
// handle is associated with a specific world
// on dropping the handle we

// TODO: can we do this via observers?
// For all entities in pool add an observer
// - on dropping entity handle we trigger cleanup

// pub struct Alive;
// pub struct Dropped;

// pub trait HandleState {}

// impl HandleState for Alive {}
// impl HandleState for Dropped {}

/// Handle that points to an entity.  
///
/// All components associated with the underlying entity must be cleaned up  
pub struct EntityHandle {
    entity: Entity,
    /// Reclaims the entity upon drop    
    pool: RefCell<EntityPool>,
    dropped: bool,
}

impl EntityHandle {
    pub fn drop(mut self, world: &mut World) {
        if !self.dropped {
            let mut entity_mut = world
                .get_entity_mut(self.entity)
                .expect("entity should live as long as handle is alive");
            // TODO: 0.14
            // clear all components on the entity
            // self.clear();

            self.dropped = true;
        }
    }
}

impl Drop for EntityHandle {
    fn drop(&mut self) {
        if !self.dropped {
            panic!("entity handles need to be explicitly dropped to clean up world");
        }

        let mut pool = self.pool.get_mut();
        // TODO: use pool to
    }
}

/// Fixed capacity entity pool that
pub struct EntityPool {
    entities: Vec<Entity>,
    ///     
    free_cursor: usize,
}

impl EntityPool {
    /// Reserve `capacity` number of entities  
    pub fn new(capacity: usize, world: &mut World) -> Self {
        let entities = world.entities().reserve_entities(capacity).collect();

        Self {
            entities: entities.into_iter().map(|e| e.index()).collect(),
            free_cursor: 0,
        }
    }

    // given an entity pool that already exists
    pub fn reserve_in_world(&self, world: &mut World) {
        for e in self.entities.iter() {
            // reserve entity in world
        }
    }

    pub fn get_handle(&mut self) -> EntityHandle {
        EntityHandle {}
    }
}

pub struct EntityPool {
    raw_entities: Vec<u32>,
    in_use: Vec<bool>,
}

// implement an trigger that
///
impl EntityPool {
    /// Reserve `capacity` number of entities  
    pub fn new(capacity: usize, world: &mut World) -> Self {
        // TODO: 0.14
        // Register an observer on each entity passed in
        // - I need to register observer on
        // - takes an observer that triggers on OnAdd<Cleanup> (Cleanup component is added)
        //
        // PROBLEM: make sure merging with different generations is okay.
        //
        // Add a trigger to every rule entity.
        // OnRemove<Rule<Final>>
        // -

        // reserve some number of entities as scratch space
        let entities = world.entities().reserve_entities(capacity).collect();

        Self {
            entities: entities.into_iter().map(|e| e.index()).collect(),
            in_use: Vec::with_capcity(entities.len()),
        }
    }

    pub fn get_handle(&mut self) -> Box<Entity> {
        Box::new()
    }

    // given an entity pool that already exists
    pub fn reserve_in_world(&self, world: &mut World) {
        for entitiy in self.entities {}
    }

    pub fn get(&mut self) -> &Entity {}

    // don't need the traditional pool behavior of giving out a handle?
}

#[derive(Debug)]
pub struct PoolHandle<'a, T> {
    stack: &'a RefCell<Vec<T>>,
    //  An advanced version would use `ManuallyDrop`, but that involves unsafe code.
    item: Option<T>,
}

#[derive(Debug)]
pub struct PoolExhausted;

//
//  Pool
//

impl<T> EntityPool<T>
where
    T: Default,
{
    pub fn new(number_items: usize) -> Self {
        let stack = (0..number_items).map(|_| T::default()).collect();

        Self {
            stack: RefCell::new(stack),
        }
    }
}

impl<T> Pool<T> {
    pub fn initialize<F, E>(number_items: usize, factory: F) -> Result<Self, E>
    where
        F: FnMut(usize) -> Result<T, E>,
    {
        let stack: Result<Vec<_>, E> = (0..number_items).map(factory).collect();

        stack.map(|stack| Self {
            stack: RefCell::new(stack),
        })
    }

    pub fn acquire(&self) -> Result<PoolHandle<'_, T>, PoolExhausted> {
        let Some(item) = self.stack.borrow_mut().pop() else {
            return Err(PoolExhausted);
        };

        Ok(PoolHandle {
            stack: &self.stack,
            item: Some(item),
        })
    }
}

// TODO: we can do this in a custom drop impl - that shouldn't have to compete with BLAH

//
//  PoolHandle
//

impl<'a, T> Drop for PoolHandle<'a, T> {
    fn drop(&mut self) {
        //  No Panic:
        //  -  `self.item` is initialized with `Some`, and never replaced.
        let item = self.item.take().unwrap();

        self.stack.borrow_mut().push(item);
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
