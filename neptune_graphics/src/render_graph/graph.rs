use std::collections::HashMap;

pub type NodeId = usize;

struct GraphEdge<E> {
    from: NodeId,
    to: NodeId,
    data: E,
}

struct GraphNode<N, E> {
    data: N,
    edges: Vec<GraphEdge<E>>,
}

pub struct Graph<N, E> {
    nodes: HashMap<NodeId, GraphNode<N, E>>,
}
