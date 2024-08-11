use std::f32::consts::PI;

use bevy::{ecs::world::DeferredWorld, prelude::*};

use crate::{
    hitbox::{HitboxCollision, HitboxPurpose, KnockbackAngle},
    input::{Action, BufferedInput, Control, DirectionalAction},
    physics::{Collision, Gravity, SetVelocity, Velocity},
    utils::{FrameCount, FrameNumber},
    Airborne, AnimationIndices, AnimationTimer, Facing, PhysicsSet,
};

pub mod megaman;

// Control thresholds
const CROUCH_THRESHOLD: f32 = 0.4;

#[derive(Component)]
pub struct Player(pub usize);

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
    fn is_intangible(&self, frame: &FrameNumber) -> bool {
        match self {
            Self::Airdodge(..) => {
                &AIRDODGE_INTANGIBLE_START <= frame && frame <= &AIRDODGE_INTANGIBLE_END
            }
            _ => false,
        }
    }
    fn is_grounded(&self) -> bool {
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
    fn is_exempt_from_normal_traction(&self) -> bool {
        match self {
            Self::JumpSquat | Self::Walk | Self::Run | Self::Dash => true,
            _ => false,
        }
    }
    fn is_affected_by_gravity(&self) -> bool {
        match self {
            Self::Airdodge(..) => false,
            _ => true,
        }
    }
}

#[derive(Component, Default, Debug)]
pub struct FighterStateTransition {
    end: StateEnd,
    // faf: Option<FrameNumber>,
    iasa: Option<IASA>,
}

const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
const AIRDODGE_DURATION_FRAMES: FrameNumber = 15;
const AIRDODGE_INTANGIBLE_START: FrameNumber = 4;
const AIRDODGE_INTANGIBLE_END: FrameNumber = 15;
const TURNAROUND_DURATION_FRAMES: FrameNumber = 7;
const RUN_TURNAROUND_DURATION_FRAMES: FrameNumber = 8;
const CROUCH_TRANSITION_THRESHOLD_FRAME: FrameNumber = 6;

const DEFAULT_LAND_CROUCH_DURATION: FrameNumber = 4;
const DEFAULT_JUMP_SQUAT_DURATION: FrameNumber = 6;
const DEFAULT_DASH_DURATION: FrameNumber = 15;

impl FighterStateTransition {
    fn _default_idle_interrupt() -> StateGetter {
        |_, control, _| {
            if control.has_action(&Action::Jump) {
                return Some(FighterState::JumpSquat);
            }
            if control.stick.y < -CROUCH_THRESHOLD {
                return Some(FighterState::EnterCrouch);
            }
            None
        }
    }

    fn default_for_state(state: &FighterState) -> Self {
        match state {
            FighterState::Idle => Self {
                end: StateEnd::None,
                iasa: IASA::immediate(Self::_default_idle_interrupt()),
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
            iasa: IASA::new(frame, Self::_default_idle_interrupt()),
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
    fn idle_on_frame(frame: FrameNumber) -> Self {
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
    frame: FrameNumber,
    state_getter: StateGetter,
}

impl IASA {
    fn new(frame: FrameNumber, state_getter: StateGetter) -> Option<Self> {
        Some(IASA {
            frame,
            state_getter,
        })
    }

    fn immediate(state_getter: StateGetter) -> Option<Self> {
        Some(IASA {
            frame: 0,
            state_getter,
        })
    }
}

fn apply_state_transition(
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
            control.clear_buffers();
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

#[derive(Component)]
#[allow(dead_code)]
pub struct FighterProperties {
    walk_speed: f32,
    dash_speed: f32,
    jump_speed: f32,
    ground_friction: f32,
    gravity: f32,
    dash_duration: FrameNumber,
    land_crouch_duration: FrameNumber,
    jumpsquat_duration: FrameNumber,
}

impl Default for FighterProperties {
    fn default() -> Self {
        Self {
            walk_speed: 3.0,
            dash_speed: 5.0,
            jump_speed: 10.0,
            ground_friction: 0.3,
            gravity: -0.3,
            dash_duration: 10,
            land_crouch_duration: 6,
            jumpsquat_duration: 5,
        }
    }
}

#[derive(Event)]
pub struct FighterStateUpdate(Entity, FighterState);

fn update_fighter_state(
    mut updates: EventReader<FighterStateUpdate>,
    mut q: Query<(&mut FighterState, &mut FrameCount)>,
) {
    for update in updates.read() {
        let entity = update.0;
        if let Ok((mut state, mut frame_count)) = q.get_mut(entity) {
            let new_state = update.1;
            debug!(
                "{:?} {:?}({:?}) -> {:?}",
                entity,
                state.clone(),
                frame_count.0,
                new_state
            );
            *state = new_state;
            frame_count.0 = 0;
        } else {
            warn!("No entity found {:?}", entity);
        }
    }
}

#[derive(Component)]
pub struct JumpSpeed(pub f32);

fn apply_jump_speed(
    mut query: Query<(
        &mut Velocity,
        &FighterState,
        &FrameCount,
        &JumpSpeed,
        &Control,
    )>,
) {
    for (mut v, s, f, jump_speed, control) in query.iter_mut() {
        if s != &FighterState::JumpSquat || f.0 != DEFAULT_JUMP_SQUAT_DURATION {
            continue;
        }
        let dv = if control
            .held_actions
            .contains(Action::Jump)
        {
            // Full hop
            jump_speed.0
        } else {
            // Short hop, half as high
            0.5_f32.sqrt() * jump_speed.0
        };
        v.0.y += dv;
    }
}

fn set_airdodge_speed(mut query: Query<(&mut Velocity, &FighterState, &FrameCount)>) {
    for (mut v, s, f) in query.iter_mut() {
        let FighterState::Airdodge(direction) = s else {
            continue;
        };
        if f.0 == AIRDODGE_DURATION_FRAMES - 1 {
            v.0 = Vec2::ZERO;
        } else {
            v.0 = *direction * AIRDODGE_INITIAL_SPEED;
        }
    }
}

#[derive(Component)]
pub struct Traction(pub f32);

fn apply_traction(mut query: Query<(&mut Velocity, &Traction, &FighterState), Without<Airborne>>) {
    for (mut v, t, s) in query.iter_mut() {
        if s.is_exempt_from_normal_traction() {
            continue;
        }
        if v.0.x.abs() <= t.0 {
            v.0.x = 0.0;
        } else if v.0.x < 0.0 {
            v.0.x += t.0;
        } else {
            v.0.x -= t.0;
        }
    }
}

fn land(
    q: Query<&FighterState>,
    mut ev_collision: EventReader<Collision>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for collision in ev_collision.read() {
        if collision.normal.x != 0.0 || collision.normal.y <= 0.0 {
            continue;
        }
        let entity_id = collision.entity;
        if let Ok(state) = q.get(entity_id) {
            match state {
                FighterState::Airdodge(..) | FighterState::IdleAirborne => {
                    ev_state.send(FighterStateUpdate(entity_id, FighterState::LandCrouch));
                }
                _ => {}
            }
        }
    }
}

fn go_airborne(
    q: Query<(Entity, &FighterState), With<Airborne>>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    q.iter()
        .filter(|(_, s)| s.is_grounded())
        .map(|(e, _)| FighterStateUpdate(e, FighterState::IdleAirborne))
        .for_each(|x| {
            ev_state.send(x);
        });
}

#[derive(Component)]
pub struct Intangible;

fn remove_intangible(
    mut commands: Commands,
    query: Query<(Entity, &FighterState, &FrameCount), With<Intangible>>,
) {
    for (entity, state, frame) in query.iter() {
        if !state.is_intangible(&frame.0) {
            commands
                .entity(entity)
                .remove::<Intangible>();
        }
    }
}

fn add_intangible(
    mut commands: Commands,
    query: Query<(Entity, &FighterState, &FrameCount), Without<Intangible>>,
) {
    for (entity, state, frame) in query.iter() {
        if state.is_intangible(&frame.0) {
            commands
                .entity(entity)
                .insert(Intangible);
        }
    }
}

fn update_gravity(mut commands: Commands, q: Query<(Entity, &FighterState, &FighterProperties)>) {
    q.iter().for_each(|(e, s, p)| {
        if s.is_affected_by_gravity() {
            commands
                .entity(e)
                .insert(Gravity(p.gravity));
        } else {
            commands.entity(e).remove::<Gravity>();
        }
    })
}

#[derive(Component, Default)]
pub struct Percent(f32);

#[derive(Component)]
pub struct Weight(f32);

impl Default for Weight {
    fn default() -> Self {
        Weight(1.0)
    }
}

fn take_damage_from_hitbox_collision(
    mut q_fighter: Query<(Entity, &mut Percent, &Weight), With<FighterState>>,
    mut ev_hitbox: EventReader<HitboxCollision>,
    mut ev_set_velocity: EventWriter<SetVelocity>,
) {
    for hitbox_collision in ev_hitbox.read() {
        debug!("{:?}", hitbox_collision);
        let HitboxPurpose::Damage {
            percent,
            base_knockback,
            scale_knockback,
            angle,
        } = hitbox_collision.other_hitbox.purpose
        else {
            continue;
        };
        let Ok((fighter_entity, mut fighter_percent, weight)) =
            q_fighter.get_mut(hitbox_collision.target)
        else {
            continue;
        };
        fighter_percent.0 += percent;
        let launch_speed =
            weight.0.recip() * (base_knockback + (scale_knockback * fighter_percent.0) * 0.01);
        let launch_angle = match angle {
            // Converting CW degrees from 12 o'clock => standard form
            KnockbackAngle::Fixed(theta) => PI * 0.5 - theta.to_radians(),
        };
        let launch_velocity = Vec2::from_angle(launch_angle)
            * launch_speed
            * hitbox_collision
                .other_transform
                .scale
                .xy();
        ev_set_velocity.send(SetVelocity(fighter_entity, launch_velocity));
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum FighterEventSet {
    Act,
    React,
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct FighterSet;

pub struct FighterPlugin;
impl Plugin for FighterPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(megaman::MegaManPlugin)
            .add_systems(
                FixedUpdate,
                (
                    (
                        apply_state_transition
                            .chain()
                            .in_set(FighterEventSet::Act),
                        (
                            land,
                            go_airborne,
                            update_fighter_state,
                            apply_jump_speed,
                            update_gravity,
                            remove_intangible,
                            add_intangible,
                            take_damage_from_hitbox_collision,
                        )
                            .chain()
                            .in_set(FighterEventSet::React),
                    )
                        .chain()
                        .in_set(FighterSet),
                    set_airdodge_speed
                        .before(PhysicsSet)
                        .after(FighterEventSet::Act),
                    apply_traction.after(PhysicsSet),
                ),
            )
            .configure_sets(
                FixedUpdate,
                FighterEventSet::Act.before(FighterEventSet::React),
            )
            .add_event::<FighterStateUpdate>();
    }
}

#[derive(Bundle)]
pub struct FighterBundle {
    pub tag: Player,
    pub frame: FrameCount,
    pub facing: Facing,
    pub velocity: Velocity,
    pub state: FighterState,
    pub state_transition_properties: FighterStateTransition,
    pub properties: FighterProperties,
    pub animation_indices: AnimationIndices,
    pub animation_timer: AnimationTimer,
    pub control: Control,
    pub percent: Percent,
    pub weight: Weight,
    pub traction: Traction,
    pub jump_speed: JumpSpeed,
}
