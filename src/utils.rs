use bevy::prelude::Component;

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

#[derive(Component, Clone, Copy)]
pub struct Facing(pub LeftRight);
