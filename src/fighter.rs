use std::f32::consts::PI;

use bevy::prelude::*;

use crate::{
    fighter_state::{
        apply_state_transition, FighterState, FighterStateTransition, AIRDODGE_DURATION_FRAMES,
        AIRDODGE_INITIAL_SPEED, DEFAULT_JUMP_SQUAT_DURATION, RUN_TURNAROUND_DURATION_FRAMES,
        TURNAROUND_DURATION_FRAMES,
    },
    hitbox::{HitboxCollision, HitboxPurpose, KnockbackAngle},
    input::{Action, Control},
    physics::{Collision, Gravity, SetVelocity, Velocity},
    utils::{Directed, FrameCount, FrameNumber},
    Airborne, AnimationIndices, AnimationTimer, Facing, PhysicsSet,
};

pub mod megaman;

// Control thresholds
pub const CROUCH_THRESHOLD: f32 = 0.4;

#[derive(Component)]
pub struct PlayerId(pub usize);

#[derive(Component)]
#[allow(dead_code)]
pub struct FighterProperties {
    walk_speed: f32,
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

fn set_airdodge_speed(mut query: Query<(&FighterState, &FrameCount, &mut Velocity)>) {
    for (s, f, mut v) in query.iter_mut() {
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
pub struct DashSpeed(pub f32);

fn set_dash_speed(
    mut query: Query<(
        &FighterState,
        &FrameCount,
        &mut Facing,
        &mut Velocity,
        &DashSpeed,
        &Control,
    )>,
) {
    for (state, frame, mut facing, mut velocity, dash_speed, control) in query.iter_mut() {
        if state != &FighterState::Dash {
            continue;
        }
        if frame.0 != 0 {
            continue;
        }
        let cardinal_direction = control.stick.get_cardinal_direction();
        if cardinal_direction.is_none() {
            return;
        }
        let horizontal = cardinal_direction.unwrap().horizontal();
        if horizontal.is_none() {
            return;
        }
        facing.0 = horizontal.unwrap();
        velocity.0.x = dash_speed.0 * horizontal.unwrap().get_sign();
    }
}

#[derive(Component)]
pub struct RunSpeed(pub f32);

fn accelerate_to_run_speed(
    mut query: Query<(
        &mut FighterState,
        &mut Velocity,
        &RunSpeed,
        &Traction,
        &Control,
    )>,
) {
    for (mut state, mut velocity, speed, traction, control) in query.iter_mut() {
        if *state != FighterState::Run {
            continue;
        }
        let cardinal_direction = control.stick.get_cardinal_direction();
        if cardinal_direction.is_none() {
            *state = FighterState::RunEnd;
            return;
        }
        let horizontal = cardinal_direction.unwrap().horizontal();
        if horizontal.is_none() {
            *state = FighterState::RunEnd;
            return;
        }
        let target_vx = horizontal
            .expect("Horizontal input during run")
            .get_sign()
            * speed.0;
        if (velocity.0.x - target_vx).abs() <= traction.0 {
            velocity.0.x = target_vx;
        } else if velocity.0.x < target_vx {
            velocity.0.x += traction.0;
        } else {
            velocity.0.x -= traction.0;
        }
    }
}

#[derive(Component)]
pub struct WalkSpeed(pub f32);

fn accelerate_to_walk_speed(
    mut query: Query<(
        &FighterState,
        &mut Velocity,
        &WalkSpeed,
        &Traction,
        &Control,
    )>,
) {
    for (state, mut velocity, speed, traction, control) in query.iter_mut() {
        if state != &FighterState::Walk {
            continue;
        }
        let target_vx = control
            .stick
            .get_cardinal_direction()
            .expect("Horizontal input during walk")
            .horizontal()
            .expect("Horizontal input during walk")
            .get_sign()
            * speed.0;
        if (velocity.0.x - target_vx).abs() <= traction.0 {
            velocity.0.x = target_vx;
        } else if velocity.0.x < target_vx {
            velocity.0.x += traction.0;
        } else {
            velocity.0.x -= traction.0;
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

fn apply_turnaround(
    mut query: Query<(&mut Facing, &FighterState, &FrameCount), Without<Airborne>>,
) {
    for (mut facing, state, frame) in query.iter_mut() {
        let should_flip = match state {
            FighterState::Turnaround => frame.0 == TURNAROUND_DURATION_FRAMES / 2,
            FighterState::RunTurnaround => frame.0 == RUN_TURNAROUND_DURATION_FRAMES / 2,
            _ => false,
        };
        if should_flip {
            facing.0 = facing.0.flip();
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

fn update_damage_display(
    q_fighter: Query<(&PlayerId, &Percent), Changed<Percent>>,
    mut q_display: Query<(&PlayerId, &mut Text)>,
) {
    for (fighter_id, percent) in q_fighter.iter() {
        for (display_id, mut text) in q_display.iter_mut() {
            if fighter_id.0 != display_id.0 {
                continue;
            }
            text.sections[0].value = format!("{:.1}%", percent.0);
        }
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
            .add_systems(Update, update_damage_display)
            .add_systems(
                FixedUpdate,
                (
                    (
                        apply_state_transition
                            .chain()
                            .in_set(FighterEventSet::Act),
                        (
                            update_fighter_state,
                            apply_turnaround,
                            apply_jump_speed,
                            set_dash_speed,
                            accelerate_to_run_speed,
                            accelerate_to_walk_speed,
                            set_airdodge_speed,
                            update_gravity,
                            land,
                            go_airborne,
                            remove_intangible,
                            add_intangible,
                            take_damage_from_hitbox_collision,
                        )
                            .chain()
                            .in_set(FighterEventSet::React),
                    )
                        .chain()
                        .in_set(FighterSet),
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
    pub tag: PlayerId,
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
    pub dash_speed: DashSpeed,
    pub run_speed: RunSpeed,
    pub walk_speed: WalkSpeed,
}
