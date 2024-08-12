use bevy::{ecs::world::DeferredWorld, prelude::*};

use crate::{
    input::{Action, BufferedInput, Control, DirectionalAction},
    utils::{CardinalDirection, Directed, Facing, FrameCount, FrameNumber},
};

use crate::fighter::CROUCH_THRESHOLD;

#[derive(Component, Clone, Copy, Default, Debug, PartialEq)]
pub enum FighterState {
    #[default]
    Idle,
    Crouch,
    EnterCrouch,
    ExitCrouch,
    Turnaround,
    RunTurnaround,
    LandCrouch,
    IdleAirborne,
    JumpSquat,
    Walk,
    Dash,
    Run,
    // Ensures that the player cannot Dash out of a Run by going Run -> Idle -> Dash
    RunEnd,
    Airdodge(Vec2),
    Attack,
}

impl FighterState {
    pub fn is_intangible(&self, frame: &FrameNumber) -> bool {
        match self {
            Self::Airdodge(..) => {
                &AIRDODGE_INTANGIBLE_START <= frame && frame <= &AIRDODGE_INTANGIBLE_END
            }
            _ => false,
        }
    }
    pub fn is_grounded(&self) -> bool {
        match self {
            Self::Idle
            | Self::LandCrouch
            | Self::JumpSquat
            | Self::Walk
            | Self::Turnaround
            | Self::RunTurnaround
            | Self::RunEnd
            | Self::Dash
            | Self::Run
            | Self::Crouch
            | Self::EnterCrouch
            | Self::ExitCrouch => true,
            _ => false,
        }
    }
    pub fn is_exempt_from_normal_traction(&self) -> bool {
        match self {
            Self::JumpSquat | Self::Walk | Self::Run | Self::Dash => true,
            _ => false,
        }
    }
    pub fn is_affected_by_gravity(&self) -> bool {
        match self {
            Self::Airdodge(..) => false,
            _ => true,
        }
    }
}

#[derive(Component, Default, Debug)]
pub struct FighterStateTransition {
    pub end: StateEnd,
    // faf: Option<FrameNumber>,
    pub iasa: Option<IASA>,
}

pub const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
pub const AIRDODGE_DURATION_FRAMES: FrameNumber = 15;
pub const AIRDODGE_INTANGIBLE_START: FrameNumber = 4;
pub const AIRDODGE_INTANGIBLE_END: FrameNumber = 15;
pub const TURNAROUND_DURATION_FRAMES: FrameNumber = 8;
pub const RUN_TURNAROUND_DURATION_FRAMES: FrameNumber = 8;
pub const CROUCH_TRANSITION_THRESHOLD_FRAME: FrameNumber = 6;

pub const DEFAULT_LAND_CROUCH_DURATION: FrameNumber = 6;
pub const DEFAULT_JUMP_SQUAT_DURATION: FrameNumber = 6;
pub const DEFAULT_DASH_DURATION: FrameNumber = 15;

impl FighterStateTransition {
    pub fn default_idle_interrupt() -> StateGetter {
        |entity, control, world| {
            if let BufferedInput::Some { value, .. } = control.directional_action
                && let DirectionalAction::Smash(direction) = value
                && direction.horizontal().is_some()
            {
                return Some(FighterState::Dash);
            }
            if control.has_action(&Action::Jump) {
                return Some(FighterState::JumpSquat);
            }
            let facing = world
                .get::<Facing>(*entity)
                .expect("Fighter's facing state");
            let stick = control.stick;
            let stick_direction = stick.get_cardinal_direction();
            if let Some(d) = stick_direction
                && d == facing.0.flip()
            {
                return Some(FighterState::Turnaround);
            } else if let Some(d) = stick_direction
                && d == facing.0
            {
                return Some(FighterState::Walk);
            }
            if stick_direction == Some(CardinalDirection::Down) && stick.y < -CROUCH_THRESHOLD {
                return Some(FighterState::EnterCrouch);
            }
            return None;
        }
    }

    pub fn default_run_interrupt() -> StateGetter {
        |entity, control, world| {
            if control.has_action(&Action::Jump) {
                return Some(FighterState::JumpSquat);
            }
            let stick = control.stick;
            let stick_direction = stick.get_cardinal_direction();
            if stick_direction == Some(CardinalDirection::Down) && stick.y < -CROUCH_THRESHOLD {
                return Some(FighterState::EnterCrouch);
            }
            let state = world
                .get::<FighterState>(*entity)
                .expect("Fighter state");
            if let Some(left_right) = stick_direction.and_then(|d| d.horizontal()) {
                let facing = world.get::<Facing>(*entity).unwrap();
                if facing.0 != left_right && state != &FighterState::RunTurnaround {
                    return Some(FighterState::RunTurnaround);
                } else {
                    return None;
                }
            } else {
                return match state {
                    FighterState::Run => Some(FighterState::RunEnd),
                    FighterState::RunEnd => Some(FighterState::Idle),
                    _ => None,
                };
            }
        }
    }

    pub fn default_for_state(state: &FighterState) -> Self {
        match state {
            FighterState::Idle => Self {
                end: StateEnd::None,
                iasa: IASA::immediate(Self::default_idle_interrupt()),
            },

            FighterState::Walk => Self {
                end: StateEnd::None,
                iasa: IASA::immediate(|entity, control, world| {
                    let result = Self::default_idle_interrupt()(entity, control, world);
                    if result == Some(FighterState::Walk) {
                        return None;
                    } else if control
                        .stick
                        .get_cardinal_direction()
                        .filter(|d| d.is_horizontal())
                        .is_none()
                    {
                        return Some(FighterState::Idle);
                    }
                    return result;
                }),
            },

            FighterState::Turnaround => Self {
                end: StateEnd::OnFrame {
                    frame: TURNAROUND_DURATION_FRAMES,
                    next_state: FighterState::Idle,
                },
                iasa: IASA::immediate(|_, control, _| {
                    if let BufferedInput::Some { value, .. } = control.directional_action
                        && let DirectionalAction::Smash(direction) = value
                        && direction.horizontal().is_some()
                    {
                        return Some(FighterState::Dash);
                    }
                    if control.has_action(&Action::Jump) {
                        return Some(FighterState::JumpSquat);
                    }
                    return None;
                }),
            },

            FighterState::Run => Self {
                end: StateEnd::None,
                iasa: IASA::immediate(Self::default_run_interrupt()),
            },

            FighterState::RunEnd => Self {
                end: StateEnd::OnFrame {
                    frame: 1,
                    next_state: FighterState::RunTurnaround,
                },
                iasa: IASA::immediate(Self::default_run_interrupt()),
            },

            FighterState::RunTurnaround => Self {
                end: StateEnd::OnFrame {
                    frame: RUN_TURNAROUND_DURATION_FRAMES,
                    next_state: FighterState::Run,
                },
                iasa: IASA::immediate(Self::default_run_interrupt()),
            },

            FighterState::EnterCrouch => Self {
                end: StateEnd::OnFrame {
                    frame: CROUCH_TRANSITION_THRESHOLD_FRAME,
                    next_state: FighterState::Crouch,
                },
                ..Default::default()
            },

            FighterState::Crouch => Self {
                end: StateEnd::None,
                iasa: IASA::immediate(|_, control, _| {
                    if control.has_action(&Action::Jump) {
                        Some(FighterState::JumpSquat)
                    } else if control.stick.y > -CROUCH_THRESHOLD {
                        Some(FighterState::ExitCrouch)
                    } else {
                        None
                    }
                }),
            },

            FighterState::ExitCrouch => Self {
                end: StateEnd::idle_on_frame(CROUCH_TRANSITION_THRESHOLD_FRAME),
                ..Default::default()
            },

            FighterState::LandCrouch => Self::idle_on_frame(DEFAULT_LAND_CROUCH_DURATION),

            FighterState::JumpSquat => Self {
                iasa: IASA::immediate(|_, control, _| {
                    if control.has_action(&Action::Shield) {
                        Some(FighterState::Airdodge(control.stick.normalize_or_zero()))
                    } else {
                        None
                    }
                }),
                ..Default::default()
            },

            FighterState::IdleAirborne => Self {
                iasa: IASA::immediate(|_, control, _| {
                    if control.has_action(&Action::Shield) {
                        Some(FighterState::Airdodge(control.stick.normalize_or_zero()))
                    } else {
                        None
                    }
                }),
                ..Default::default()
            },

            FighterState::Airdodge(..) => Self {
                end: StateEnd::OnFrame {
                    frame: AIRDODGE_DURATION_FRAMES,
                    next_state: FighterState::IdleAirborne,
                },
                ..Default::default()
            },

            FighterState::Dash => Self {
                end: StateEnd::OnFrame {
                    frame: DEFAULT_DASH_DURATION,
                    next_state: FighterState::Run,
                },
                iasa: Some(IASA {
                    frame: 0,
                    state_getter: |entity, control, world| {
                        // Dash -> Jump
                        if let BufferedInput::Some { value, .. } = control.action
                            && value == Action::Jump
                        {
                            return Some(FighterState::JumpSquat);
                        }
                        let player_facing = world
                            .get::<Facing>(*entity)
                            .expect("Player facing");
                        // Dash -> Dash (in the other direction)
                        if let BufferedInput::Some { value, .. } = control.directional_action
                            && let DirectionalAction::Smash(cardinal) = value
                            && let Some(input_facing) = cardinal.horizontal()
                            && input_facing != player_facing.0
                        {
                            return Some(FighterState::Dash);
                        }
                        return None;
                        // TODO: Dash attacks and stuff
                    },
                }),
            },

            _ => Self::default(),
        }
    }

    fn idle_on_frame(frame: FrameNumber) -> Self {
        Self {
            end: StateEnd::idle_on_frame(frame),
            iasa: IASA::new(frame, Self::default_idle_interrupt()),
        }
    }
}

#[derive(Default, Debug)]
pub enum StateEnd {
    #[default]
    None,
    OnFrame {
        frame: FrameNumber,
        next_state: FighterState,
    },
}

impl StateEnd {
    pub fn idle_on_frame(frame: FrameNumber) -> Self {
        StateEnd::OnFrame {
            frame,
            next_state: FighterState::Idle,
        }
    }
}

type StateGetter = fn(&Entity, &Control, &DeferredWorld) -> Option<FighterState>;

// Interruptible As Soon As
#[derive(Debug)]
pub struct IASA {
    pub frame: FrameNumber,
    pub state_getter: StateGetter,
}

impl IASA {
    pub fn new(frame: FrameNumber, state_getter: StateGetter) -> Option<Self> {
        Some(IASA {
            frame,
            state_getter,
        })
    }

    pub fn immediate(state_getter: StateGetter) -> Option<Self> {
        Some(IASA {
            frame: 0,
            state_getter,
        })
    }
}

pub fn apply_state_transition(
    mut q: Query<(
        &FighterStateTransition,
        &mut FrameCount,
        &mut FighterState,
        Entity,
        &mut Control,
    )>,
    world: DeferredWorld,
) {
    for (props, mut state_frame, mut state, entity, mut control) in q.iter_mut() {
        let frame_number = state_frame.0;

        // Compute input for IASA condition
        if let Some(new_state) = props
            .iasa
            .as_ref()
            .filter(|iasa| iasa.frame <= frame_number)
            .and_then(|iasa| (iasa.state_getter)(&entity, control.as_ref(), &world))
        {
            debug!(
                "Interrupted {:?} on frame {:?} => {:?}",
                *state, frame_number, new_state
            );
            *state = new_state;
            state_frame.0 = 0;
            // control.clear_buffers();
            match new_state {
                FighterState::Dash => {
                    control.directional_action = BufferedInput::None;
                }
                FighterState::Airdodge(..) => {
                    control.directional_action = BufferedInput::None;
                    control.action = BufferedInput::None;
                }
                _ => {
                    control.action = BufferedInput::None;
                }
            }
        }
        // Compute natural state end
        else if let StateEnd::OnFrame { frame, next_state } = props.end
            && frame <= frame_number
        {
            debug!(
                "{:?} ran out on frame {:?} => {:?}",
                *state, frame_number, next_state
            );
            *state = next_state;
            state_frame.0 = 0;
        }
    }
}
