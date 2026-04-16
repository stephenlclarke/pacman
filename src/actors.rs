//! Defines shared actor and ghost identity types that the gameplay systems use to coordinate entity-specific behavior.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Pacman,
    Blinky,
    Pinky,
    Inky,
    Clyde,
    Fruit,
}

impl EntityKind {
    pub const fn all() -> [Self; 6] {
        [
            Self::Pacman,
            Self::Blinky,
            Self::Pinky,
            Self::Inky,
            Self::Clyde,
            Self::Fruit,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GhostKind {
    Blinky,
    Pinky,
    Inky,
    Clyde,
}

impl GhostKind {
    pub const ALL: [Self; 4] = [Self::Blinky, Self::Pinky, Self::Inky, Self::Clyde];

    pub const fn entity(self) -> EntityKind {
        match self {
            Self::Blinky => EntityKind::Blinky,
            Self::Pinky => EntityKind::Pinky,
            Self::Inky => EntityKind::Inky,
            Self::Clyde => EntityKind::Clyde,
        }
    }

    pub const fn index(self) -> usize {
        match self {
            Self::Blinky => 0,
            Self::Pinky => 1,
            Self::Inky => 2,
            Self::Clyde => 3,
        }
    }
}
