use crate::{
    ghosts::Ghost,
    modes::GhostMode,
    nodes::NodeGroup,
    pacman::{BasicPacman, Direction, NodeMovementMode, NodePacman},
    pellets::PelletGroup,
    render::FrameData,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    BlankScreen,
    BasicMovement,
    Nodes,
    NodeMovement1,
    NodeMovement2,
    NodeMovement3,
    MazeBasics,
    PacmanMaze,
    Portals,
    Pellets,
    EatingPellets,
    Level3,
}

#[derive(Clone, Debug)]
pub struct Game {
    scene: Scene,
}

#[derive(Clone, Debug)]
enum Scene {
    BlankScreen,
    BasicMovement {
        pacman: BasicPacman,
    },
    Nodes {
        nodes: NodeGroup,
        pacman: BasicPacman,
    },
    NodeMovement {
        nodes: NodeGroup,
        pacman: NodePacman,
    },
    Maze {
        nodes: NodeGroup,
        pacman: NodePacman,
        pellets: Option<PelletGroup>,
        eat_pellets: bool,
    },
    Ghosts {
        nodes: NodeGroup,
        pacman: NodePacman,
        pellets: PelletGroup,
        ghost: Ghost,
    },
}

impl Game {
    pub fn new(stage: Stage) -> Self {
        let scene = match stage {
            Stage::BlankScreen => Scene::BlankScreen,
            Stage::BasicMovement => Scene::BasicMovement {
                pacman: BasicPacman::new(),
            },
            Stage::Nodes => Scene::Nodes {
                nodes: NodeGroup::setup_test_nodes(),
                pacman: BasicPacman::new(),
            },
            Stage::NodeMovement1 => {
                let nodes = NodeGroup::setup_test_nodes();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Teleport);
                Scene::NodeMovement { nodes, pacman }
            }
            Stage::NodeMovement2 => {
                let nodes = NodeGroup::setup_test_nodes();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::OvershootStop);
                Scene::NodeMovement { nodes, pacman }
            }
            Stage::NodeMovement3 => {
                let nodes = NodeGroup::setup_test_nodes();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::NodeMovement { nodes, pacman }
            }
            Stage::MazeBasics => {
                let nodes = NodeGroup::maze_basics();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: None,
                    eat_pellets: false,
                }
            }
            Stage::PacmanMaze => {
                let nodes = NodeGroup::pacman_maze();
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: None,
                    eat_pellets: false,
                }
            }
            Stage::Portals => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: None,
                    eat_pellets: false,
                }
            }
            Stage::Pellets => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: Some(PelletGroup::maze1()),
                    eat_pellets: false,
                }
            }
            Stage::EatingPellets => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                Scene::Maze {
                    nodes,
                    pacman,
                    pellets: Some(PelletGroup::maze1()),
                    eat_pellets: true,
                }
            }
            Stage::Level3 => {
                let mut nodes = NodeGroup::pacman_maze();
                nodes.set_portal_pair((0.0, 17.0), (27.0, 17.0));
                let home = nodes.create_home_nodes(11.5, 14.0);
                nodes.connect_home_nodes(home, (12.0, 14.0), Direction::Left);
                nodes.connect_home_nodes(home, (15.0, 14.0), Direction::Right);
                let pacman =
                    NodePacman::new(nodes.start_node(), &nodes, NodeMovementMode::Reversible);
                let mut ghost = Ghost::new(nodes.start_node());
                ghost.initialize_position(&nodes);
                let spawn_node = nodes
                    .get_node_from_tiles(13.5, 17.0)
                    .expect("level 3 spawn node should exist");
                ghost.set_spawn_node(spawn_node);

                Scene::Ghosts {
                    nodes,
                    pacman,
                    pellets: PelletGroup::maze1(),
                    ghost,
                }
            }
        };

        Self { scene }
    }

    pub fn update(&mut self, dt: f32, requested_direction: Direction) {
        match &mut self.scene {
            Scene::BlankScreen => {}
            Scene::BasicMovement { pacman } | Scene::Nodes { pacman, .. } => {
                pacman.update(dt, requested_direction);
            }
            Scene::NodeMovement { nodes, pacman } => pacman.update(dt, requested_direction, nodes),
            Scene::Maze {
                nodes,
                pacman,
                pellets,
                eat_pellets,
            } => {
                pacman.update(dt, requested_direction, nodes);
                if let Some(pellets) = pellets {
                    pellets.update(dt);
                    if *eat_pellets {
                        pellets.try_eat(pacman.position(), pacman.collide_radius());
                    }
                }
            }
            Scene::Ghosts {
                nodes,
                pacman,
                pellets,
                ghost,
            } => {
                pacman.update(dt, requested_direction, nodes);
                ghost.update(dt, nodes, pacman.position());
                pellets.update(dt);

                if let Some(pellet) = pellets.try_eat(pacman.position(), pacman.collide_radius())
                    && pellet.kind() == crate::pellets::PelletKind::PowerPellet
                {
                    ghost.start_freight();
                }

                if pacman.collide_check(ghost.position(), ghost.collide_radius())
                    && ghost.mode() == GhostMode::Freight
                {
                    ghost.start_spawn(nodes);
                }
            }
        }
    }

    pub fn frame(&self) -> FrameData {
        let mut frame = FrameData::default();

        match &self.scene {
            Scene::BlankScreen => {}
            Scene::BasicMovement { pacman } => frame.circles.push(pacman.renderable()),
            Scene::Nodes { nodes, pacman } => {
                nodes.append_renderables(&mut frame);
                frame.circles.push(pacman.renderable());
            }
            Scene::NodeMovement { nodes, pacman } => {
                nodes.append_renderables(&mut frame);
                frame.circles.push(pacman.renderable());
            }
            Scene::Maze {
                nodes,
                pacman,
                pellets,
                ..
            } => {
                nodes.append_renderables(&mut frame);
                if let Some(pellets) = pellets {
                    pellets.append_renderables(&mut frame);
                }
                frame.circles.push(pacman.renderable());
            }
            Scene::Ghosts {
                nodes,
                pacman,
                pellets,
                ghost,
            } => {
                nodes.append_renderables(&mut frame);
                pellets.append_renderables(&mut frame);
                frame.circles.push(pacman.renderable());
                frame.circles.push(ghost.renderable());
            }
        }

        frame
    }
}

#[cfg(test)]
mod tests {
    use super::{Game, Stage};
    use crate::pacman::Direction;

    #[test]
    fn nodes_stage_renders_graph_and_pacman() {
        let game = Game::new(Stage::Nodes);
        let frame = game.frame();

        assert!(!frame.lines.is_empty());
        assert_eq!(frame.circles.len(), 8);
    }

    #[test]
    fn reversible_stage_updates_without_panicking() {
        let mut game = Game::new(Stage::NodeMovement3);
        game.update(0.2, Direction::Right);
        game.update(0.2, Direction::Left);

        let frame = game.frame();
        assert_eq!(frame.circles.len(), 8);
    }

    #[test]
    fn pellets_stage_renders_nodes_pellets_and_pacman() {
        let game = Game::new(Stage::Pellets);
        let frame = game.frame();

        assert_eq!(frame.circles.len(), 313);
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn eating_pellets_stage_consumes_the_starting_pellet() {
        let mut game = Game::new(Stage::EatingPellets);
        let before = game.frame().circles.len();

        game.update(0.0, Direction::Stop);
        let after = game.frame().circles.len();

        assert_eq!(before, 313);
        assert_eq!(after, 312);
    }

    #[test]
    fn level3_stage_renders_ghosts_pellets_and_pacman() {
        let game = Game::new(Stage::Level3);
        let frame = game.frame();

        assert_eq!(frame.circles.len(), 322);
        assert!(!frame.lines.is_empty());
    }

    #[test]
    fn level3_stage_updates_without_panicking() {
        let mut game = Game::new(Stage::Level3);

        game.update(0.1, Direction::Right);
        game.update(0.1, Direction::Stop);

        let frame = game.frame();
        assert!(frame.circles.len() >= 315);
    }
}
