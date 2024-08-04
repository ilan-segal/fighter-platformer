use super::{FighterProperties, FighterState};
use bevy::prelude::*;

use crate::{
    fighter::{FighterEventSet, FighterStateUpdate},
    hitbox::{Hitbox, HitboxBundle, HitboxGroup, HitboxGroupBundle, HitboxPurpose, Shape},
    input::{Action, Buffer},
    utils::{FrameCount, FrameNumber},
    AnimationIndices, AnimationUpdate, AnimationUpdateEvent, Velocity,
};

const LANDING_LAG: FrameNumber = 6;
const IDLE_CYCLE: FrameNumber = 240;
const JUMPSQUAT: FrameNumber = 4;
const FRICTION: f32 = 0.3;
const WALK_SPEED: f32 = 3.0;
const DASH_SPEED: f32 = 5.0;
const DASH_DURATION: FrameNumber = 20;
const GRAVITY: f32 = -0.3;

#[derive(Component)]
pub struct MegaMan;

impl MegaMan {
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
                            major_radius: 20.0,
                            minor_radius: 10.0,
                        },
                        purpose: HitboxPurpose::Body,
                    },
                    transform: TransformBundle {
                        local: Transform::from_xyz(0.0, 20.0, 1.0),
                        ..Default::default()
                    },
                });
            });
    }
}

impl FighterProperties for MegaMan {
    fn gravity(&self) -> f32 {
        GRAVITY
    }

    fn dash_duration(&self) -> FrameNumber {
        DASH_DURATION
    }

    fn dash_speed(&self) -> f32 {
        DASH_SPEED
    }

    fn walk_speed(&self) -> f32 {
        WALK_SPEED
    }

    fn land_crouch_duration(&self) -> FrameNumber {
        LANDING_LAG
    }

    fn idle_cycle_duration(&self) -> FrameNumber {
        IDLE_CYCLE
    }

    fn jumpsquat(&self) -> FrameNumber {
        JUMPSQUAT
    }

    fn jump_speed(&self) -> f32 {
        10.0
    }

    fn ground_friction(&self) -> f32 {
        FRICTION
    }

    fn animation_for_state(&self, state: &FighterState) -> Option<AnimationUpdate> {
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
            _ => None,
        }
    }
}

fn consome_action_events(
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
        _ => None,
    }
}

fn emit_animation_update(
    q: Query<(Entity, &FighterState, &FrameCount, &Velocity), With<MegaMan>>,
    mut ev_animation: EventWriter<AnimationUpdateEvent>,
) {
    for (e, state, frame, velocity) in &q {
        if let Some(update) = match (state, frame.0) {
            // Blinky blinky
            (FighterState::Idle, 200) => Some(AnimationUpdate::MultiFrame {
                indices: AnimationIndices { first: 0, last: 2 },
                seconds_per_frame: 0.1,
            }),
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

pub struct MegaManPlugin;
impl Plugin for MegaManPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy_trait_query::RegisterExt;

        app.register_component_as::<dyn FighterProperties, MegaMan>()
            .add_systems(
                FixedUpdate,
                (consome_action_events, emit_animation_update).in_set(FighterEventSet::Act),
            );
    }
}
