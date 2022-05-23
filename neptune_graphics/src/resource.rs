use std::any::{Any, TypeId};
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

//TODO: fix this entire class!!!!!!
pub struct Resource<T: 'static> {
    resource: Option<T>,
    deleter: Arc<Mutex<ResourceDeleterInner>>,
}

impl<T> Resource<T> {
    fn new(resource: T, deleter: Arc<Mutex<ResourceDeleterInner>>) -> Self {
        Self {
            resource: Some(resource),
            deleter,
        }
    }
}

impl<T> Deref for Resource<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.resource.as_ref().unwrap()
    }
}

impl<T: 'static> Drop for Resource<T> {
    fn drop(&mut self) {
        let resource = self.resource.take().unwrap();
        {
            let mut lock = self.deleter.borrow_mut().lock().unwrap();
            lock.free_resource(resource);
        }
    }
}

pub(crate) struct ResourceDeleterInner {
    current_frame: usize,
    no_wait: bool,
    freed_resource_lists: Vec<HashMap<TypeId, Box<dyn Any>>>,
}

impl ResourceDeleterInner {
    pub(crate) fn new(frame_count: usize) -> Self {
        Self {
            current_frame: 0,
            no_wait: false,
            freed_resource_lists: (0..frame_count).map(|_| HashMap::new()).collect(),
        }
    }

    fn set_no_wait(&mut self, no_wait: bool) {
        self.no_wait = no_wait;
    }

    pub(crate) fn clear_frame(&mut self) -> HashMap<TypeId, Box<dyn Any>> {
        self.current_frame = (self.current_frame + 1) % self.freed_resource_lists.len();
        std::mem::take(&mut self.freed_resource_lists[self.current_frame])
    }

    pub(crate) fn free_resource<T: 'static>(&mut self, resource: T) {
        if self.no_wait {
            drop(resource);
        } else {
            let type_id = TypeId::of::<T>();
            if let Some(free_list) = self.freed_resource_lists[self.current_frame].get_mut(&type_id)
            {
                let free_list: &mut Box<Vec<T>> = free_list.downcast_mut().unwrap();
                free_list.push(resource)
            } else {
                let free_list: Box<Vec<T>> = Box::new(vec![resource]);
                self.freed_resource_lists[self.current_frame].insert(type_id, free_list);
            }
        }
    }
}

impl Drop for ResourceDeleterInner {
    fn drop(&mut self) {
        self.no_wait = true;
    }
}

pub struct ResourceDeleter {
    deleter: Arc<Mutex<ResourceDeleterInner>>,
}

impl ResourceDeleter {
    pub(crate) fn new(frame_count: usize) -> Self {
        Self {
            deleter: Arc::new(Mutex::new(ResourceDeleterInner::new(frame_count))),
        }
    }

    pub(crate) fn create_resource<T: 'static>(&self, resource: T) -> Resource<T> {
        Resource::new(resource, self.deleter.clone())
    }

    pub(crate) fn clear_frame(&mut self) {
        //Despite using an Arc this isn't perfectly thread safe yet
        //If another thread calls between the two set_no_wait calls in will auto drop the resource which is bad
        let resources = {
            let mut lock = self.deleter.borrow_mut().lock().unwrap();
            lock.set_no_wait(true);
            lock.clear_frame()
        };
        drop(resources);
        {
            let mut lock = self.deleter.borrow_mut().lock().unwrap();
            lock.set_no_wait(false);
        }
    }
}
