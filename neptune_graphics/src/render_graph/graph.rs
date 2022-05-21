use std::collections::HashMap;

pub type NodeId = usize;

pub struct GraphEdge<E> {
    pub from: NodeId,
    pub data: E,
}

pub struct GraphNode<N, E> {
    pub data: N,
    pub edges: Vec<GraphEdge<E>>,
}

pub struct Graph<N, E> {
    pub next_node_id: NodeId,
    pub nodes: HashMap<NodeId, GraphNode<N, E>>,
}

impl<N, E> Graph<N, E> {
    pub fn new() -> Self {
        Self {
            next_node_id: 0,
            nodes: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node: N) -> NodeId {
        let node_id = self.next_node_id + 1;
        self.next_node_id += 1;

        self.nodes.insert(
            node_id,
            GraphNode {
                data: node,
                edges: vec![],
            },
        );

        node_id
    }

    pub fn add_edge(&mut self, from: NodeId, to: NodeId, data: E) {
        self.nodes
            .get_mut(&to)
            .unwrap()
            .edges
            .push(GraphEdge { from, data });
    }

    pub fn get_unconnected_nodes(&mut self) -> Option<Vec<GraphNode<N, E>>> {
        let mut unconnected_nodes = Vec::new();

        'outer: for (&id, node) in self.nodes.iter() {
            for edge in node.edges.iter() {
                if self.nodes.contains_key(&edge.from) {
                    continue 'outer;
                }
            }
            unconnected_nodes.push(id);
        }

        if unconnected_nodes.is_empty() {
            None
        } else {
            Some(
                unconnected_nodes
                    .drain(..)
                    .map(|id| self.nodes.remove(&id).unwrap())
                    .collect(),
            )
        }
    }
}
