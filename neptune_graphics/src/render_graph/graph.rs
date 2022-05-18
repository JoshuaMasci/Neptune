use std::collections::HashMap;

pub type NodeId = usize;

struct GraphEdge<E> {
    from: NodeId,
    data: E,
}

struct GraphNode<N, E> {
    data: N,
    edges: Vec<GraphEdge<E>>,
}

pub struct Graph<N, E> {
    next_node_id: NodeId,
    nodes: HashMap<NodeId, GraphNode<N, E>>,
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

    pub fn get_unconnected_nodes(&mut self) {}
}
