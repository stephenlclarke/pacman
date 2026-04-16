//! Builds and queries the maze node graph that Pac-Man, ghosts, and the autopilot traverse.

use std::collections::{HashMap, HashSet};

use crate::{
    actors::EntityKind,
    constants::{RED, TILE_HEIGHT, TILE_WIDTH, WHITE},
    pacman::Direction,
    render::{Circle, FrameData, Line},
    vector::Vector2,
};

#[cfg(test)]
const MAZE_ONE: &str = include_str!("../assets/arcade/maze-logic.txt");

pub type NodeId = usize;

#[derive(Clone, Debug)]
pub struct Node {
    position: Vector2,
    neighbors: [Option<NodeId>; 4],
    portal: Option<NodeId>,
    access: [HashSet<EntityKind>; 4],
}

#[derive(Clone, Debug, Default)]
pub struct NodeGroup {
    nodes: Vec<Node>,
    lookup: HashMap<(i32, i32), NodeId>,
    home_node: Option<NodeId>,
}

#[derive(Clone, Debug)]
struct MazeData {
    tiles: Vec<Vec<char>>,
    width: usize,
}

impl Node {
    /// Creates new.
    fn new(x: i32, y: i32) -> Self {
        Self {
            position: Vector2::new(x as f32, y as f32),
            neighbors: [None; 4],
            portal: None,
            access: std::array::from_fn(|_| EntityKind::all().into_iter().collect()),
        }
    }

    /// Denies access.
    fn deny_access(&mut self, direction: Direction, entity: EntityKind) {
        let Some(index) = direction.neighbor_index() else {
            return;
        };
        self.access[index].remove(&entity);
    }

    /// Allows access.
    fn allow_access(&mut self, direction: Direction, entity: EntityKind) {
        let Some(index) = direction.neighbor_index() else {
            return;
        };
        self.access[index].insert(entity);
    }

    /// Handles access.
    fn can_access(&self, direction: Direction, entity: EntityKind) -> bool {
        let Some(index) = direction.neighbor_index() else {
            return false;
        };

        self.neighbors[index].is_some() && self.access[index].contains(&entity)
    }
}

impl NodeGroup {
    #[cfg(test)]
    /// Handles maze.
    pub fn pacman_maze() -> Self {
        Self::from_pacman_layout(MAZE_ONE)
    }

    /// Handles pacman layout.
    pub fn from_pacman_layout(text: &str) -> Self {
        Self::from_text(text, &['+', 'P', 'n'], &['.', '-', '|', 'p'])
    }

    /// Starts node.
    pub fn start_node(&self) -> NodeId {
        0
    }

    /// Handles neighbor.
    pub fn neighbor(&self, node_id: NodeId, direction: Direction) -> Option<NodeId> {
        let index = direction.neighbor_index()?;
        self.nodes
            .get(node_id)
            .and_then(|node| node.neighbors[index])
    }

    /// Handles travel.
    pub fn can_travel(&self, node_id: NodeId, direction: Direction, entity: EntityKind) -> bool {
        self.nodes
            .get(node_id)
            .is_some_and(|node| node.can_access(direction, entity))
    }

    /// Handles portal.
    pub fn portal(&self, node_id: NodeId) -> Option<NodeId> {
        self.nodes.get(node_id).and_then(|node| node.portal)
    }

    /// Handles position.
    pub fn position(&self, node_id: NodeId) -> Vector2 {
        self.nodes[node_id].position
    }

    #[cfg(test)]
    /// Handles count.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Handles ids.
    pub fn node_ids(&self) -> impl Iterator<Item = NodeId> + '_ {
        0..self.nodes.len()
    }

    #[cfg(test)]
    /// Gets node from pixels.
    pub fn get_node_from_pixels(&self, xpixel: i32, ypixel: i32) -> Option<NodeId> {
        self.lookup.get(&(xpixel, ypixel)).copied()
    }

    /// Gets node from tiles.
    pub fn get_node_from_tiles(&self, col: f32, row: f32) -> Option<NodeId> {
        let key = Self::construct_key(col, row);
        self.lookup.get(&key).copied()
    }

    /// Sets portal pair.
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

    /// Handles home nodes.
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

        let home_node = self
            .get_node_from_tiles(xoffset + 2.0, yoffset)
            .expect("home node should exist after creation");
        self.home_node = Some(home_node);
        home_node
    }

    /// Handles home nodes.
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

    /// Denies access.
    pub fn deny_access(&mut self, col: f32, row: f32, direction: Direction, entity: EntityKind) {
        let Some(node) = self.get_node_from_tiles(col, row) else {
            return;
        };
        self.nodes[node].deny_access(direction, entity);
    }

    /// Allows access.
    pub fn allow_access(&mut self, col: f32, row: f32, direction: Direction, entity: EntityKind) {
        let Some(node) = self.get_node_from_tiles(col, row) else {
            return;
        };
        self.nodes[node].allow_access(direction, entity);
    }

    /// Denies access list.
    pub fn deny_access_list<I>(&mut self, col: f32, row: f32, direction: Direction, entities: I)
    where
        I: IntoIterator<Item = EntityKind>,
    {
        // Iterate through each item in the current collection or range.
        for entity in entities {
            self.deny_access(col, row, direction, entity);
        }
    }

    /// Allows access list.
    pub fn allow_access_list<I>(&mut self, col: f32, row: f32, direction: Direction, entities: I)
    where
        I: IntoIterator<Item = EntityKind>,
    {
        // Iterate through each item in the current collection or range.
        for entity in entities {
            self.allow_access(col, row, direction, entity);
        }
    }

    /// Denies home access.
    pub fn deny_home_access(&mut self, entity: EntityKind) {
        // Branch based on the current runtime condition.
        if let Some(home_node) = self.home_node {
            self.nodes[home_node].deny_access(Direction::Down, entity);
        }
    }

    /// Allows home access.
    pub fn allow_home_access(&mut self, entity: EntityKind) {
        // Branch based on the current runtime condition.
        if let Some(home_node) = self.home_node {
            self.nodes[home_node].allow_access(Direction::Down, entity);
        }
    }

    /// Denies home access list.
    pub fn deny_home_access_list<I>(&mut self, entities: I)
    where
        I: IntoIterator<Item = EntityKind>,
    {
        // Iterate through each item in the current collection or range.
        for entity in entities {
            self.deny_home_access(entity);
        }
    }

    /// Appends renderables.
    pub fn append_renderables(&self, frame: &mut FrameData) {
        // Iterate through each item in the current collection or range.
        for (index, node) in self.nodes.iter().enumerate() {
            // Iterate through each item in the current collection or range.
            for direction in Direction::cardinals() {
                // Branch based on the current runtime condition.
                if let Some(neighbor_id) = self.neighbor(index, direction) {
                    frame.lines.push(Line {
                        start: node.position,
                        end: self.position(neighbor_id),
                        color: WHITE,
                        thickness: 4.0,
                    });
                }
            }

            // Branch based on the current runtime condition.
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

    /// Handles text.
    fn from_text(text: &str, node_symbols: &[char], path_symbols: &[char]) -> Self {
        let maze = MazeData::parse(text);
        let mut group = Self::default();
        group.create_node_table(&maze, node_symbols, 0.0, 0.0);
        group.connect_horizontally(&maze, node_symbols, path_symbols, 0.0, 0.0);
        group.connect_vertically(&maze, node_symbols, path_symbols, 0.0, 0.0);
        group
    }

    /// Handles node table.
    fn create_node_table(
        &mut self,
        data: &MazeData,
        node_symbols: &[char],
        xoffset: f32,
        yoffset: f32,
    ) {
        // Iterate through each item in the current collection or range.
        for row in 0..data.tiles.len() {
            // Iterate through each item in the current collection or range.
            for col in 0..data.width {
                // Branch based on the current runtime condition.
                if node_symbols.contains(&data.tiles[row][col]) {
                    let key = Self::construct_key(col as f32 + xoffset, row as f32 + yoffset);
                    // Branch based on the current runtime condition.
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

    /// Handles horizontally.
    fn connect_horizontally(
        &mut self,
        data: &MazeData,
        node_symbols: &[char],
        path_symbols: &[char],
        xoffset: f32,
        yoffset: f32,
    ) {
        // Iterate through each item in the current collection or range.
        for row in 0..data.tiles.len() {
            let mut current: Option<NodeId> = None;
            // Iterate through each item in the current collection or range.
            for col in 0..data.width {
                let symbol = data.tiles[row][col];
                // Branch based on the current runtime condition.
                if node_symbols.contains(&symbol) {
                    let node_id = self
                        .get_node_from_tiles(col as f32 + xoffset, row as f32 + yoffset)
                        .expect("node should exist after lookup table creation");
                    // Branch based on the current runtime condition.
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

    /// Handles vertically.
    fn connect_vertically(
        &mut self,
        data: &MazeData,
        node_symbols: &[char],
        path_symbols: &[char],
        xoffset: f32,
        yoffset: f32,
    ) {
        // Iterate through each item in the current collection or range.
        for col in 0..data.width {
            let mut current: Option<NodeId> = None;
            // Iterate through each item in the current collection or range.
            for row in 0..data.tiles.len() {
                let symbol = data.tiles[row][col];
                // Branch based on the current runtime condition.
                if node_symbols.contains(&symbol) {
                    let node_id = self
                        .get_node_from_tiles(col as f32 + xoffset, row as f32 + yoffset)
                        .expect("node should exist after lookup table creation");
                    // Branch based on the current runtime condition.
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

    /// Handles key.
    fn construct_key(col: f32, row: f32) -> (i32, i32) {
        (
            (col * TILE_WIDTH as f32).round() as i32,
            (row * TILE_HEIGHT as f32).round() as i32,
        )
    }
}

impl MazeData {
    /// Parses parse.
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

    /// Handles rows.
    fn from_rows(rows: &[&[char]]) -> Self {
        let tiles = rows.iter().map(|row| row.to_vec()).collect::<Vec<_>>();
        let width = tiles
            .first()
            .map(|row| row.len())
            .expect("maze data should contain at least one row");

        Self { tiles, width }
    }
}

/// Handles neighbors.
fn link_neighbors(nodes: &mut [Node], node_id: NodeId, direction: Direction, neighbor_id: NodeId) {
    let index = direction
        .neighbor_index()
        .expect("cardinal directions should have a neighbor slot");
    nodes[node_id].neighbors[index] = Some(neighbor_id);
}

#[cfg(test)]
mod tests {
    use super::NodeGroup;
    use crate::{actors::EntityKind, pacman::Direction};

    #[test]
    /// Handles maze matches the arcade logic layout.
    fn pacman_maze_matches_the_arcade_logic_layout() {
        let nodes = NodeGroup::pacman_maze();

        assert_eq!(nodes.node_count(), 66);
        assert_eq!(nodes.position(nodes.start_node()).as_tuple(), (16.0, 64.0));
        assert!(nodes.get_node_from_tiles(0.0, 17.0).is_some());
        assert!(nodes.get_node_from_tiles(27.0, 17.0).is_some());
    }

    #[test]
    /// Handles pairs link the expected nodes.
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
    /// Handles nodes are created and connected.
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

    #[test]
    /// Handles access blocks travel for the requested entity only.
    fn denied_access_blocks_travel_for_the_requested_entity_only() {
        let mut nodes = NodeGroup::pacman_maze();
        let home = nodes.create_home_nodes(11.5, 14.0);
        nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
        nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);
        nodes.deny_home_access(EntityKind::Pacman);

        assert!(!nodes.can_travel(home, Direction::Down, EntityKind::Pacman));
        assert!(nodes.can_travel(home, Direction::Down, EntityKind::Blinky));
    }
}
