use super::{FighterState, FighterStateMachine};
use bevy::prelude::*;

use crate::{
    fighter::FighterEventSet,
    utils::{FrameCount, FrameNumber},
    AnimationIndices, AnimationUpdate, AnimationUpdateEvent, Velocity,
};

const LANDING_LAG: FrameNumber = 12;
const IDLE_CYCLE: FrameNumber = 240;
const JUMPSQUAT: FrameNumber = 5;
const FRICTION: f32 = 0.3;
const WALK_SPEED: f32 = 3.0;
const DASH_SPEED: f32 = 5.0;
const DASH_DURATION: FrameNumber = 20;

#[derive(Component)]
pub struct MegaMan;

impl FighterStateMachine for MegaMan {
    fn dash_duration(&self) -> FrameNumber {
        DASH_DURATION
    }

    fn dash_speed(&self) -> f32 {
        DASH_SPEED
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
}

// pub fn compute_side_effects(
//     query: Query<(Entity, &FighterState, &FrameCount, &Facing), With<MegaMan>>,
//     mut ev_state: EventWriter<FighterStateUpdate>,
//     mut ev_facing: EventWriter<FacingUpdate>,
// ) {
//     for (entity, state, frame, facing) in query.iter() {}
// }

fn emit_animation_update(
    q: Query<(Entity, &FighterState, &FrameCount, &Velocity), With<MegaMan>>,
    mut ev_animation: EventWriter<AnimationUpdateEvent>,
) {
    for (e, state, frame, velocity) in &q {
        if let Some(update) = match (state, frame.0) {
            (FighterState::Idle, 1) => Some(AnimationUpdate::SingleFrame(0)),
            // Blinky blinky
            (FighterState::Idle, 200) => Some(AnimationUpdate::MultiFrame {
                indices: AnimationIndices { first: 0, last: 2 },
                seconds_per_frame: 0.1,
            }),
            (FighterState::LandCrouch, 1) => Some(AnimationUpdate::SingleFrame(133)),
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
            (FighterState::JumpSquat, 1) => Some(AnimationUpdate::SingleFrame(133)),
            (FighterState::Walk, 1) => Some(AnimationUpdate::MultiFrame {
                indices: AnimationIndices { first: 5, last: 14 },
                seconds_per_frame: 0.1,
            }),
            (FighterState::Airdodge, 1) => Some(AnimationUpdate::SingleFrame(33)),
            (FighterState::Dash, 1) => Some(AnimationUpdate::SingleFrame(24)),
            (FighterState::Turnaround, 1) => Some(AnimationUpdate::SingleFrame(74)),
            (FighterState::RunTurnaround, 1) => Some(AnimationUpdate::SingleFrame(30)),
            (FighterState::Run, 1) => Some(AnimationUpdate::MultiFrame {
                indices: AnimationIndices { first: 5, last: 14 },
                seconds_per_frame: 0.1,
            }),
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

        app.register_component_as::<dyn FighterStateMachine, MegaMan>()
            .add_systems(
                FixedUpdate,
                emit_animation_update.in_set(FighterEventSet::Emit),
            );
    }
}
