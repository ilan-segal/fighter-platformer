use bevy::{ecs::world::DeferredWorld, prelude::*};

use crate::{
    input::{Action, BufferedInput, Control, DirectionalAction, RotationDirection},
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
    Moonwalk,
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
            | Self::Moonwalk
            | Self::Crouch
            | Self::EnterCrouch
            | Self::ExitCrouch => true,
            _ => false,
        }
    }
    pub fn is_exempt_from_normal_traction(&self) -> bool {
        match self {
            Self::JumpSquat | Self::Walk | Self::Run | Self::Dash | Self::Moonwalk => true,
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

fn try_dash(data: &InterruptPlayerData) -> Option<FighterState> {
    let can_dash_same_direction = match data.state {
        FighterState::Dash
        | FighterState::Run
        | FighterState::RunTurnaround
        | FighterState::RunEnd => false,
        _ => true,
    };
    if let BufferedInput::Some { value, .. } = data.control.directional_action
        && let DirectionalAction::Smash(direction) = value
        && let Some(horizontal) = direction.horizontal()
        && (horizontal != data.component::<Facing>().unwrap().0 || can_dash_same_direction)
    {
        return Some(FighterState::Dash);
    } else {
        return None;
    }
}

fn try_moonwalk(data: &InterruptPlayerData) -> Option<FighterState> {
    if let BufferedInput::Some { value, .. } = data.control.directional_action
        && let DirectionalAction::HalfCircle(direction, rotation) = value
    {
        return match (direction, rotation) {
            (CardinalDirection::Left, RotationDirection::Clockwise)
            | (CardinalDirection::Right, RotationDirection::CounterClockwise) => {
                Some(FighterState::Moonwalk)
            }
            _ => None,
        };
    } else {
        return None;
    }
}

fn try_jump(data: &InterruptPlayerData) -> Option<FighterState> {
    if data.control.has_action(&Action::Jump) {
        Some(FighterState::JumpSquat)
    } else {
        None
    }
}

fn try_turnaround(data: &InterruptPlayerData) -> Option<FighterState> {
    if data.state == &FighterState::Turnaround {
        return None;
    }
    let facing = data
        .component::<Facing>()
        .expect("Player facing");
    if let Some(direction) = data
        .control
        .stick
        .get_cardinal_direction()
        && direction == facing.0.flip()
    {
        Some(FighterState::Turnaround)
    } else {
        None
    }
}

fn try_run_turnaround(data: &InterruptPlayerData) -> Option<FighterState> {
    if data.state == &FighterState::RunTurnaround {
        return None;
    }
    let facing = data
        .component::<Facing>()
        .expect("Player facing");
    if let Some(direction) = data
        .control
        .stick
        .get_cardinal_direction()
        && direction == facing.0.flip()
    {
        Some(FighterState::RunTurnaround)
    } else {
        None
    }
}

fn try_walk(data: &InterruptPlayerData) -> Option<FighterState> {
    if data.state == &FighterState::Walk {
        return None;
    }
    let facing = data
        .component::<Facing>()
        .expect("Player facing");
    if let Some(direction) = data
        .control
        .stick
        .get_cardinal_direction()
        && direction == facing.0
    {
        Some(FighterState::Walk)
    } else {
        None
    }
}

fn try_crouch(data: &InterruptPlayerData) -> Option<FighterState> {
    if let Some(direction) = data
        .control
        .stick
        .get_cardinal_direction()
        && direction == CardinalDirection::Down
        && data.control.stick.y < -CROUCH_THRESHOLD
    {
        Some(FighterState::EnterCrouch)
    } else {
        None
    }
}

fn try_end_crouch(data: &InterruptPlayerData) -> Option<FighterState> {
    if data.control.stick.y >= -CROUCH_THRESHOLD {
        Some(FighterState::ExitCrouch)
    } else {
        None
    }
}

fn try_end_run(data: &InterruptPlayerData) -> Option<FighterState> {
    if data
        .control
        .stick
        .get_cardinal_direction()
        .and_then(|d| d.horizontal())
        .is_some()
    {
        return None;
    }
    return match data.state {
        FighterState::Run => Some(FighterState::RunEnd),
        FighterState::RunEnd => Some(FighterState::Idle),
        _ => None,
    };
}

fn try_end_walk(data: &InterruptPlayerData) -> Option<FighterState> {
    if data
        .control
        .stick
        .get_cardinal_direction()
        .filter(|d| d.is_horizontal())
        .is_none()
    {
        return Some(FighterState::Idle);
    }
    return None;
}

fn try_airdodge(data: &InterruptPlayerData) -> Option<FighterState> {
    if data.control.has_action(&Action::Shield) {
        Some(FighterState::Airdodge(
            data.control.stick.normalize_or_zero(),
        ))
    } else {
        None
    }
}

fn try_attack(data: &InterruptPlayerData) -> Option<FighterState> {
    if data.control.has_action(&Action::Attack) {
        Some(FighterState::Attack)
    } else {
        None
    }
}

impl FighterStateTransition {
    pub fn default_idle_interrupt() -> StateGetter {
        |data| {
            try_dash(data)
                .or_else(|| try_attack(data))
                .or_else(|| try_jump(data))
                .or_else(|| try_turnaround(data))
                .or_else(|| try_walk(data))
                .or_else(|| try_crouch(data))
        }
    }

    pub fn default_run_interrupt() -> StateGetter {
        |data| {
            try_jump(data)
                .or_else(|| try_crouch(data))
                .or_else(|| try_run_turnaround(data))
                .or_else(|| try_end_run(data))
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
                iasa: IASA::immediate(|data| {
                    Self::default_idle_interrupt()(data).or_else(|| try_end_walk(data))
                }),
            },

            FighterState::Turnaround => Self {
                end: StateEnd::OnFrame {
                    frame: TURNAROUND_DURATION_FRAMES,
                    next_state: FighterState::Idle,
                },
                iasa: IASA::immediate(|data| try_dash(data).or_else(|| try_jump(data))),
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
                iasa: IASA::immediate(|data| try_jump(data).or_else(|| try_end_crouch(data))),
            },

            FighterState::ExitCrouch => Self {
                end: StateEnd::idle_on_frame(CROUCH_TRANSITION_THRESHOLD_FRAME),
                ..Default::default()
            },

            FighterState::LandCrouch => Self::idle_on_frame(DEFAULT_LAND_CROUCH_DURATION),

            FighterState::JumpSquat => Self {
                iasa: IASA::immediate(try_airdodge),
                ..Default::default()
            },

            FighterState::IdleAirborne => Self {
                iasa: IASA::immediate(try_airdodge),
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
                iasa: IASA::immediate(|data| {
                    try_jump(data)
                        .or_else(|| try_moonwalk(data))
                        .or_else(|| try_dash(data))
                }),
            },

            FighterState::Moonwalk => Self {
                end: StateEnd::OnFrame {
                    frame: DEFAULT_DASH_DURATION,
                    next_state: FighterState::Idle,
                },
                iasa: IASA::immediate(|data| try_jump(data).or_else(|| try_moonwalk(data))),
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

pub struct InterruptPlayerData<'a> {
    pub control: &'a Control,
    pub state: &'a FighterState,
    pub entity: &'a Entity,
    pub world: &'a DeferredWorld<'a>,
}

impl<'a> InterruptPlayerData<'a> {
    pub fn component<C: Component>(&self) -> Option<&'a C> {
        self.world.get::<C>(*self.entity)
    }
}

type StateGetter = fn(&InterruptPlayerData) -> Option<FighterState>;

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
            .and_then(|iasa| {
                let data = InterruptPlayerData {
                    control: control.as_ref(),
                    state: state.as_ref(),
                    entity: &entity,
                    world: &world,
                };
                return (iasa.state_getter)(&data);
            })
        {
            debug!(
                "Interrupted {:?} on frame {:?} => {:?}",
                *state, frame_number, new_state
            );
            *state = new_state;
            state_frame.0 = 0;
            // control.clear_buffers();
            match new_state {
                FighterState::Dash | FighterState::Moonwalk => {
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
