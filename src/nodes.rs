use crate::{
    pacman::Direction,
    render::{Circle, FrameData, Line},
    vector::Vector2,
};

pub type NodeId = usize;

#[derive(Clone, Debug)]
pub struct Node {
    position: Vector2,
    neighbors: [Option<NodeId>; 4],
}

#[derive(Clone, Debug, Default)]
pub struct NodeGroup {
    nodes: Vec<Node>,
}

impl Node {
    pub fn new(x: f32, y: f32) -> Self {
        Self {
            position: Vector2::new(x, y),
            neighbors: [None, None, None, None],
        }
    }
}

impl NodeGroup {
    pub fn setup_test_nodes() -> Self {
        let mut nodes = vec![
            Node::new(80.0, 80.0),
            Node::new(160.0, 80.0),
            Node::new(80.0, 160.0),
            Node::new(160.0, 160.0),
            Node::new(208.0, 160.0),
            Node::new(80.0, 320.0),
            Node::new(208.0, 320.0),
        ];

        set_neighbor(&mut nodes, 0, Direction::Right, 1);
        set_neighbor(&mut nodes, 0, Direction::Down, 2);
        set_neighbor(&mut nodes, 1, Direction::Left, 0);
        set_neighbor(&mut nodes, 1, Direction::Down, 3);
        set_neighbor(&mut nodes, 2, Direction::Up, 0);
        set_neighbor(&mut nodes, 2, Direction::Right, 3);
        set_neighbor(&mut nodes, 2, Direction::Down, 5);
        set_neighbor(&mut nodes, 3, Direction::Up, 1);
        set_neighbor(&mut nodes, 3, Direction::Left, 2);
        set_neighbor(&mut nodes, 3, Direction::Right, 4);
        set_neighbor(&mut nodes, 4, Direction::Left, 3);
        set_neighbor(&mut nodes, 4, Direction::Down, 6);
        set_neighbor(&mut nodes, 5, Direction::Up, 2);
        set_neighbor(&mut nodes, 5, Direction::Right, 6);
        set_neighbor(&mut nodes, 6, Direction::Up, 4);
        set_neighbor(&mut nodes, 6, Direction::Left, 5);

        Self { nodes }
    }

    pub fn start_node(&self) -> NodeId {
        0
    }

    pub fn neighbor(&self, node_id: NodeId, direction: Direction) -> Option<NodeId> {
        let index = direction.neighbor_index()?;
        self.nodes
            .get(node_id)
            .and_then(|node| node.neighbors[index])
    }

    pub fn position(&self, node_id: NodeId) -> Vector2 {
        self.nodes[node_id].position
    }

    pub fn append_renderables(&self, frame: &mut FrameData) {
        for (index, node) in self.nodes.iter().enumerate() {
            for direction in Direction::cardinals() {
                if let Some(neighbor_id) = self.neighbor(index, direction) {
                    frame.lines.push(Line {
                        start: node.position,
                        end: self.position(neighbor_id),
                        color: [255, 255, 255, 255],
                        thickness: 4.0,
                    });
                }
            }

            frame.circles.push(Circle {
                center: node.position,
                radius: 12.0,
                color: [255, 0, 0, 255],
            });
        }
    }
}

fn set_neighbor(nodes: &mut [Node], node_id: NodeId, direction: Direction, neighbor_id: NodeId) {
    let index = direction
        .neighbor_index()
        .expect("cardinal directions should have a neighbor slot");
    nodes[node_id].neighbors[index] = Some(neighbor_id);
}

#[cfg(test)]
mod tests {
    use super::NodeGroup;
    use crate::pacman::Direction;

    #[test]
    fn test_nodes_match_the_tutorial_graph() {
        let nodes = NodeGroup::setup_test_nodes();

        assert_eq!(nodes.neighbor(0, Direction::Right), Some(1));
        assert_eq!(nodes.neighbor(0, Direction::Down), Some(2));
        assert_eq!(nodes.neighbor(3, Direction::Right), Some(4));
        assert_eq!(nodes.neighbor(5, Direction::Right), Some(6));
        assert_eq!(nodes.neighbor(6, Direction::Down), None);
    }

    #[test]
    fn start_node_is_node_a() {
        let nodes = NodeGroup::setup_test_nodes();
        assert_eq!(nodes.position(nodes.start_node()).as_tuple(), (80.0, 80.0));
    }
}
