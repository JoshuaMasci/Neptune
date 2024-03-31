use crate::transform::Transform;
use crate::universe::world::World;
use rapier3d::parry::utils::hashmap::HashMap;
use std::any::TypeId;

pub trait EntitySystem {
    fn update_pre_physics(&mut self, entity: &mut Entity, delta_time: f32);
    fn update_post_physics(&mut self, entity: &mut Entity, delta_time: f32);

    fn add_to_world(&mut self, world: &mut World, entity: &mut Entity);
    fn remove_from_world(&mut self, world: &mut World, entity: &mut Entity);
}

pub trait EntityComponent {}
pub struct Entity {
    name: String,
    transform: Transform,
    components: HashMap<TypeId, Box<dyn EntityComponent>>,
    nodes: NodePool,
    root_node: NodeIndex,
}

impl Default for Entity {
    fn default() -> Self {
        let mut nodes = NodePool::default();
        let root_node = nodes.add(Node {
            name: "Root Node".to_string(),
            ..Default::default()
        });

        Self {
            name: "Unnamed Entity".to_string(),
            transform: Default::default(),
            components: Default::default(),
            nodes,
            root_node,
        }
    }
}

pub trait NodeComponent {}

#[derive(Default)]
pub struct Node {
    name: String,
    local_transform: Transform,
    root_transform: Transform,
    components: HashMap<TypeId, Box<dyn NodeComponent>>,
    children: Vec<NodeIndex>,
}

#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub struct NodeIndex(usize);

#[derive(Default)]
pub struct NodePool {
    nodes: Vec<Option<Node>>,
    freed_ids: Vec<NodeIndex>,
}

impl NodePool {
    pub fn add(&mut self, node: Node) -> NodeIndex {
        if let Some(free_index) = self.freed_ids.pop() {
            self.nodes[free_index.0] = Some(node);
            free_index
        } else {
            let index = NodeIndex(self.nodes.len());
            self.nodes.push(Some(node));
            index
        }
    }

    pub fn remove(&mut self, index: NodeIndex) -> Option<Node> {
        if let Some(node) = self.nodes[index.0].take() {
            self.freed_ids.push(index);
            Some(node)
        } else {
            None
        }
    }

    pub fn get(&self, index: NodeIndex) -> Option<&Node> {
        self.nodes.get(index.0).and_then(Option::as_ref)
    }

    pub fn get_mut(&mut self, index: NodeIndex) -> Option<&mut Node> {
        self.nodes.get_mut(index.0).and_then(Option::as_mut)
    }
}
