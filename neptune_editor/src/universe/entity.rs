use crate::transform::Transform;
use crate::universe::system::EntitySystemPool;
use rapier3d::parry::utils::hashmap::HashMap;
use std::any::{Any, TypeId};

pub struct Entity {
    pub data: EntityData,
    pub systems: EntitySystemPool,
}

pub struct EntityData {
    pub name: String,
    pub transform: Transform,
    pub components: ComponentPool,
    pub nodes: NodePool,
}

impl Default for EntityData {
    fn default() -> Self {
        Self {
            name: "Unnamed Entity".to_string(),
            transform: Default::default(),
            components: Default::default(),
            nodes: Default::default(),
        }
    }
}

#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct NodeIndex(usize);

#[derive(Default)]
pub struct Node {
    pub name: String,
    pub local_transform: Transform,
    pub components: ComponentPool,

    pub parent_node: Option<NodeIndex>,
    pub children: Vec<NodeIndex>,
}

#[derive(Default)]
pub struct NodePool {
    root_nodes: Vec<NodeIndex>,
    nodes: Vec<Option<Node>>,
    freed_ids: Vec<NodeIndex>,
}

impl NodePool {
    pub fn insert(&mut self, parent: Option<NodeIndex>, node: Node) -> NodeIndex {
        let index = if let Some(freed_index) = self.freed_ids.pop() {
            freed_index
        } else {
            let index = NodeIndex(self.nodes.len());
            self.nodes.push(None);
            index
        };

        if let Some(parent_index) = parent {
            if let Some(parent_node) = self.nodes.get_mut(parent_index.0).and_then(Option::as_mut) {
                parent_node.children.push(index);
            }
        } else {
            self.root_nodes.push(index);
        }

        self.nodes[index.0] = Some(node);
        index
    }

    pub fn remove(&mut self, index: NodeIndex) {
        if let Some(mut node) = self.nodes.get_mut(index.0).and_then(Option::take) {
            for child_index in node.children.drain(..) {
                self.remove(child_index);
            }

            if let Some(parent_index) = node.parent_node {
                if let Some(parent_node) = self.get_mut(parent_index) {
                    parent_node.children.retain(|child| *child != index);
                }
            } else {
                self.root_nodes.retain(|root| *root != index);
            }

            self.freed_ids.push(index);
        }
    }

    pub fn get(&self, index: NodeIndex) -> Option<&Node> {
        self.nodes.get(index.0).and_then(Option::as_ref)
    }

    pub fn get_mut(&mut self, index: NodeIndex) -> Option<&mut Node> {
        self.nodes.get_mut(index.0).and_then(Option::as_mut)
    }
}

#[derive(Default)]
pub struct ComponentPool {
    component_map: HashMap<TypeId, Box<dyn Any>>, //TODO: have components require serde (for saving/loading) + GUI display code (for editor functionality)
}

impl ComponentPool {
    pub fn insert<T: 'static>(&mut self, component: T) -> Option<T> {
        self.component_map
            .insert(TypeId::of::<T>(), Box::new(component))
            .map(|boxed| *boxed.downcast().unwrap())
    }

    pub fn remove<T: 'static>(&mut self) -> Option<T> {
        self.component_map
            .remove(&TypeId::of::<T>())
            .map(|boxed| *boxed.downcast().unwrap())
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.component_map
            .get(&TypeId::of::<T>())
            .map(|boxed| boxed.downcast_ref().unwrap())
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.component_map
            .get_mut(&TypeId::of::<T>())
            .map(|boxed| boxed.downcast_mut().unwrap())
    }
}

#[macro_export]
macro_rules! components {
    [] => (
       ComponentPool::default()
    );
    [$($component:expr),*] => {{
        let mut pool = ComponentPool::default();
        $(pool.insert($component);)*
        pool
    }};
}
