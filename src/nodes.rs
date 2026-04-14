use std::collections::HashMap;

use crate::{
    constants::{RED, TILE_HEIGHT, TILE_WIDTH, WHITE},
    pacman::Direction,
    render::{Circle, FrameData, Line},
    vector::Vector2,
};

const MAZE_TEST: &str = include_str!("../assets/mazetest.txt");
const MAZE_ONE: &str = include_str!("../assets/maze1.txt");

pub type NodeId = usize;

#[derive(Clone, Debug)]
pub struct Node {
    position: Vector2,
    neighbors: [Option<NodeId>; 4],
    portal: Option<NodeId>,
}

#[derive(Clone, Debug, Default)]
pub struct NodeGroup {
    nodes: Vec<Node>,
    lookup: HashMap<(i32, i32), NodeId>,
}

#[derive(Clone, Debug)]
struct MazeData {
    tiles: Vec<Vec<char>>,
    width: usize,
}

impl Node {
    fn new(x: i32, y: i32) -> Self {
        Self {
            position: Vector2::new(x as f32, y as f32),
            neighbors: [None; 4],
            portal: None,
        }
    }
}

impl NodeGroup {
    pub fn setup_test_nodes() -> Self {
        let nodes = vec![
            Node::new(80, 80),
            Node::new(160, 80),
            Node::new(80, 160),
            Node::new(160, 160),
            Node::new(208, 160),
            Node::new(80, 320),
            Node::new(208, 320),
        ];
        let mut group = Self::with_nodes(nodes);

        link_neighbors(&mut group.nodes, 0, Direction::Right, 1);
        link_neighbors(&mut group.nodes, 0, Direction::Down, 2);
        link_neighbors(&mut group.nodes, 1, Direction::Left, 0);
        link_neighbors(&mut group.nodes, 1, Direction::Down, 3);
        link_neighbors(&mut group.nodes, 2, Direction::Up, 0);
        link_neighbors(&mut group.nodes, 2, Direction::Right, 3);
        link_neighbors(&mut group.nodes, 2, Direction::Down, 5);
        link_neighbors(&mut group.nodes, 3, Direction::Up, 1);
        link_neighbors(&mut group.nodes, 3, Direction::Left, 2);
        link_neighbors(&mut group.nodes, 3, Direction::Right, 4);
        link_neighbors(&mut group.nodes, 4, Direction::Left, 3);
        link_neighbors(&mut group.nodes, 4, Direction::Down, 6);
        link_neighbors(&mut group.nodes, 5, Direction::Up, 2);
        link_neighbors(&mut group.nodes, 5, Direction::Right, 6);
        link_neighbors(&mut group.nodes, 6, Direction::Up, 4);
        link_neighbors(&mut group.nodes, 6, Direction::Left, 5);

        group
    }

    pub fn maze_basics() -> Self {
        Self::from_text(MAZE_TEST, &['+'], &['.'])
    }

    pub fn pacman_maze() -> Self {
        Self::from_text(MAZE_ONE, &['+', 'P', 'n'], &['.', '-', '|', 'p'])
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

    pub fn portal(&self, node_id: NodeId) -> Option<NodeId> {
        self.nodes.get(node_id).and_then(|node| node.portal)
    }

    pub fn position(&self, node_id: NodeId) -> Vector2 {
        self.nodes[node_id].position
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn get_node_from_pixels(&self, xpixel: i32, ypixel: i32) -> Option<NodeId> {
        self.lookup.get(&(xpixel, ypixel)).copied()
    }

    pub fn get_node_from_tiles(&self, col: f32, row: f32) -> Option<NodeId> {
        let key = Self::construct_key(col, row);
        self.lookup.get(&key).copied()
    }

    pub fn set_portal_pair(&mut self, pair1: (f32, f32), pair2: (f32, f32)) {
        let Some(node1) = self.get_node_from_tiles(pair1.0, pair1.1) else {
            return;
        };
        let Some(node2) = self.get_node_from_tiles(pair2.0, pair2.1) else {
            return;
        };

        self.nodes[node1].portal = Some(node2);
        self.nodes[node2].portal = Some(node1);
    }

    pub fn create_home_nodes(&mut self, xoffset: f32, yoffset: f32) -> NodeId {
        let home = MazeData::from_rows(&[
            &['X', 'X', '+', 'X', 'X'],
            &['X', 'X', '.', 'X', 'X'],
            &['+', 'X', '.', 'X', '+'],
            &['+', '.', '+', '.', '+'],
            &['+', 'X', 'X', 'X', '+'],
        ]);

        self.create_node_table(&home, &['+'], xoffset, yoffset);
        self.connect_horizontally(&home, &['+'], &['.'], xoffset, yoffset);
        self.connect_vertically(&home, &['+'], &['.'], xoffset, yoffset);

        self.get_node_from_tiles(xoffset + 2.0, yoffset)
            .expect("home node should exist after creation")
    }

    pub fn connect_home_nodes(
        &mut self,
        home_node: NodeId,
        other_key: (f32, f32),
        direction: Direction,
    ) {
        let Some(other) = self.get_node_from_tiles(other_key.0, other_key.1) else {
            return;
        };

        link_neighbors(&mut self.nodes, home_node, direction, other);
        link_neighbors(&mut self.nodes, other, direction.opposite(), home_node);
    }

    pub fn append_renderables(&self, frame: &mut FrameData) {
        for (index, node) in self.nodes.iter().enumerate() {
            for direction in Direction::cardinals() {
                if let Some(neighbor_id) = self.neighbor(index, direction) {
                    frame.lines.push(Line {
                        start: node.position,
                        end: self.position(neighbor_id),
                        color: WHITE,
                        thickness: 4.0,
                    });
                }
            }

            if let Some(portal_id) = self.portal(index) {
                frame.lines.push(Line {
                    start: node.position,
                    end: self.position(portal_id),
                    color: WHITE,
                    thickness: 4.0,
                });
            }

            frame.circles.push(Circle {
                center: node.position,
                radius: 12.0,
                color: RED,
            });
        }
    }

    fn from_text(text: &str, node_symbols: &[char], path_symbols: &[char]) -> Self {
        let maze = MazeData::parse(text);
        let mut group = Self::default();
        group.create_node_table(&maze, node_symbols, 0.0, 0.0);
        group.connect_horizontally(&maze, node_symbols, path_symbols, 0.0, 0.0);
        group.connect_vertically(&maze, node_symbols, path_symbols, 0.0, 0.0);
        group
    }

    fn create_node_table(
        &mut self,
        data: &MazeData,
        node_symbols: &[char],
        xoffset: f32,
        yoffset: f32,
    ) {
        for row in 0..data.tiles.len() {
            for col in 0..data.width {
                if node_symbols.contains(&data.tiles[row][col]) {
                    let key = Self::construct_key(col as f32 + xoffset, row as f32 + yoffset);
                    if self.lookup.contains_key(&key) {
                        continue;
                    }

                    let id = self.nodes.len();
                    self.lookup.insert(key, id);
                    self.nodes.push(Node::new(key.0, key.1));
                }
            }
        }
    }

    fn connect_horizontally(
        &mut self,
        data: &MazeData,
        node_symbols: &[char],
        path_symbols: &[char],
        xoffset: f32,
        yoffset: f32,
    ) {
        for row in 0..data.tiles.len() {
            let mut current: Option<NodeId> = None;
            for col in 0..data.width {
                let symbol = data.tiles[row][col];
                if node_symbols.contains(&symbol) {
                    let node_id = self
                        .get_node_from_tiles(col as f32 + xoffset, row as f32 + yoffset)
                        .expect("node should exist after lookup table creation");
                    if let Some(previous) = current {
                        link_neighbors(&mut self.nodes, previous, Direction::Right, node_id);
                        link_neighbors(&mut self.nodes, node_id, Direction::Left, previous);
                    }
                    current = Some(node_id);
                } else if !path_symbols.contains(&symbol) {
                    current = None;
                }
            }
        }
    }

    fn connect_vertically(
        &mut self,
        data: &MazeData,
        node_symbols: &[char],
        path_symbols: &[char],
        xoffset: f32,
        yoffset: f32,
    ) {
        for col in 0..data.width {
            let mut current: Option<NodeId> = None;
            for row in 0..data.tiles.len() {
                let symbol = data.tiles[row][col];
                if node_symbols.contains(&symbol) {
                    let node_id = self
                        .get_node_from_tiles(col as f32 + xoffset, row as f32 + yoffset)
                        .expect("node should exist after lookup table creation");
                    if let Some(previous) = current {
                        link_neighbors(&mut self.nodes, previous, Direction::Down, node_id);
                        link_neighbors(&mut self.nodes, node_id, Direction::Up, previous);
                    }
                    current = Some(node_id);
                } else if !path_symbols.contains(&symbol) {
                    current = None;
                }
            }
        }
    }

    fn with_nodes(nodes: Vec<Node>) -> Self {
        let lookup = nodes
            .iter()
            .enumerate()
            .map(|(id, node)| ((node.position.x as i32, node.position.y as i32), id))
            .collect();

        Self { nodes, lookup }
    }

    fn construct_key(col: f32, row: f32) -> (i32, i32) {
        (
            (col * TILE_WIDTH as f32).round() as i32,
            (row * TILE_HEIGHT as f32).round() as i32,
        )
    }
}

impl MazeData {
    fn parse(text: &str) -> Self {
        let tiles: Vec<Vec<char>> = text
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                line.split_whitespace()
                    .map(|symbol| {
                        symbol
                            .chars()
                            .next()
                            .expect("maze symbols should be single characters")
                    })
                    .collect::<Vec<char>>()
            })
            .collect();

        let width = tiles
            .first()
            .map(|row| row.len())
            .expect("maze text should contain at least one row");
        assert!(tiles.iter().all(|row| row.len() == width));

        Self { tiles, width }
    }

    fn from_rows(rows: &[&[char]]) -> Self {
        let tiles = rows.iter().map(|row| row.to_vec()).collect::<Vec<_>>();
        let width = tiles
            .first()
            .map(|row| row.len())
            .expect("maze data should contain at least one row");

        Self { tiles, width }
    }
}

fn link_neighbors(nodes: &mut [Node], node_id: NodeId, direction: Direction, neighbor_id: NodeId) {
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
    fn start_node_is_the_first_maze_test_node() {
        let nodes = NodeGroup::setup_test_nodes();
        assert_eq!(nodes.position(nodes.start_node()).as_tuple(), (80.0, 80.0));
        assert_eq!(nodes.node_count(), 7);
    }

    #[test]
    fn maze_basics_layout_matches_the_downloaded_text_file() {
        let nodes = NodeGroup::maze_basics();

        assert_eq!(nodes.node_count(), 7);
        assert_eq!(nodes.position(nodes.start_node()).as_tuple(), (16.0, 16.0));
        assert_eq!(nodes.neighbor(0, Direction::Right), Some(1));
        assert_eq!(nodes.neighbor(0, Direction::Down), Some(2));
    }

    #[test]
    fn pacman_maze_matches_the_downloaded_layout() {
        let nodes = NodeGroup::pacman_maze();

        assert_eq!(nodes.node_count(), 66);
        assert_eq!(nodes.position(nodes.start_node()).as_tuple(), (16.0, 64.0));
        assert!(nodes.get_node_from_tiles(0.0, 17.0).is_some());
        assert!(nodes.get_node_from_tiles(27.0, 17.0).is_some());
    }

    #[test]
    fn portal_pairs_link_the_expected_nodes() {
        let mut nodes = NodeGroup::pacman_maze();
        nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));

        let left = nodes
            .get_node_from_tiles(0.0, 17.0)
            .expect("left portal node should exist");
        let right = nodes
            .get_node_from_tiles(27.0, 17.0)
            .expect("right portal node should exist");

        assert_eq!(nodes.portal(left), Some(right));
        assert_eq!(nodes.portal(right), Some(left));
        assert_eq!(nodes.get_node_from_pixels(0, 272), Some(left));
    }

    #[test]
    fn home_nodes_are_created_and_connected() {
        let mut nodes = NodeGroup::pacman_maze();
        let home = nodes.create_home_nodes(11.5, 14.0);
        nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
        nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);

        assert_eq!(nodes.node_count(), 74);
        assert!(nodes.get_node_from_tiles(13.5, 17.0).is_some());
        assert_eq!(
            nodes.neighbor(home, Direction::Left),
            nodes.get_node_from_tiles(12.0, 14.0)
        );
        assert_eq!(
            nodes.neighbor(home, Direction::Right),
            nodes.get_node_from_tiles(15.0, 14.0)
        );
    }
}
