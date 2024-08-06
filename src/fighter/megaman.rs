use super::{FighterProperties, FighterState};
use bevy::prelude::*;

use crate::{
    fighter::{FighterEventSet, FighterStateUpdate},
    hitbox::{Hitbox, HitboxBundle, HitboxGroup, HitboxGroupBundle, HitboxPurpose, Shape},
    input::{Action, Buffer},
    utils::{FrameCount, FrameNumber},
    AnimationIndices, AnimationUpdate, AnimationUpdateEvent, Velocity,
};

const ATTACK_DURATION: FrameNumber = 20;
const ATTACK_EMIT_LEMON: FrameNumber = 10;

#[derive(Component)]
pub struct MegaMan;

impl MegaMan {
    pub fn get_properties() -> FighterProperties {
        FighterProperties {
            walk_speed: 3.0,
            dash_speed: 5.0,
            jump_speed: 10.0,
            ground_friction: 0.3,
            gravity: -0.3,
            dash_duration: 10,
            land_crouch_duration: 6,
            jumpsquat_duration: 4,
            ..Default::default()
        }
    }

    pub fn spawn_body_hitboxes(child_builder: &mut ChildBuilder) {
        child_builder
            .spawn(HitboxGroupBundle {
                hitbox_group: HitboxGroup::default(),
                transform: TransformBundle::default(),
            })
            .with_children(|hitbox_group| {
                hitbox_group.spawn(HitboxBundle {
                    hitbox: Hitbox {
                        shape: Shape::Pill {
                            major_radius: 6.0,
                            minor_radius: 15.5,
                        },
                        purpose: HitboxPurpose::Body,
                    },
                    transform: TransformBundle {
                        local: Transform::from_xyz(-1.0, 20.75, 1.0),
                        ..Default::default()
                    },
                });
            });
    }
}

fn update_state_for_frame_count(
    q: Query<(Entity, &FighterState, &FrameCount), With<MegaMan>>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for (e, state, FrameCount(frame)) in q.iter() {
        match (state, *frame) {
            (FighterState::Attack, ATTACK_DURATION) => {
                ev_state.send(FighterStateUpdate(e, FighterState::Idle));
            }
            _ => {}
        }
    }
}

fn consume_action_events(
    q: Query<(Entity, &FighterState, &Buffer), With<MegaMan>>,
    mut ev_state: EventWriter<FighterStateUpdate>,
    mut commands: Commands,
) {
    for (e, state, buffer) in q.iter() {
        debug!("{:?}", buffer);
        if let Some(new_state) = get_action_transition(state, &buffer.action) {
            ev_state.send(FighterStateUpdate(e, new_state));
            commands.entity(e).remove::<Buffer>();
        }
    }
}

fn get_action_transition(state: &FighterState, action: &Action) -> Option<FighterState> {
    match (state, action) {
        (FighterState::Idle, Action::Jump)
        | (FighterState::Crouch, Action::Jump)
        | (FighterState::EnterCrouch, Action::Jump)
        | (FighterState::ExitCrouch, Action::Jump)
        | (FighterState::Walk, Action::Jump)
        | (FighterState::Turnaround, Action::Jump)
        | (FighterState::Dash, Action::Jump)
        | (FighterState::Run, Action::Jump) => Some(FighterState::JumpSquat),
        (FighterState::IdleAirborne, Action::Shield) => Some(FighterState::Airdodge),
        (FighterState::JumpSquat, Action::Shield) => Some(FighterState::Airdodge),
        (FighterState::Idle, Action::Attack) => Some(FighterState::Attack),
        _ => None,
    }
}

fn emit_animation_update(
    q: Query<(Entity, &FighterState, &FrameCount, &Velocity), With<MegaMan>>,
    mut ev_animation: EventWriter<AnimationUpdateEvent>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for (e, state, frame, velocity) in &q {
        if let Some(basic_update) = animation_for_state(state).map(|u| AnimationUpdateEvent(e, u)) {
            ev_animation.send(basic_update);
            continue;
        }
        if let Some(update) = match (state, frame.0) {
            // Blinky blinky
            (FighterState::Idle, 200) => Some(AnimationUpdate::MultiFrame {
                indices: AnimationIndices { first: 0, last: 2 },
                seconds_per_frame: 0.1,
            }),
            (FighterState::Idle, 240) => {
                ev_state.send(FighterStateUpdate(e, FighterState::Idle));
                None
            }
            (FighterState::IdleAirborne, _) => {
                let y = velocity.0.y;
                if y > 1.5 {
                    Some(AnimationUpdate::SingleFrame(18))
                } else if y > -1.5 {
                    Some(AnimationUpdate::SingleFrame(19))
                } else {
                    Some(AnimationUpdate::MultiFrame {
                        indices: AnimationIndices {
                            first: 20,
                            last: 21,
                        },
                        seconds_per_frame: 0.15,
                    })
                }
            }
            _ => None,
        } {
            let event = AnimationUpdateEvent(e, update);
            ev_animation.send(event);
        }
    }
}

fn animation_for_state(state: &FighterState) -> Option<AnimationUpdate> {
    match state {
        FighterState::Idle => Some(AnimationUpdate::SingleFrame(0)),
        FighterState::JumpSquat
        | FighterState::LandCrouch
        | FighterState::EnterCrouch
        | FighterState::ExitCrouch => Some(AnimationUpdate::SingleFrame(133)),
        FighterState::Crouch => Some(AnimationUpdate::SingleFrame(134)),
        FighterState::Walk => Some(AnimationUpdate::MultiFrame {
            indices: AnimationIndices { first: 5, last: 14 },
            seconds_per_frame: 0.1,
        }),
        FighterState::Airdodge => Some(AnimationUpdate::SingleFrame(33)),
        FighterState::Dash => Some(AnimationUpdate::SingleFrame(24)),
        FighterState::Turnaround => Some(AnimationUpdate::SingleFrame(74)),
        FighterState::RunTurnaround => Some(AnimationUpdate::SingleFrame(30)),
        FighterState::Run => Some(AnimationUpdate::MultiFrame {
            indices: AnimationIndices { first: 5, last: 14 },
            seconds_per_frame: 0.1,
        }),
        FighterState::Attack => Some(AnimationUpdate::SingleFrame(43)),
        _ => None,
    }
}

pub struct MegaManPlugin;
impl Plugin for MegaManPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            (update_state_for_frame_count, consume_action_events, emit_animation_update)
                .chain()
                .in_set(FighterEventSet::Act),
        );
    }
}
