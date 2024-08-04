use bevy::prelude::*;
use bevy_trait_query::One;

use crate::{
    input::{Action, Control, DirectionalAction, DirectionalActionType},
    physics::{AddVelocity, Collision, Gravity, SetVelocity, Velocity},
    utils::{CardinalDirection, Directed, FrameCount, FrameNumber, LeftRight},
    AccelerateTowards, Airborne, AnimationIndices, AnimationTimer, AnimationUpdate,
    AnimationUpdateEvent, Facing,
};

pub mod megaman;

const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
const AIRDODGE_DURATION_FRAMES: FrameNumber = 15;
const AIRDODGE_INTANGIBLE_START: FrameNumber = 4;
const AIRDODGE_INTANGIBLE_END: FrameNumber = 15;
const TURNAROUND_DURATION_FRAMES: FrameNumber = 7;
const TURNAROUND_THRESHOLD_FRAME: FrameNumber = TURNAROUND_DURATION_FRAMES / 2;
const RUN_TURNAROUND_DURATION_FRAMES: FrameNumber = 8;
const RUN_TURNAROUND_THRESHOLD_FRAME: FrameNumber = TURNAROUND_DURATION_FRAMES / 2;
const CROUCH_TRANSITION_THRESHOLD_FRAME: FrameNumber = 6;

// Control thresholds
const CROUCH_THRESHOLD: f32 = 0.4;

#[derive(Component)]
pub struct Player(pub usize);

#[derive(Component, Clone, Copy, Default, Debug, PartialEq, Eq)]
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
    Airdodge,
}

impl FighterState {
    fn is_intangible(&self, frame: &FrameNumber) -> bool {
        match self {
            Self::Airdodge => {
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
    fn has_neutral_friction(&self) -> bool {
        match self {
            Self::Idle | Self::LandCrouch | Self::Crouch | Self::EnterCrouch | Self::ExitCrouch => {
                true
            }
            _ => false,
        }
    }
    fn is_affected_by_gravity(&self) -> bool {
        match self {
            Self::Airdodge => false,
            _ => true,
        }
    }
}

#[bevy_trait_query::queryable]
pub trait FighterProperties {
    fn dash_duration(&self) -> FrameNumber;
    fn dash_speed(&self) -> f32;
    fn walk_speed(&self) -> f32;
    fn land_crouch_duration(&self) -> FrameNumber;
    fn idle_cycle_duration(&self) -> FrameNumber;
    fn jumpsquat(&self) -> FrameNumber;
    fn jump_speed(&self) -> f32;
    fn ground_friction(&self) -> f32;
    fn gravity(&self) -> f32;
    fn turnaround_duration(&self) -> FrameNumber {
        TURNAROUND_DURATION_FRAMES
    }
    fn turnaround_threshold(&self) -> FrameNumber {
        TURNAROUND_THRESHOLD_FRAME
    }
    fn run_turnaround_duration(&self) -> FrameNumber {
        RUN_TURNAROUND_DURATION_FRAMES
    }
    fn run_turnaround_threshold(&self) -> FrameNumber {
        RUN_TURNAROUND_THRESHOLD_FRAME
    }
    fn airdodge_duration(&self) -> FrameNumber {
        AIRDODGE_DURATION_FRAMES
    }
    /* Basic per-state animation, doesn't account for things like velocity or frame number */
    fn animation_for_state(&self, state: &FighterState) -> Option<AnimationUpdate>;
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

fn get_animation_from_state(
    q: Query<(
        Entity,
        &FighterState,
        One<&dyn FighterProperties>,
        &FrameCount,
    )>,
    mut ev_animation: EventWriter<AnimationUpdateEvent>,
) {
    for (e, state, properties, frame) in &q {
        if frame.0 != 1 {
            continue;
        }
        if let Some(event) = properties
            .animation_for_state(state)
            .map(|update| AnimationUpdateEvent(e, update))
        {
            ev_animation.send(event);
        }
    }
}

fn compute_common_side_effects(
    query: Query<(
        Entity,
        &FighterState,
        &FrameCount,
        &Facing,
        One<&dyn FighterProperties>,
        &Control,
        Option<&DirectionalAction>,
    )>,
    mut ev_state: EventWriter<FighterStateUpdate>,
    mut ev_facing: EventWriter<FacingUpdate>,
    mut ev_accelerate: EventWriter<AccelerateTowards>,
    mut ev_add_velocity: EventWriter<AddVelocity>,
    mut ev_set_velocity: EventWriter<SetVelocity>,
    mut commands: Commands,
) {
    for (entity, state, frame, facing, properties, control, directional_action) in &query {
        // Implementation-specific stuff
        match state {
            FighterState::LandCrouch if frame.0 == properties.land_crouch_duration() => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
            }
            FighterState::Idle if frame.0 == properties.idle_cycle_duration() => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
            }
            FighterState::Dash if frame.0 == properties.dash_duration() => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Run));
            }
            FighterState::JumpSquat if frame.0 == properties.jumpsquat() => {
                if control
                    .held_actions
                    .contains(Action::Shield)
                {
                    ev_state.send(FighterStateUpdate(entity, FighterState::Airdodge));
                    return;
                }
                let jump_speed = if control
                    .held_actions
                    .contains(Action::Jump)
                {
                    properties.jump_speed()
                } else {
                    // Short-hop, half the max-height of a full-hop
                    properties.jump_speed() * 0.5_f32.sqrt()
                };
                ev_add_velocity.send(AddVelocity(entity, Vec2::new(0.0, jump_speed)));
            }
            FighterState::Idle | FighterState::LandCrouch
                if control.stick.y < -CROUCH_THRESHOLD =>
            {
                ev_state.send(FighterStateUpdate(entity, FighterState::EnterCrouch));
            }
            FighterState::Crouch if control.stick.y >= -CROUCH_THRESHOLD => {
                ev_state.send(FighterStateUpdate(entity, FighterState::ExitCrouch));
            }
            FighterState::Idle
            | FighterState::Turnaround
            | FighterState::Walk
            | FighterState::Dash
                if let Some(d) = directional_action
                    && d.action_type == DirectionalActionType::Smash
                    && d.direction.is_sideways() =>
            {
                debug!("{:?}", d);
                let direction = d.direction.get_sideways_direction();
                if direction == facing.0 && *state == FighterState::Dash {
                    return;
                }
                ev_state.send(FighterStateUpdate(entity, FighterState::Dash));
                ev_facing.send(FacingUpdate(entity, Facing(direction)));
                commands
                    .entity(entity)
                    .remove::<DirectionalAction>();
            }
            FighterState::Idle if control.stick.x.abs() > 0.1 => {
                let control_direction = control.stick.get_sideways_direction();
                if control_direction == facing.0 {
                    ev_state.send(FighterStateUpdate(entity, FighterState::Walk));
                } else {
                    ev_state.send(FighterStateUpdate(entity, FighterState::Turnaround));
                }
            }
            FighterState::Walk => {
                if control.stick.x == 0.0 {
                    ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
                } else if control.stick.get_sideways_direction() != facing.0 {
                    ev_state.send(FighterStateUpdate(entity, FighterState::Turnaround));
                } else {
                    let target = Vec2::new(control.stick.x, 0.0) * properties.walk_speed();
                    ev_accelerate.send(AccelerateTowards {
                        entity,
                        target,
                        acceleration: properties.ground_friction(),
                    });
                }
            }
            FighterState::Run => {
                if control.stick.x == 0.0 {
                    ev_state.send(FighterStateUpdate(entity, FighterState::RunEnd));
                } else if control.stick.get_cardinal_direction() == CardinalDirection::Down {
                    ev_state.send(FighterStateUpdate(entity, FighterState::Crouch));
                } else if control.stick.get_sideways_direction() != facing.0 {
                    ev_state.send(FighterStateUpdate(entity, FighterState::RunTurnaround));
                } else {
                    let target =
                        Vec2::new(control.stick.x, 0.0).normalize() * properties.dash_speed();
                    ev_accelerate.send(AccelerateTowards {
                        entity,
                        target,
                        acceleration: properties.ground_friction(),
                    });
                }
            }
            _ => {}
        }
        if state.is_grounded() && state.has_neutral_friction() {
            ev_accelerate.send(AccelerateTowards {
                entity,
                target: Vec2::ZERO,
                acceleration: properties.ground_friction(),
            });
        }
        // Global stuff
        match (state, frame.0) {
            (FighterState::Turnaround, TURNAROUND_DURATION_FRAMES) => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
            }
            (FighterState::Turnaround, TURNAROUND_THRESHOLD_FRAME) => {
                ev_facing.send(FacingUpdate(entity, Facing(facing.0.flip())));
            }
            (FighterState::RunTurnaround, RUN_TURNAROUND_DURATION_FRAMES) => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Run));
            }
            (FighterState::RunTurnaround, RUN_TURNAROUND_THRESHOLD_FRAME) => {
                ev_facing.send(FacingUpdate(entity, Facing(facing.0.flip())));
            }
            (FighterState::RunEnd, 1) => {
                let new_state = if control.stick.x != 0.0
                    && control.stick.get_sideways_direction() != facing.0
                {
                    FighterState::RunTurnaround
                } else {
                    FighterState::Idle
                };
                ev_state.send(FighterStateUpdate(entity, new_state));
            }
            (FighterState::Airdodge, 1) => {
                let control = control.stick.normalize_or_zero();
                ev_set_velocity.send(SetVelocity(entity, control * AIRDODGE_INITIAL_SPEED));
            }
            (FighterState::Airdodge, AIRDODGE_DURATION_FRAMES) => {
                ev_set_velocity.send(SetVelocity(entity, Vec2::ZERO));
                ev_state.send(FighterStateUpdate(entity, FighterState::IdleAirborne));
            }
            (FighterState::Dash, 1) => {
                let sign = if facing.0 == LeftRight::Left {
                    -1.0
                } else {
                    1.0
                };
                let dv_x = sign * properties.dash_speed();
                ev_set_velocity.send(SetVelocity(entity, Vec2::new(dv_x, 0.0)));
            }
            (FighterState::EnterCrouch, CROUCH_TRANSITION_THRESHOLD_FRAME) => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Crouch));
            }
            (FighterState::ExitCrouch, CROUCH_TRANSITION_THRESHOLD_FRAME) => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
            }
            _ => {}
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
                FighterState::Airdodge | FighterState::IdleAirborne => {
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

#[derive(Event, Clone, Copy)]
pub struct FacingUpdate(Entity, Facing);

fn update_facing(mut updates: EventReader<FacingUpdate>, mut commands: Commands) {
    for update in updates.read() {
        commands
            .entity(update.0)
            .insert(update.1);
    }
}

fn update_gravity(
    mut commands: Commands,
    q: Query<(Entity, &FighterState, One<&dyn FighterProperties>)>,
) {
    q.iter().for_each(|(e, s, p)| {
        if s.is_affected_by_gravity() {
            commands
                .entity(e)
                .insert(Gravity(p.gravity()));
        } else {
            commands.entity(e).remove::<Gravity>();
        }
    })
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
                    compute_common_side_effects.in_set(FighterEventSet::Act),
                    (
                        land,
                        go_airborne,
                        update_fighter_state,
                        update_gravity,
                        update_facing,
                        remove_intangible,
                        add_intangible,
                        get_animation_from_state,
                    )
                        .chain()
                        .in_set(FighterEventSet::React),
                )
                    .chain()
                    .in_set(FighterSet),
            )
            .configure_sets(
                FixedUpdate,
                FighterEventSet::Act.before(FighterEventSet::React),
            )
            .add_event::<FighterStateUpdate>()
            .add_event::<FacingUpdate>();
    }
}

#[derive(Bundle)]
pub struct FighterBundle {
    pub tag: Player,
    pub frame: FrameCount,
    pub facing: Facing,
    pub velocity: Velocity,
    pub state: FighterState,
    pub sprite_sheet_bundle: SpriteSheetBundle,
    pub animation_indices: AnimationIndices,
    pub animation_timer: AnimationTimer,
    pub control: Control,
}
