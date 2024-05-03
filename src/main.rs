// use std::collections::HashSet;

use bevy::{prelude::*, sprite::Anchor};
use iyes_perf_ui::prelude::*;
use log::info;

const FRAMES_PER_SECOND: u16 = 60;
const GRAVITY: f32 = -0.3;
const MAX_FLOOR_SLOPE: f32 = 0.1;
const INPUT_BUFFER_SIZE: u8 = 10;
const MAX_PLAYER_COUNT: usize = 4;
const CONTROL_STICK_DEADZONE: f32 = 0.8;
const HORIZONTAL_DEADZONE: f32 = 0.3;
const DASH_THRESHOLD: f32 = 0.9;

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

#[derive(Component, Default)]
struct Facing(Direction);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum PlayerState {
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

impl PlayerState {
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

#[derive(Component, Default)]
struct PlayerStateMachine {
    state: PlayerState,
    facing: Direction,
    frame_count: u16,
}

impl PlayerStateMachine {
    const LANDING_LAG: u16 = 12;
    const IDLE_CYCLE: u16 = 240;
    const JUMPSQUAT: u16 = 5;
    const FRICTION: f32 = 0.3;
    const WALK_SPEED: f32 = 3.0;
    const DASH_SPEED: f32 = 5.0;
    const DASH_DURATION: u16 = 20;
    const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
    const AIRDODGE_DURATION_FRAMES: u16 = 30;
    const AIRDODGE_INTANGIBLE_START: u16 = 4;
    const AIRDODGE_INTANGIBLE_END: u16 = 20;
    const TURNAROUND_DURATION_FRAMES: u16 = 14;
    const TURNAROUND_THRESHOLD_FRAME: u16 = Self::TURNAROUND_DURATION_FRAMES / 2;
    const RUN_TURNAROUND_DURATION_FRAMES: u16 = 14;
    const RUN_TURNAROUND_THRESHOLD_FRAME: u16 = Self::TURNAROUND_DURATION_FRAMES / 2;

    fn is_intangible(&self) -> bool {
        match self.state {
            PlayerState::Airdodge => {
                Self::AIRDODGE_INTANGIBLE_START <= self.frame_count
                    && self.frame_count <= Self::AIRDODGE_INTANGIBLE_END
            }
            _ => false,
        }
    }

    fn set_new_state(&mut self, state: &PlayerState) {
        info!("{:?} -> {:?}", self.state, state);
        self.state = *state;
        self.frame_count = 0;
    }

    fn advance_frame(&mut self) {
        self.frame_count += 1;
        debug!("{:?}", (self.state, self.frame_count));
        match (self.state, self.frame_count) {
            (PlayerState::LandCrouch, Self::LANDING_LAG) => self.set_new_state(&PlayerState::Idle),
            // Blinky blinky
            (PlayerState::Idle, Self::IDLE_CYCLE) => {
                self.frame_count = 0;
            }
            (PlayerState::Turnaround, Self::TURNAROUND_DURATION_FRAMES) => {
                self.set_new_state(&PlayerState::Idle);
            }
            (PlayerState::Turnaround, Self::TURNAROUND_THRESHOLD_FRAME) => {
                self.facing = self.facing.flip();
            }
            (PlayerState::RunTurnaround, Self::RUN_TURNAROUND_DURATION_FRAMES) => {
                self.set_new_state(&PlayerState::Run);
            }
            (PlayerState::RunTurnaround, Self::RUN_TURNAROUND_THRESHOLD_FRAME) => {
                self.facing = self.facing.flip();
            }
            (PlayerState::Airdodge, Self::AIRDODGE_DURATION_FRAMES) => {
                self.set_new_state(&PlayerState::IdleAirborne);
            }
            (PlayerState::Dash, Self::DASH_DURATION) => {
                self.set_new_state(&PlayerState::Run);
            }
            (PlayerState::RunEnd, 2) => {
                self.set_new_state(&PlayerState::Idle);
            }
            _ => {}
        }
    }

    fn update_physics(&self, velocity: &mut Velocity, input: &PlayerInput) {
        match (self.state, self.frame_count) {
            (PlayerState::JumpSquat, Self::JUMPSQUAT) => {
                velocity.0.y += 10.0;
                // pos.0 += Vec2::new(0.0, 1.0);
            }
            (PlayerState::Airdodge, 0) => {
                let control = if input.control.length() > CONTROL_STICK_DEADZONE {
                    input.control.normalize_or_zero()
                } else {
                    Vec2::ZERO
                };
                velocity.0 = control * Self::AIRDODGE_INITIAL_SPEED;
            }
            (PlayerState::Airdodge, _) => {
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
            (PlayerState::Dash, 0) => {
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
            PlayerState::IdleAirborne => {
                self.set_new_state(&PlayerState::LandCrouch);
            }
            PlayerState::Airdodge => {
                self.set_new_state(&PlayerState::LandCrouch);
            }
            _ => {}
        }
    }

    fn go_airborne(&mut self) {
        if !self.state.is_grounded() {
            return;
        }
        self.set_new_state(&PlayerState::IdleAirborne);
    }

    fn apply_input(&mut self, input: &PlayerInput) -> BufferResult {
        let button = input.buffered_action.map(|x| x.action);
        match (self.state, button, input.dash) {
            (PlayerState::Idle, _, Some(direction))
            | (PlayerState::Turnaround, _, Some(direction)) => {
                self.facing = direction;
                self.set_new_state(&PlayerState::Dash);
                BufferResult::Rejected
            }
            (PlayerState::Dash, _, Some(direction)) => {
                if direction != self.facing {
                    self.facing = direction;
                    self.set_new_state(&PlayerState::Dash);
                }
                BufferResult::Rejected
            }
            (PlayerState::Idle, Some(PlayerButton::Jump), _)
            | (PlayerState::Dash, Some(PlayerButton::Jump), _)
            | (PlayerState::Run, Some(PlayerButton::Jump), _)
            | (PlayerState::Walk, Some(PlayerButton::Jump), _)
            | (PlayerState::Turnaround, Some(PlayerButton::Jump), _) => {
                self.set_new_state(&PlayerState::JumpSquat);
                BufferResult::Accepted
            }
            (PlayerState::Idle, None, _) => {
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
                    self.set_new_state(&PlayerState::Turnaround);
                    BufferResult::Rejected
                } else {
                    self.set_new_state(&PlayerState::Walk);
                    BufferResult::Rejected
                }
            }
            (PlayerState::Run, None, _) => {
                let control_x = input.control.x;
                if control_x.abs() < HORIZONTAL_DEADZONE {
                    self.set_new_state(&PlayerState::RunEnd);
                    return BufferResult::Rejected;
                }
                let control_direction = if control_x < 0.0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
                if control_direction != self.facing {
                    self.set_new_state(&PlayerState::RunTurnaround);
                }
                BufferResult::Rejected
            }
            (PlayerState::RunEnd, None, _) => {
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
                    self.set_new_state(&PlayerState::RunTurnaround);
                }
                BufferResult::Rejected
            }
            (PlayerState::Walk, _, Some(direction)) => {
                if self.facing == direction {
                    self.set_new_state(&PlayerState::Dash);
                }
                BufferResult::Rejected
            }
            (PlayerState::Walk, None, _) => {
                let control_x = input.control.x;
                if control_x.abs() < HORIZONTAL_DEADZONE {
                    self.set_new_state(&PlayerState::Idle);
                    return BufferResult::Rejected;
                }
                let control_direction = if control_x < 0.0 {
                    Direction::Left
                } else {
                    Direction::Right
                };
                if control_direction != self.facing {
                    self.set_new_state(&PlayerState::Turnaround);
                }
                BufferResult::Rejected
            }
            (PlayerState::IdleAirborne, Some(PlayerButton::Shield), _)
            | (PlayerState::JumpSquat, Some(PlayerButton::Shield), _) => {
                self.set_new_state(&PlayerState::Airdodge);
                BufferResult::Accepted
            }
            _ => BufferResult::Rejected,
        }
    }

    // Empty if no new data is to be given
    fn get_animation_data(&self, velocity: &Velocity) -> AnimationUpdate {
        match (self.state, self.frame_count) {
            (PlayerState::Idle, 200) => AnimationUpdate::MultiFrame {
                first: 0,
                last: 2,
                seconds_per_frame: 0.1,
            },
            (PlayerState::Idle, 0) => AnimationUpdate::SingleFrame(0),
            (PlayerState::Turnaround, 0) => AnimationUpdate::SingleFrame(74),
            (PlayerState::IdleAirborne, _) => {
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
            (PlayerState::LandCrouch, 0) => AnimationUpdate::SingleFrame(133),
            (PlayerState::JumpSquat, 0) => AnimationUpdate::SingleFrame(133),
            (PlayerState::Walk, 0) => AnimationUpdate::MultiFrame {
                first: 5,
                last: 14,
                seconds_per_frame: 0.1,
            },
            (PlayerState::Airdodge, 0) => AnimationUpdate::SingleFrame(33),
            (PlayerState::Dash, 0) => AnimationUpdate::SingleFrame(24),
            (PlayerState::RunTurnaround, 0) => AnimationUpdate::SingleFrame(30),
            (PlayerState::Run, 0) => AnimationUpdate::MultiFrame {
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
        if psm.state == PlayerState::JumpSquat {
            // Don't touch during jumpsquat
            continue;
        }
        v.0 = match psm.state {
            PlayerState::Walk => PlayerStateMachine::WALK_SPEED,
            PlayerState::Dash => PlayerStateMachine::DASH_SPEED,
            PlayerState::Run => PlayerStateMachine::DASH_SPEED,
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
        if psm.state.is_grounded() && psm.state != PlayerState::Dash {
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
        if !psm.state.is_grounded() || psm.state == PlayerState::Dash {
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

const INTANGIBLE_FLASH_PERIOD: u16 = 4;

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
