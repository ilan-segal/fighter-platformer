use bevy::prelude::Component;

pub type FrameNumber = u32;

#[derive(Component)]
pub struct FrameCount(pub FrameNumber);

pub enum LeftRight {
    Left,
    Right,
}
