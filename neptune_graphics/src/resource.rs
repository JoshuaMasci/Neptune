use std::cell::RefCell;
use std::ops::Deref;
use std::rc::Rc;

pub struct Resource<T> {
    resource: Option<T>,
    deleter: Rc<RefCell<ResourceDeleter>>,
}

impl<T> Resource<T> {
    pub(crate) fn new(resource: T, deleter: Rc<RefCell<ResourceDeleter>>) -> Self {
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

impl<T> Drop for Resource<T> {
    fn drop(&mut self) {
        println!("Drop Resource!");
        let resource = self.resource.take().unwrap();
        self.deleter.borrow_mut().free_resource(move || {
            let _ = resource;
        });
    }
}

pub(crate) struct ResourceDeleter {
    current_frame: usize,
    freed_resource_lists: Vec<Vec<Box<dyn FnOnce()>>>,
}

impl ResourceDeleter {
    pub(crate) fn new(frame_count: usize) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Self {
            current_frame: 0,
            freed_resource_lists: (0..frame_count).map(|_| Vec::new()).collect(),
        }))
    }

    pub(crate) fn clear_frame(&mut self) {
        self.current_frame = (self.current_frame + 1) % self.freed_resource_lists.len();
        for drop_fn in self.freed_resource_lists[self.current_frame].drain(..) {
            drop_fn();
        }
    }

    pub(crate) fn free_resource(&mut self, drop_fn: impl FnOnce() + 'static) {
        self.freed_resource_lists[self.current_frame].push(Box::new(drop_fn));
    }
}

impl Drop for ResourceDeleter {
    fn drop(&mut self) {
        for _ in 0..self.freed_resource_lists.len() {
            self.clear_frame();
        }
    }
}
