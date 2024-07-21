use bevy::prelude::{Component, Vec2};

pub type FrameNumber = usize;

#[derive(Component)]
pub struct FrameCount(pub FrameNumber);

#[derive(PartialEq, Eq, Default, Clone, Copy)]
pub enum LeftRight {
    Left,
    #[default]
    Right,
}

impl LeftRight {
    pub fn flip(&self) -> Self {
        match self {
            LeftRight::Left => LeftRight::Right,
            LeftRight::Right => LeftRight::Left,
        }
    }
}

#[derive(PartialEq, Eq)]
pub enum CardinalDirection {
    Up,
    Right,
    Left,
    Down,
}

pub trait Directed {
    fn get_cardinal_direction(&self) -> CardinalDirection;
    fn get_sideways_direction(&self) -> LeftRight;
    fn is_sideways(&self) -> bool {
        match self.get_cardinal_direction() {
            CardinalDirection::Left | CardinalDirection::Right => true,
            _ => false,
        }
    }
}

impl Directed for Vec2 {
    fn get_cardinal_direction(&self) -> CardinalDirection {
        if self.y > self.x.abs() {
            if self.y > 0.0 {
                CardinalDirection::Up
            } else {
                CardinalDirection::Down
            }
        } else if self.x > 0.0 {
            CardinalDirection::Right
        } else {
            CardinalDirection::Left
        }
    }

    fn get_sideways_direction(&self) -> LeftRight {
        if self.x > 0.0 {
            LeftRight::Right
        } else {
            LeftRight::Left
        }
    }
}

#[derive(Component, Clone, Copy)]
pub struct Facing(pub LeftRight);
