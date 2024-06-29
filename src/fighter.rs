use bevy::{
    app::{FixedUpdate, Plugin},
    prelude::*,
};
use bevy_trait_query::One;

use crate::{
    input::ControlStick,
    physics::{AddVelocity, Collision, Gravity, Position, SetVelocity, Velocity, MAX_FLOOR_SLOPE},
    utils::{FrameCount, FrameNumber},
    Airborne, AnimationIndices, AnimationTimer, Facing,
};

pub mod megaman;

const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
const AIRDODGE_DURATION_FRAMES: FrameNumber = 30;
const AIRDODGE_INTANGIBLE_START: FrameNumber = 4;
const AIRDODGE_INTANGIBLE_END: FrameNumber = 20;
const TURNAROUND_DURATION_FRAMES: FrameNumber = 14;
const TURNAROUND_THRESHOLD_FRAME: FrameNumber = TURNAROUND_DURATION_FRAMES / 2;
const RUN_TURNAROUND_DURATION_FRAMES: FrameNumber = 14;
const RUN_TURNAROUND_THRESHOLD_FRAME: FrameNumber = TURNAROUND_DURATION_FRAMES / 2;

#[derive(Component)]
pub struct Player(pub usize);

#[derive(Component, Clone, Copy, Default, Debug)]
pub enum FighterState {
    #[default]
    Idle,
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
            | Self::Run => true,
            _ => false,
        }
    }
}

#[bevy_trait_query::queryable]
pub trait FighterStateMachine {
    fn dash_duration(&self) -> FrameNumber;
    fn dash_speed(&self) -> f32;
    fn land_crouch_duration(&self) -> FrameNumber;
    fn idle_cycle_duration(&self) -> FrameNumber;
    fn jumpsquat(&self) -> FrameNumber;
    fn jump_speed(&self) -> f32;
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
            log::info!(
                "{:?} {:?}({:?}) -> {:?}",
                entity,
                state.clone(),
                frame_count.0,
                new_state
            );
            *state = new_state;
            frame_count.0 = 0;
        } else {
            log::warn!("No entity found {:?}", entity);
        }
    }
}

fn compute_common_side_effects(
    query: Query<(
        Entity,
        &FighterState,
        &FrameCount,
        &Facing,
        One<&dyn FighterStateMachine>,
        &ControlStick,
        &Velocity,
    )>,
    mut ev_state: EventWriter<FighterStateUpdate>,
    mut ev_facing: EventWriter<FacingUpdate>,
    mut ev_add_velocity: EventWriter<AddVelocity>,
    mut ev_set_velocity: EventWriter<SetVelocity>,
) {
    for (entity, state, frame, facing, sm, control_stick, v) in &query {
        // log::info!("Getting side effects for {:?}", entity);
        // Implementation-specific stuff
        match state {
            FighterState::LandCrouch if frame.0 == sm.land_crouch_duration() => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
            }
            FighterState::Idle if frame.0 == sm.idle_cycle_duration() => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
            }
            FighterState::Dash if frame.0 == sm.dash_duration() => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Run));
            }
            FighterState::JumpSquat if frame.0 == sm.jumpsquat() => {
                ev_add_velocity.send(AddVelocity(entity, Vec2::new(0.0, sm.jump_speed())));
            }
            _ => {}
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
            (FighterState::Airdodge, AIRDODGE_DURATION_FRAMES) => {
                ev_state.send(FighterStateUpdate(entity, FighterState::IdleAirborne));
            }
            (FighterState::RunEnd, 2) => {
                ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
            }
            (FighterState::Airdodge, 0) => {
                let control = if control_stick.0.length() > crate::CONTROL_STICK_DEADZONE {
                    control_stick.0.normalize_or_zero()
                } else {
                    Vec2::ZERO
                };
                ev_set_velocity.send(SetVelocity(entity, control * AIRDODGE_INITIAL_SPEED));
            }
            (FighterState::Airdodge, _) if v.0.length() > 0.0 => {
                let speed_reduction_per_frame =
                    AIRDODGE_INITIAL_SPEED / (AIRDODGE_DURATION_FRAMES as f32);
                if v.0.length() <= speed_reduction_per_frame {
                    ev_set_velocity.send(SetVelocity(entity, Vec2::ZERO));
                } else {
                    ev_add_velocity.send(AddVelocity(
                        entity,
                        -v.0.normalize() * speed_reduction_per_frame,
                    ));
                }
            }
            (FighterState::Dash, 0) => {
                let dv_x = control_stick.0.x.signum() * sm.dash_speed();
                ev_set_velocity.send(SetVelocity(entity, Vec2::new(0.0, dv_x)));
            }
            _ => {}
        }
    }
}

fn land(
    q: Query<(&FighterState, &Velocity)>,
    mut ev_collision: EventReader<Collision>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for collision in ev_collision.read() {
        let entity_id = collision.entity;
        if let Ok((state, velocity)) = q.get(entity_id) {
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
    // for (e, s) in &q {
    //     if s.is_grounded() {
    //         ev_state.send(FighterStateUpdate(e, FighterState::IdleAirborne));
    //     }
    // }
}

#[derive(Component)]
pub struct Intangible;

#[derive(Event)]
pub struct IntangibleUpdate(Entity, bool);

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

pub struct FighterPlugin;
impl Plugin for FighterPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_plugins(megaman::MegaManPlugin)
            .add_systems(
                FixedUpdate,
                (
                    land,
                    go_airborne,
                    remove_intangible,
                    add_intangible,
                    update_facing,
                    update_fighter_state,
                    compute_common_side_effects,
                )
                    .chain()
                    .before(crate::update_frame_count),
            )
            .add_event::<FighterStateUpdate>()
            .add_event::<IntangibleUpdate>()
            .add_event::<FacingUpdate>();
    }
}

#[derive(Bundle)]
pub struct FighterBundle {
    pub tag: Player,
    pub frame: FrameCount,
    pub facing: Facing,
    pub position: Position,
    pub velocity: Velocity,
    pub gravity: Gravity,
    pub state: FighterState,
    pub sprite_sheet_bundle: SpriteSheetBundle,
    pub animation_indices: AnimationIndices,
    pub animation_timer: AnimationTimer,
    pub control_stick: ControlStick,
}
