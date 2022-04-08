use crate::resource_deleter::ResourceDeleter;
use std::cell::RefCell;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

pub trait ResourceDrop {
    fn drop_resource(deleter: &mut ResourceDeleter, resource: Self);
}

struct Resource<T: ResourceDrop> {
    resource: Option<T>,
    deleter: Rc<RefCell<ResourceDeleter>>,
}

impl<T: ResourceDrop> Resource<T> {
    fn new(resource: T, deleter: Rc<RefCell<ResourceDeleter>>) -> Self {
        Self {
            resource: Some(resource),
            deleter,
        }
    }
}

impl<T: ResourceDrop> Deref for Resource<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.resource.as_ref().unwrap()
    }
}

impl<T: ResourceDrop> DerefMut for Resource<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.resource.as_mut().unwrap()
    }
}

impl<T: ResourceDrop> Drop for Resource<T> {
    fn drop(&mut self) {
        let resource = self.resource.take().unwrap();
        T::drop_resource(self.deleter.get_mut(), resource);
    }
}
