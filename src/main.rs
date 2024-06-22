// use std::collections::HashSet;

use std::fmt::Debug;
use std::{collections::HashSet, ops::Add};

use bevy::{prelude::*, sprite::Anchor};
use iyes_perf_ui::prelude::*;
use log::info;

mod fighter;
mod input;
mod physics;
mod utils;

use utils::{FrameCount, FrameNumber};

const FRAMES_PER_SECOND: FrameNumber = 60;
const GRAVITY: f32 = -0.3;
const MAX_FLOOR_SLOPE: f32 = 0.1;
const INPUT_BUFFER_SIZE: u8 = 10;
const MAX_PLAYER_COUNT: usize = 4;
const CONTROL_STICK_DEADZONE: f32 = 0.8;
const HORIZONTAL_DEADZONE: f32 = 0.3;
const DASH_THRESHOLD: f32 = 0.9;
const NEAR_MISS_DISTANCE: f32 = 0.1;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        // we want Bevy to measure these values for us:
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        .insert_resource(Time::<Fixed>::from_hz(FRAMES_PER_SECOND as f64))
        .insert_resource(PlayerInputs(core::array::from_fn(|_| {
            PlayerInput::default()
        })))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                // gamepad_connections,
                read_player_input,
                animate_sprite,
                align_sprites_with_facing,
            ),
        )
        .add_systems(
            FixedUpdate,
            (
                update_frame_count,
                apply_player_input,
                update_player_facing,
                update_target_horizontal_velocity,
                update_player_physics,
                add_gravity_component,
                remove_gravity_component,
                add_friction_component,
                remove_friction_component,
                apply_gravity,
                apply_friction,
                apply_velocity_to_players,
                update_intangible_tag,
                snap_texture_to_position,
                update_animation,
                advance_frame,
                update_intangible_flash,
                update_intangible_no_flash,
            )
                .chain(),
        )
        .run();
}

fn update_frame_count(mut query: Query<&mut FrameCount>) {
    for mut frame_count in &mut query {
        frame_count.0 += 1;
    }
}

fn apply_player_input(
    mut buffer: ResMut<PlayerInputs>,
    mut query: Query<(&PlayerId, &mut PlayerStateMachine)>,
) {
    for (player_id, mut psm) in &mut query {
        let result = psm.apply_input(&buffer.0[player_id.0]);
        if result == BufferResult::Accepted {
            buffer.0[player_id.0].buffered_action = None;
        }
    }
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let tag = PlayerId(0);
    let texture = asset_server.load("spritesheet/x3_2.png");
    let layout = TextureAtlasLayout::from_grid(Vec2::new(80.0, 73.0), 12, 12, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    // Use only the subset of sprites in the sheet that make up the run animation
    let animation_indices = AnimationIndices {
        first: 1,
        last: 137,
    };
    let animation_timer = AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating));
    let sprite_sheet_bundle = SpriteSheetBundle {
        texture,
        atlas: TextureAtlas {
            layout: texture_atlas_layout,
            index: animation_indices.first,
        },
        sprite: Sprite {
            anchor: Anchor::BottomCenter,
            ..default()
        },
        transform: Transform::from_scale(Vec3::splat(2.0)),
        ..default()
    };
    let state_machine = PlayerStateMachine::default();

    commands.spawn(Camera2dBundle::default());
    commands.spawn(PlayerBundle {
        tag,
        facing: Facing(Direction::Right),
        position: Position::default(),
        velocity: Velocity(Vec2::new(5.0, 0.0)),
        target_horizontal_velocity: TargetHorizontalVelocity::default(),
        gravity: Gravity(GRAVITY),
        state_machine,
        sprite_sheet_bundle,
        animation_indices,
        animation_timer,
    });
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                scale: Vec3::new(800.0, 1.0, 0.0),
                ..default()
            },
            sprite: Sprite {
                color: Color::rgb(1.0, 1.0, 1.0),
                ..default()
            },
            ..default()
        },
        Collider {
            normal: Vec2::new(0.0, 1.0),
            breadth: 800.0,
        },
        Position(Vec2::new(0.0, -200.0)),
    ));
    commands.spawn(PerfUiCompleteBundle::default());
}

#[derive(Clone, Copy, Debug)]
enum PlayerButton {
    Jump,
    // Attack,
    // Special,
    // Grab,
    Shield,
}

#[derive(Clone, Copy)]
struct BufferedPlayerButton {
    action: PlayerButton,
    age_in_frames: u8,
}

impl BufferedPlayerButton {
    fn aged(&self) -> Option<Self> {
        let next_age = self.age_in_frames + 1;
        if next_age <= INPUT_BUFFER_SIZE {
            Some(BufferedPlayerButton {
                action: self.action,
                age_in_frames: next_age,
            })
        } else {
            None
        }
    }
}

#[derive(Default)]
struct PlayerInput {
    control: Vec2,
    _last_control: Vec2,
    dash: Option<Direction>,
    // held_actions: HashSet<PlayerButton>,
    buffered_action: Option<BufferedPlayerButton>,
}

#[derive(Resource)]
struct PlayerInputs([PlayerInput; MAX_PLAYER_COUNT]);

impl PlayerInputs {
    fn buffer_button(&mut self, id: usize, action: PlayerButton) {
        self.0[id].buffered_action = Some(BufferedPlayerButton {
            action,
            age_in_frames: 0,
        });
    }

    fn set_control(&mut self, id: usize, control: Vec2) {
        if control.x > DASH_THRESHOLD
            && (self.0[id].control.x < HORIZONTAL_DEADZONE
                || self.0[id]._last_control.x < HORIZONTAL_DEADZONE)
        {
            self.0[id].dash = Some(Direction::Right);
        } else if control.x < -DASH_THRESHOLD
            && (self.0[id].control.x > -HORIZONTAL_DEADZONE
                || self.0[id]._last_control.x > -HORIZONTAL_DEADZONE)
        {
            self.0[id].dash = Some(Direction::Left);
        } else {
            self.0[id].dash = None;
        }

        self.0[id]._last_control = self.0[id].control;
        self.0[id].control = control;
    }

    fn advance_frame(&mut self) {
        for i in 0..MAX_PLAYER_COUNT {
            self.0[i].buffered_action = self.0[i]
                .buffered_action
                .map(|x| x.aged())
                .flatten();
        }
    }
}

fn align_sprites_with_facing(mut query: Query<(&Facing, &mut Sprite)>) {
    for (facing, mut sprite) in &mut query {
        sprite.flip_x = facing.0 == Direction::Left;
    }
}

fn read_player_input(
    gamepads: Res<Gamepads>,
    button_inputs: Res<ButtonInput<GamepadButton>>,
    // button_axes: Res<Axis<GamepadButton>>,
    axes: Res<Axis<GamepadAxis>>,
    mut inputs: ResMut<PlayerInputs>,
) {
    for gamepad in gamepads.iter() {
        let control_x = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickX))
            .unwrap();
        let control_y = axes
            .get(GamepadAxis::new(gamepad, GamepadAxisType::LeftStickY))
            .unwrap();
        let control = Vec2::new(control_x, control_y);
        inputs.set_control(gamepad.id, control);
        // Check for jump
        let north = GamepadButton::new(gamepad, GamepadButtonType::North);
        let east = GamepadButton::new(gamepad, GamepadButtonType::East);
        if button_inputs.any_just_pressed([north, east]) {
            inputs.buffer_button(gamepad.id, PlayerButton::Jump);
            return;
        }
        let left_trigger = GamepadButton::new(gamepad, GamepadButtonType::LeftTrigger2);
        let right_trigger = GamepadButton::new(gamepad, GamepadButtonType::RightTrigger2);
        if button_inputs.any_just_pressed([left_trigger, right_trigger]) {
            inputs.buffer_button(gamepad.id, PlayerButton::Shield);
        }
    }
}

// First index, last index, seconds per frame
enum AnimationUpdate {
    None,
    SingleFrame(usize),
    MultiFrame {
        first: usize,
        last: usize,
        seconds_per_frame: f32,
    },
}

#[derive(PartialEq, Eq)]
enum BufferResult {
    Accepted,
    Rejected,
}

#[derive(Default, PartialEq, Eq, Clone, Copy)]
enum Direction {
    Left,
    #[default]
    Right,
}

impl Direction {
    fn flip(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
}

#[derive(Component, Default, Clone, Copy)]
struct Facing(Direction);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum BasicState {
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

impl BasicState {
    fn is_grounded(&self) -> bool {
        match self {
            Self::Idle => true,
            Self::LandCrouch => true,
            Self::JumpSquat => true,
            Self::Walk => true,
            Self::Turnaround => true,
            Self::RunTurnaround => true,
            Self::RunEnd => true,
            Self::Dash => true,
            Self::Run => true,
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

enum Button2 {
    Attack,
    Special,
    Shield,
    Grab,
}

enum CircularDirection {
    Clockwise,
    CounterClockwise,
}

enum DirectionInput {
    Tilt(Vec2),
    Smash(Vec2),
    HalfCircle(Vec2, CircularDirection),
}

struct PlayerInput2(HashSet<Button2>, DirectionInput);

enum InputBufferResult {
    Accept,
    Reject,
}

enum SideEffect<S> {
    StateChange(S),
    AddVelocity(Vec2),
    SetFacing(Direction),
}

type SideEffects<S> = Vec<SideEffect<S>>;

trait PlayerState: Sized + Debug {
    // Effects of being in a given state
    fn get_side_effects(&self) -> SideEffects<Self>;
    // Events
    fn apply_input(&self, input: &PlayerInput2) -> Option<SideEffects<Self>>;
    fn land(&self) -> SideEffects<Self>;
    // Default states
    fn get_default_airborne() -> Self;
    // Hierarchical logic
    fn get_super_state(&self) -> Option<Self> {
        None
    }
}

impl PlayerState for (BasicState, FrameNumber) {
    fn get_side_effects(&self) -> SideEffects<Self> {
        todo!()
    }

    fn apply_input(&self, input: &PlayerInput2) -> Option<SideEffects<Self>> {
        todo!()
    }

    fn land(&self) -> SideEffects<Self> {
        todo!()
    }

    fn get_default_airborne() -> Self {
        todo!()
    }
}

#[derive(Component, Default)]
struct PlayerStateMachine {
    state: BasicState,
    facing: Direction,
    frame_count: FrameNumber,
}

impl PlayerStateMachine {
    const LANDING_LAG: FrameNumber = 12;
    const IDLE_CYCLE: FrameNumber = 240;
    const JUMPSQUAT: FrameNumber = 5;
    const FRICTION: f32 = 0.3;
    const WALK_SPEED: f32 = 3.0;
    const DASH_SPEED: f32 = 5.0;
    const DASH_DURATION: FrameNumber = 20;
    const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
    const AIRDODGE_DURATION_FRAMES: FrameNumber = 30;
    const AIRDODGE_INTANGIBLE_START: FrameNumber = 4;
    const AIRDODGE_INTANGIBLE_END: FrameNumber = 20;
    const TURNAROUND_DURATION_FRAMES: FrameNumber = 14;
    const TURNAROUND_THRESHOLD_FRAME: FrameNumber = Self::TURNAROUND_DURATION_FRAMES / 2;
    const RUN_TURNAROUND_DURATION_FRAMES: FrameNumber = 14;
    const RUN_TURNAROUND_THRESHOLD_FRAME: FrameNumber = Self::TURNAROUND_DURATION_FRAMES / 2;

    fn is_intangible(&self) -> bool {
        match self.state {
            BasicState::Airdodge => {
                Self::AIRDODGE_INTANGIBLE_START <= self.frame_count
                    && self.frame_count <= Self::AIRDODGE_INTANGIBLE_END
            }
            _ => false,
        }
    }

    fn set_new_state(&mut self, state: &BasicState) {
        info!("{:?} -> {:?}", self.state, state);
        self.state = *state;
        self.frame_count = 0;
    }

    fn advance_frame(&mut self) {
        self.frame_count += 1;
        debug!("{:?}", (self.state, self.frame_count));
        match (self.state, self.frame_count) {
            (BasicState::LandCrouch, Self::LANDING_LAG) => self.set_new_state(&BasicState::Idle),
            // Blinky blinky
            (BasicState::Idle, Self::IDLE_CYCLE) => {
                self.frame_count = 0;
            }
            (BasicState::Turnaround, Self::TURNAROUND_DURATION_FRAMES) => {
                self.set_new_state(&BasicState::Idle);
            }
            (BasicState::Turnaround, Self::TURNAROUND_THRESHOLD_FRAME) => {
                self.facing = self.facing.flip();
            }
            (BasicState::RunTurnaround, Self::RUN_TURNAROUND_DURATION_FRAMES) => {
                self.set_new_state(&BasicState::Run);
            }
            (BasicState::RunTurnaround, Self::RUN_TURNAROUND_THRESHOLD_FRAME) => {
                self.facing = self.facing.flip();
            }
            (BasicState::Airdodge, Self::AIRDODGE_DURATION_FRAMES) => {
                self.set_new_state(&BasicState::IdleAirborne);
            }
            (BasicState::Dash, Self::DASH_DURATION) => {
                self.set_new_state(&BasicState::Run);
            }
            (BasicState::RunEnd, 2) => {
                self.set_new_state(&BasicState::Idle);
            }
            _ => {}
        }
    }

    fn update_physics(&self, velocity: &mut Velocity, input: &PlayerInput) {
        match (self.state, self.frame_count) {
            (BasicState::JumpSquat, Self::JUMPSQUAT) => {
                velocity.0.y += 10.0;
                // pos.0 += Vec2::new(0.0, 1.0);
            }
            (BasicState::Airdodge, 0) => {
                let control = if input.control.length() > CONTROL_STICK_DEADZONE {
                    input.control.normalize_or_zero()
                } else {
                    Vec2::ZERO
                };
                velocity.0 = control * Self::AIRDODGE_INITIAL_SPEED;
            }
            (BasicState::Airdodge, _) => {
                let speed_reduction_per_frame =
                    Self::AIRDODGE_INITIAL_SPEED / (Self::AIRDODGE_DURATION_FRAMES as f32);
                let current_speed = velocity.0.length();
                if current_speed == 0.0 {
                    return;
                }
                let desired_speed = current_speed - speed_reduction_per_frame;
                let ratio = desired_speed / current_speed;
                velocity.0 *= ratio;
            }
            (BasicState::Dash, 0) => {
                velocity.0.x = if input.control.x.is_sign_negative() {
                    -Self::DASH_SPEED
                } else {
                    Self::DASH_SPEED
                };
            }
            _ => {}
        }
    }

    fn land(&mut self, pushback: &Vec2, velocity: &mut Velocity) {
        let normal = pushback.normalize();
        let modified_pushback = normal * (normal.dot(*pushback));
        velocity.0 += modified_pushback;
        match self.state {
            BasicState::IdleAirborne => {
                self.set_new_state(&BasicState::LandCrouch);
            }
            BasicState::Airdodge => {
                self.set_new_state(&BasicState::LandCrouch);
            }
            _ => {}
        }
    }

    fn go_airborne(&mut self) {
        if !self.state.is_grounded() {
            return;
        }
        self.set_new_state(&BasicState::IdleAirborne);
    }

    fn apply_input(&mut self, input: &PlayerInput) -> BufferResult {
        let button = input.buffered_action.map(|x| x.action);
        match (self.state, button, input.dash) {
            (BasicState::Idle, _, Some(direction))
            | (BasicState::Turnaround, _, Some(direction)) => {
                self.facing = direction;
                self.set_new_state(&BasicState::Dash);
                BufferResult::Rejected
            }
            (BasicState::Dash, _, Some(direction)) => {
                if direction != self.facing {
                    self.facing = direction;
                    self.set_new_state(&BasicState::Dash);
                }
                BufferResult::Rejected
            }
            (BasicState::Idle, Some(PlayerButton::Jump), _)
            | (BasicState::Dash, Some(PlayerButton::Jump), _)
            | (BasicState::Run, Some(PlayerButton::Jump), _)
            | (BasicState::Walk, Some(PlayerButton::Jump), _)
            | (BasicState::Turnaround, Some(PlayerButton::Jump), _) => {
                self.set_new_state(&BasicState::JumpSquat);
                BufferResult::Accepted
            }
            (BasicState::Idle, None, _) => {
                let control_x = input.control.x;
                if control_x.abs() < HORIZONTAL_DEADZONE {
                    return BufferResult::Rejected;
                }
                let control_direction = if control_x < 0.0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
                if control_direction != self.facing {
                    self.set_new_state(&BasicState::Turnaround);
                    BufferResult::Rejected
                } else {
                    self.set_new_state(&BasicState::Walk);
                    BufferResult::Rejected
                }
            }
            (BasicState::Run, None, _) => {
                let control_x = input.control.x;
                if control_x.abs() < HORIZONTAL_DEADZONE {
                    self.set_new_state(&BasicState::RunEnd);
                    return BufferResult::Rejected;
                }
                let control_direction = if control_x < 0.0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
                if control_direction != self.facing {
                    self.set_new_state(&BasicState::RunTurnaround);
                }
                BufferResult::Rejected
            }
            (BasicState::RunEnd, None, _) => {
                let control_x = input.control.x;
                if control_x.abs() < HORIZONTAL_DEADZONE {
                    return BufferResult::Rejected;
                }
                let control_direction = if control_x < 0.0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
                if control_direction != self.facing {
                    self.set_new_state(&BasicState::RunTurnaround);
                }
                BufferResult::Rejected
            }
            (BasicState::Walk, _, Some(direction)) => {
                if self.facing == direction {
                    self.set_new_state(&BasicState::Dash);
                }
                BufferResult::Rejected
            }
            (BasicState::Walk, None, _) => {
                let control_x = input.control.x;
                if control_x.abs() < HORIZONTAL_DEADZONE {
                    self.set_new_state(&BasicState::Idle);
                    return BufferResult::Rejected;
                }
                let control_direction = if control_x < 0.0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
                if control_direction != self.facing {
                    self.set_new_state(&BasicState::Turnaround);
                }
                BufferResult::Rejected
            }
            (BasicState::IdleAirborne, Some(PlayerButton::Shield), _)
            | (BasicState::JumpSquat, Some(PlayerButton::Shield), _) => {
                self.set_new_state(&BasicState::Airdodge);
                BufferResult::Accepted
            }
            _ => BufferResult::Rejected,
        }
    }

    // Empty if no new data is to be given
    fn get_animation_data(&self, velocity: &Velocity) -> AnimationUpdate {
        match (self.state, self.frame_count) {
            (BasicState::Idle, 200) => AnimationUpdate::MultiFrame {
                first: 0,
                last: 2,
                seconds_per_frame: 0.1,
            },
            (BasicState::Idle, 0) => AnimationUpdate::SingleFrame(0),
            (BasicState::Turnaround, 0) => AnimationUpdate::SingleFrame(74),
            (BasicState::IdleAirborne, _) => {
                let y = velocity.0.y;
                if y > 1.5 {
                    AnimationUpdate::SingleFrame(18)
                } else if y > -1.5 {
                    AnimationUpdate::SingleFrame(19)
                } else {
                    AnimationUpdate::MultiFrame {
                        first: 20,
                        last: 21,
                        seconds_per_frame: 0.15,
                    }
                }
            }
            (BasicState::LandCrouch, 0) => AnimationUpdate::SingleFrame(133),
            (BasicState::JumpSquat, 0) => AnimationUpdate::SingleFrame(133),
            (BasicState::Walk, 0) => AnimationUpdate::MultiFrame {
                first: 5,
                last: 14,
                seconds_per_frame: 0.1,
            },
            (BasicState::Airdodge, 0) => AnimationUpdate::SingleFrame(33),
            (BasicState::Dash, 0) => AnimationUpdate::SingleFrame(24),
            (BasicState::RunTurnaround, 0) => AnimationUpdate::SingleFrame(30),
            (BasicState::Run, 0) => AnimationUpdate::MultiFrame {
                first: 5,
                last: 14,
                seconds_per_frame: 0.1,
            },
            _ => AnimationUpdate::None,
        }
    }
}

fn update_player_physics(
    mut q: Query<(&PlayerId, &PlayerStateMachine, &mut Velocity)>,
    inputs: Res<PlayerInputs>,
) {
    for (p, psm, mut velocity) in &mut q {
        psm.update_physics(&mut velocity, &inputs.0[p.0]);
    }
}

fn advance_frame(mut q: Query<&mut PlayerStateMachine>, mut inputs: ResMut<PlayerInputs>) {
    for mut psm in &mut q {
        psm.advance_frame();
    }
    inputs.advance_frame();
}

#[derive(Component, Clone)]
struct AnimationIndices {
    first: usize,
    last: usize,
}

#[derive(Component, Deref, DerefMut)]
struct AnimationTimer(Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (indices, mut timer, mut atlas) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            atlas.index = if atlas.index == indices.last {
                indices.first
            } else {
                atlas.index + 1
            };
        }
    }
}

#[derive(Component)]
struct PlayerId(usize);

#[derive(Component, Default)]
struct Position(Vec2);

#[derive(Component, Default)]
struct Velocity(Vec2);

#[derive(Component, Default)]
struct TargetHorizontalVelocity(f32);

#[derive(Component)]
struct Friction(f32);

#[derive(Component)]
struct Gravity(f32);

#[derive(Component)]
struct Intangible;

#[derive(Bundle)]
struct PlayerBundle {
    tag: PlayerId,
    position: Position,
    velocity: Velocity,
    facing: Facing,
    target_horizontal_velocity: TargetHorizontalVelocity,
    gravity: Gravity,
    state_machine: PlayerStateMachine,
    sprite_sheet_bundle: SpriteSheetBundle,
    animation_indices: AnimationIndices,
    animation_timer: AnimationTimer,
}

#[derive(Component)]
struct Collider {
    normal: Vec2,
    breadth: f32,
}

impl Collider {
    fn get_pushback(&self, p: &Vec2, d: &Vec2, c: &Vec2) -> Option<Vec2> {
        let denominator = self.normal.dot(*d);
        // If denominator is 0, velocity is parallel to collider
        // If denominator is greater than 0, we're moving away from the collider
        if denominator >= 0.0 {
            return None;
        }
        let numerator = self.normal.dot(*c - *p);
        let t = numerator / denominator;
        if t < 0.0 || t > 1.0 {
            return None;
        }
        let b_0 = *p + t * *d;
        let distance_from_centre = (b_0 - *c).length();
        if distance_from_centre > self.breadth * 0.5 {
            return None;
        }
        Some((t - 1.0) * d.dot(self.normal) * self.normal)
    }
}

fn update_animation(
    mut query: Query<
        (
            &PlayerStateMachine,
            &mut AnimationIndices,
            &mut AnimationTimer,
            &mut TextureAtlas,
            &Velocity,
        ),
        With<PlayerId>,
    >,
) {
    for (psm, mut indices, mut timer, mut atlas, v) in &mut query {
        match psm.get_animation_data(&v) {
            AnimationUpdate::None => {}
            AnimationUpdate::SingleFrame(frame) => {
                *indices = AnimationIndices {
                    first: frame,
                    last: frame,
                };
                *timer = AnimationTimer(Timer::from_seconds(0.0, TimerMode::Once));
                atlas.index = frame;
            }
            AnimationUpdate::MultiFrame {
                first,
                last,
                seconds_per_frame,
            } => {
                if indices.first == first && indices.last == last {
                    // No change
                    return;
                }
                *indices = AnimationIndices { first, last };
                *timer =
                    AnimationTimer(Timer::from_seconds(seconds_per_frame, TimerMode::Repeating));
                atlas.index = first;
            }
        }
    }
}

fn snap_texture_to_position(mut query: Query<(&Position, &mut Transform)>) {
    for (pos, mut transform) in &mut query {
        transform.translation.x = pos.0.x;
        transform.translation.y = pos.0.y;
    }
}

fn update_player_facing(mut query: Query<(&PlayerStateMachine, &mut Facing)>) {
    for (psm, mut f) in &mut query {
        f.0 = psm.facing;
    }
}

fn update_target_horizontal_velocity(
    mut query: Query<(&PlayerStateMachine, &mut TargetHorizontalVelocity)>,
) {
    for (psm, mut v) in &mut query {
        if psm.state == BasicState::JumpSquat {
            // Don't touch during jumpsquat
            continue;
        }
        v.0 = match psm.state {
            BasicState::Walk => PlayerStateMachine::WALK_SPEED,
            BasicState::Dash => PlayerStateMachine::DASH_SPEED,
            BasicState::Run => PlayerStateMachine::DASH_SPEED,
            _ => 0.0,
        } * match psm.facing {
            Direction::Left => -1.0,
            Direction::Right => 1.0,
        };
    }
}

fn add_gravity_component(
    query: Query<(Entity, &PlayerStateMachine), Without<Gravity>>,
    mut commands: Commands,
) {
    for (id, psm) in query.iter() {
        if psm.state.is_affected_by_gravity() {
            commands
                .get_entity(id)
                .expect("Player entity id should exist")
                .insert(Gravity(GRAVITY));
        }
    }
}

fn remove_gravity_component(
    query: Query<(Entity, &PlayerStateMachine), With<Gravity>>,
    mut commands: Commands,
) {
    for (id, psm) in query.iter() {
        if !psm.state.is_affected_by_gravity() {
            commands
                .get_entity(id)
                .expect("Player entity id should exist")
                .remove::<Gravity>();
        }
    }
}

fn add_friction_component(
    query: Query<(Entity, &PlayerStateMachine), Without<Friction>>,
    mut commands: Commands,
) {
    for (id, psm) in query.iter() {
        if psm.state.is_grounded() && psm.state != BasicState::Dash {
            commands
                .get_entity(id)
                .expect("Player entity id should exist")
                .insert(Friction(PlayerStateMachine::FRICTION));
        }
    }
}

fn remove_friction_component(
    query: Query<(Entity, &PlayerStateMachine), With<Friction>>,
    mut commands: Commands,
) {
    for (id, psm) in query.iter() {
        if !psm.state.is_grounded() || psm.state == BasicState::Dash {
            commands
                .get_entity(id)
                .expect("Player entity id should exist")
                .remove::<Friction>();
        }
    }
}

fn apply_friction(mut query: Query<(&Friction, &TargetHorizontalVelocity, &mut Velocity)>) {
    for (f, tv, mut v) in &mut query {
        if f.0 == 0.0 {
            continue;
        }
        let tolerance = f.0.abs();
        let difference = (tv.0 - v.0.x).abs();
        if difference <= tolerance {
            v.0.x = tv.0;
        } else if v.0.x < tv.0 {
            v.0.x += f.0;
        } else {
            v.0.x -= f.0;
        }
    }
}

fn apply_velocity_to_players(
    mut query: Query<(&mut Position, &mut Velocity, &mut PlayerStateMachine)>,
    colliders: Query<(&Collider, &Position), Without<Velocity>>,
) {
    for (mut p, mut v, mut psm) in &mut query {
        let pushback = displace_and_return_pushback(&mut p, &v.0, colliders.iter());
        if pushback.y != 0.0 && (pushback.x / pushback.y).abs() <= MAX_FLOOR_SLOPE {
            psm.land(&pushback, &mut v);
        } else if v.0.y != 0.0 {
            psm.go_airborne();
        }
    }
}

fn apply_gravity(mut query: Query<(&mut Velocity, &Gravity)>) {
    for (mut v, g) in &mut query {
        v.0.y += g.0;
    }
}

fn displace_and_return_pushback<'a>(
    position: &mut Position,
    displacement: &Vec2,
    colliders: impl Iterator<Item = (&'a Collider, &'a Position)>,
) -> Vec2 {
    let pushback = colliders
        .into_iter()
        .filter_map(|(collider, centre)| {
            collider.get_pushback(&position.0, displacement, &centre.0)
        })
        // .filter(|p| p.length() > 1.0)
        .next()
        .unwrap_or_default();
    position.0 += *displacement + pushback;
    return pushback;
}

const INTANGIBLE_FLASH_PERIOD: FrameNumber = 4;

fn update_intangible_flash(mut query: Query<(&mut Sprite, &PlayerStateMachine), With<Intangible>>) {
    for (mut sprite, psm) in &mut query {
        let period_number = psm.frame_count / INTANGIBLE_FLASH_PERIOD;
        let lightened = period_number % 2 == 0;
        if lightened {
            sprite.color = Color::rgb(1.5, 1.5, 1.5);
        } else {
            sprite.color = Color::WHITE;
        }
    }
}

fn update_intangible_no_flash(mut query: Query<&mut Sprite, Without<Intangible>>) {
    for mut sprite in &mut query {
        sprite.color = Color::WHITE;
    }
}

fn update_intangible_tag(
    mut commands: Commands,
    is_intangible: Query<(Entity, &PlayerStateMachine), With<Intangible>>,
    isnt_intangible: Query<(Entity, &PlayerStateMachine), Without<Intangible>>,
) {
    for (id, psm) in is_intangible.iter() {
        if !psm.is_intangible() {
            commands
                .get_entity(id)
                .expect("Player entity id should exist")
                .remove::<Intangible>();
        }
    }
    for (id, psm) in isnt_intangible.iter() {
        if psm.is_intangible() {
            commands
                .get_entity(id)
                .expect("Player entity id should exist")
                .insert(Intangible);
        }
    }
}
struct NearestPass {
    midpoint: Vec2,
    distance: f32,
}

impl NearestPass {
    fn is_collision(&self) -> bool {
        self.distance <= 0.0
    }
}

impl PartialEq for NearestPass {
    fn eq(&self, other: &Self) -> bool {
        self.distance.eq(&other.distance)
    }
}

impl PartialOrd for NearestPass {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.distance
            .partial_cmp(&other.distance)
    }
}

impl Eq for NearestPass {}

impl Ord for NearestPass {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance
            .partial_cmp(&other.distance)
            .unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
enum Shape {
    Circle { r: f32, p: Vec2 },
    Pill { r: f32, a: Vec2, b: Vec2 },
}

impl Shape {
    fn nearest_pass(s1: &Self, s2: &Self) -> NearestPass {
        match (s1, s2) {
            (Shape::Circle { r: r1, p: p1 }, Shape::Circle { r: r2, p: p2 }) => NearestPass {
                midpoint: 0.5 * (*p1 + *p2),
                distance: (*p1 - *p2).length() - r1 - r2,
            },
            (Shape::Circle { r: r1, p: c }, Shape::Pill { r: r2, a, b }) => {
                // Distance to endpoints
                let d_a = (*c - *a).length();
                let d_b = (*c - *b).length();
                // Perpendicular distance (see GDD)
                let t = (*c - *a).dot(*b - *a) / (*b - *a).length_squared();
                let point_on_line_closest_to_c = *c - (*a + (*b - *a) * t);
                let d_p = if 0.0 <= t && t <= 1.0 {
                    point_on_line_closest_to_c.length()
                } else {
                    f32::INFINITY
                };
                let distance = [d_a, d_b, d_p]
                    .into_iter()
                    .reduce(f32::min)
                    .expect("Circle-Pill distances should exist");
                let midpoint = if distance == d_a {
                    0.5 * (*a + *c)
                } else if distance == d_b {
                    0.5 * (*b + *c)
                } else {
                    0.5 * (*c + point_on_line_closest_to_c)
                };
                NearestPass {
                    midpoint,
                    distance: distance - r1 - r2,
                }
            }
            // No need to re-implement the wheel
            (Shape::Pill { .. }, Shape::Circle { .. }) => Self::nearest_pass(s2, s1),
            (
                Shape::Pill {
                    r: r1,
                    a: a1,
                    b: b1,
                },
                Shape::Pill {
                    r: r2,
                    a: a2,
                    b: b2,
                },
            ) => {
                if let Some(intersection) = intersection_of_line_segments(a1, b1, a2, b2) {
                    return NearestPass {
                        midpoint: intersection,
                        distance: -r1 - r2,
                    };
                }
                [
                    (Shape::Circle { r: *r1, p: *a1 }, s2),
                    (Shape::Circle { r: *r1, p: *b1 }, s2),
                    (Shape::Circle { r: *r2, p: *a2 }, s1),
                    (Shape::Circle { r: *r2, p: *b2 }, s1),
                ]
                .into_iter()
                .map(|(a, b)| Self::nearest_pass(&a, b))
                .reduce(std::cmp::min)
                .expect("Pill-pill distances should exist")
            }
        }
    }
}

fn cross_product(v: &Vec2, w: &Vec2) -> f32 {
    v.x * w.y - v.y * w.x
}

// https://stackoverflow.com/a/565282/5046693
fn intersection_of_line_segments(p1: &Vec2, p2: &Vec2, q1: &Vec2, q2: &Vec2) -> Option<Vec2> {
    let p = *p1;
    let r = *p2 - p;
    let q = *q1;
    let s = *q2 - q;

    let r_cross_s = cross_product(&r, &s);
    let s_cross_r = cross_product(&s, &r);

    let t = cross_product(&(q - p), &(s / r_cross_s));
    let u = cross_product(&(p - q), &(r / s_cross_r));

    let q_minus_p_cross_r = cross_product(&(q - p), &r);

    if r_cross_s == 0.0 && q_minus_p_cross_r == 0.0 {
        // Collinear
        let t0 = (q - p).dot(r / r.dot(r));
        let t1 = (q + s - p).dot(r / r.dot(r));
        if 0.0 <= t0 && t0 <= 1.0 {
            Some(p + t0 * r)
        } else if 0.0 <= t1 && t1 <= 1.0 {
            Some(p + t1 * r)
        } else if (t0 < 0.0 && t1 > 1.0) || (t1 < 0.0 && t0 > 1.0) {
            // First line segment is fully contained in second one
            Some(p)
        } else {
            None
        }
    } else if r_cross_s == 0.0 && q_minus_p_cross_r != 0.0 {
        // Parallel and non-intersecting
        None
    } else if r_cross_s != 0.0 && 0.0 <= t && t <= 1.0 && 0.0 <= u && u <= 1.0 {
        // Divergent and intersecting
        Some(p + t * r)
    } else {
        // Divergent and non-intersecting
        None
    }
}

impl Add<Vec2> for Shape {
    type Output = Self;

    fn add(self, rhs: Vec2) -> Self::Output {
        match self {
            Shape::Circle { r, p } => Shape::Circle { r, p: p + rhs },
            Shape::Pill { r, a, b } => Shape::Pill {
                r,
                a: a + rhs,
                b: b + rhs,
            },
        }
    }
}

enum HitboxPurpose {
    Body,
    Damage {
        percent: u16,
        base_knockback: f32,
        scaling_knockback: f32,
    },
}

struct Hitbox {
    shape: Shape,
    purpose: HitboxPurpose,
    priority: Option<u16>,
}

struct HitboxGroup {
    id: u16,
    hitboxes: Vec<Hitbox>,
    ignored_group_ids: HashSet<u16>,
}

impl HitboxGroup {}
