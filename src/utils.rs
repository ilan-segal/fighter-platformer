use bevy::prelude::*;

use crate::fighter::FighterEventSet;

pub type FrameNumber = u32;

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

#[derive(Resource, Default)]
pub struct DebugMode(pub bool);

fn check_for_debug_toggle(keys: Res<ButtonInput<KeyCode>>, mut debug_mode: ResMut<DebugMode>) {
    if keys.just_pressed(KeyCode::Backquote) {
        debug_mode.0 = !debug_mode.0;
        if debug_mode.0 {
            debug!("Debug: on");
        } else {
            debug!("Debug: off");
        }
    }
}

pub fn in_debug_mode(debug_mode: Res<DebugMode>) -> bool {
    debug_mode.0
}

pub fn not_in_debug_mode(debug_mode: Res<DebugMode>) -> bool {
    !debug_mode.0
}

#[derive(Component)]
pub struct VisibleDuringDebug;

pub fn show_debug_entities(mut query: Query<&mut Visibility, With<VisibleDuringDebug>>) {
    for mut visibility in query.iter_mut() {
        if *visibility != Visibility::Visible {
            *visibility = Visibility::Visible;
        }
    }
}

pub fn hide_debug_entities(mut query: Query<&mut Visibility, With<VisibleDuringDebug>>) {
    for mut visibility in query.iter_mut() {
        if *visibility != Visibility::Hidden {
            *visibility = Visibility::Hidden;
        }
    }
}

pub struct DebugPlugin;

impl Plugin for DebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugMode>();
        app.add_systems(
            Update,
            (
                check_for_debug_toggle,
                show_debug_entities.run_if(in_debug_mode),
                hide_debug_entities.run_if(not_in_debug_mode),
            ),
        );
    }
}

#[derive(Component)]
pub struct Lifetime(pub FrameNumber);

fn decrement_lifetime(mut commands: Commands, mut q: Query<(Entity, &mut Lifetime)>) {
    for (entity, mut lifetime) in q.iter_mut() {
        lifetime.0 -= 1;
        if lifetime.0 == 0 {
            commands
                .entity(entity)
                .despawn_recursive();
        }
    }
}

pub struct LifetimePlugin;

impl Plugin for LifetimePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            decrement_lifetime.after(FighterEventSet::React),
        );
    }
}
