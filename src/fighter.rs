use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{
    hitbox::{HitboxCollision, HitboxPurpose, KnockbackAngle},
    input::{Action, Control, DirectionalAction, DirectionalActionType},
    physics::{AddVelocity, Collision, Gravity, SetVelocity, Velocity},
    utils::{CardinalDirection, Directed, FrameCount, FrameNumber, LeftRight},
    AccelerateTowards, Airborne, AnimationIndices, AnimationTimer, Facing,
};

pub mod megaman;

const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
const AIRDODGE_DURATION_FRAMES: FrameNumber = 15;
const AIRDODGE_INTANGIBLE_START: FrameNumber = 4;
const AIRDODGE_INTANGIBLE_END: FrameNumber = 15;
const TURNAROUND_DURATION_FRAMES: FrameNumber = 7;
const RUN_TURNAROUND_DURATION_FRAMES: FrameNumber = 8;
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
    Attack,
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

#[derive(Component)]
pub struct FighterProperties {
    walk_speed: f32,
    dash_speed: f32,
    jump_speed: f32,
    ground_friction: f32,
    gravity: f32,
    dash_duration: FrameNumber,
    land_crouch_duration: FrameNumber,
    jumpsquat_duration: FrameNumber,
    turnaround_duration: FrameNumber,
    run_turnaround_duration: FrameNumber,
    airdodge_duration: FrameNumber,
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
            turnaround_duration: TURNAROUND_DURATION_FRAMES,
            run_turnaround_duration: RUN_TURNAROUND_DURATION_FRAMES,
            airdodge_duration: AIRDODGE_DURATION_FRAMES,
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

fn transition_from_land_crouch(
    q: Query<(&FighterState, &FighterProperties, &FrameCount, Entity)>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for (state, props, FrameCount(frame), entity) in q.iter() {
        if state != &FighterState::LandCrouch {
            continue;
        }
        if frame != &props.land_crouch_duration {
            continue;
        }
        ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
    }
}

fn transition_from_dash(
    q: Query<(&FighterState, &FighterProperties, &FrameCount, Entity)>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for (state, props, FrameCount(frame), entity) in q.iter() {
        if state != &FighterState::Dash {
            continue;
        }
        if frame != &props.dash_duration {
            continue;
        }
        ev_state.send(FighterStateUpdate(entity, FighterState::Run));
    }
}

fn transition_from_turnaround(
    mut q: Query<(
        &FighterState,
        &FighterProperties,
        &FrameCount,
        &mut Facing,
        Entity,
    )>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for (state, props, FrameCount(frame), mut facing, entity) in q.iter_mut() {
        if state != &FighterState::Turnaround {
            continue;
        }
        if frame == &props.turnaround_duration {
            ev_state.send(FighterStateUpdate(entity, FighterState::Idle));
        } else if *frame == props.turnaround_duration / 2 {
            facing.0 = facing.0.flip();
        }
    }
}

fn transition_from_run_turnaround(
    mut q: Query<(
        &FighterState,
        &FighterProperties,
        &FrameCount,
        &mut Facing,
        Entity,
    )>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for (state, props, FrameCount(frame), mut facing, entity) in q.iter_mut() {
        if state != &FighterState::RunTurnaround {
            continue;
        }
        if frame == &props.run_turnaround_duration {
            ev_state.send(FighterStateUpdate(entity, FighterState::Run));
        } else if *frame == props.run_turnaround_duration / 2 {
            facing.0 = facing.0.flip();
        }
    }
}

fn transition_from_airdodge(
    q: Query<(&FighterState, &FighterProperties, &FrameCount, Entity)>,
    mut ev_state: EventWriter<FighterStateUpdate>,
    mut ev_set_velocity: EventWriter<SetVelocity>,
) {
    for (state, props, FrameCount(frame), entity) in q.iter() {
        if state != &FighterState::Airdodge {
            continue;
        }
        if frame != &props.airdodge_duration {
            continue;
        }
        ev_set_velocity.send(SetVelocity(entity, Vec2::ZERO));
        ev_state.send(FighterStateUpdate(entity, FighterState::IdleAirborne));
    }
}

fn transition_from_jumpsquat(
    q: Query<(
        Entity,
        &FighterState,
        &FighterProperties,
        &FrameCount,
        &Control,
    )>,
    mut ev_state: EventWriter<FighterStateUpdate>,
    mut ev_add_velocity: EventWriter<AddVelocity>,
    mut ev_set_velocity: EventWriter<SetVelocity>,
) {
    for (entity, state, props, FrameCount(frame), control) in q.iter() {
        if state != &FighterState::JumpSquat {
            continue;
        }
        if frame != &props.jumpsquat_duration {
            continue;
        }
        if control
            .held_actions
            .contains(Action::Shield)
        {
            ev_state.send(FighterStateUpdate(entity, FighterState::LandCrouch));
            ev_set_velocity.send(SetVelocity(
                entity,
                control.stick.normalize_or_zero() * AIRDODGE_INITIAL_SPEED,
            ));
            continue;
        }
        let jump_speed = if control
            .held_actions
            .contains(Action::Jump)
        {
            props.jump_speed
        } else {
            // Short-hop, half the max-height of a full-hop
            props.jump_speed * 0.5_f32.sqrt()
        };
        ev_add_velocity.send(AddVelocity(entity, Vec2::new(0.0, jump_speed)));
    }
}

fn compute_common_side_effects(
    mut query: Query<(
        Entity,
        &FighterState,
        &FrameCount,
        &mut Facing,
        &FighterProperties,
        &Control,
        Option<&DirectionalAction>,
    )>,
    mut ev_state: EventWriter<FighterStateUpdate>,
    mut ev_accelerate: EventWriter<AccelerateTowards>,
    mut ev_set_velocity: EventWriter<SetVelocity>,
    mut commands: Commands,
) {
    for (entity, state, frame, mut facing, properties, control, directional_action) in
        query.iter_mut()
    {
        // Implementation-specific stuff
        match state {
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
                facing.0 = direction;
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
                    let target = Vec2::new(control.stick.x, 0.0) * properties.walk_speed;
                    ev_accelerate.send(AccelerateTowards {
                        entity,
                        target,
                        acceleration: properties.ground_friction,
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
                        Vec2::new(control.stick.x, 0.0).normalize() * properties.dash_speed;
                    ev_accelerate.send(AccelerateTowards {
                        entity,
                        target,
                        acceleration: properties.ground_friction,
                    });
                }
            }
            _ => {}
        }
        if state.is_grounded() && state.has_neutral_friction() {
            ev_accelerate.send(AccelerateTowards {
                entity,
                target: Vec2::ZERO,
                acceleration: properties.ground_friction,
            });
        }
        // Global stuff
        match (state, frame.0) {
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
            (FighterState::Dash, 1) => {
                let sign = if facing.0 == LeftRight::Left {
                    -1.0
                } else {
                    1.0
                };
                let dv_x = sign * properties.dash_speed;
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
                        transition_from_land_crouch,
                        transition_from_dash,
                        transition_from_turnaround,
                        transition_from_run_turnaround,
                        transition_from_airdodge,
                        transition_from_jumpsquat,
                        compute_common_side_effects,
                    )
                        .in_set(FighterEventSet::Act),
                    (
                        land,
                        go_airborne,
                        update_fighter_state,
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
    pub properties: FighterProperties,
    pub animation_indices: AnimationIndices,
    pub animation_timer: AnimationTimer,
    pub control: Control,
    pub percent: Percent,
    pub weight: Weight,
}
