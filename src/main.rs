// use std::collections::HashSet;

use bevy::{prelude::*, sprite::Anchor};
use input::ControlStick;
use iyes_perf_ui::prelude::*;
use log::info;

mod fighter;
mod input;
mod physics;
mod utils;
mod view;

use fighter::{megaman::MegaMan, FighterBundle, Player as PlayerId};
use physics::*;
use utils::{Facing, FrameCount, FrameNumber, LeftRight};
use view::*;

const FRAMES_PER_SECOND: FrameNumber = 60;
const GRAVITY: f32 = -0.3;
const CONTROL_STICK_DEADZONE: f32 = 0.8;

fn main() {
    info!("Starting...");
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        // we want Bevy to measure these values for us:
        .add_plugins(bevy::diagnostic::FrameTimeDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::EntityCountDiagnosticsPlugin)
        .add_plugins(bevy::diagnostic::SystemInformationDiagnosticsPlugin)
        .add_plugins(PerfUiPlugin)
        .add_plugins((input::InputPlugin, view::ViewPlugin, fighter::FighterPlugin, physics::PhysicsPlugin))
        .insert_resource(Time::<Fixed>::from_hz(FRAMES_PER_SECOND as f64))
        .add_systems(Startup, setup)
        .add_systems(
            FixedUpdate,
                update_frame_count,
        )
        .run();
}

fn update_frame_count(mut query: Query<&mut FrameCount>) {
    for mut frame_count in &mut query {
        frame_count.0 += 1;
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
    // let state_machine = PlayerStateMachine::default();

    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        FighterBundle {
            tag,
            frame: FrameCount(0),
            facing: Facing(LeftRight::Right),
            position: Position::default(),
            velocity: Velocity::default(),
            gravity: Gravity(GRAVITY),
            state: fighter::FighterState::default(),
            sprite_sheet_bundle,
            animation_indices,
            animation_timer,
            control_stick: ControlStick::default(),
        },
        MegaMan,
    ));
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

// impl PlayerStateMachine {
//     const LANDING_LAG: FrameNumber = 12;
//     const IDLE_CYCLE: FrameNumber = 240;
//     const JUMPSQUAT: FrameNumber = 5;
//     const FRICTION: f32 = 0.3;
//     const WALK_SPEED: f32 = 3.0;
//     const DASH_SPEED: f32 = 5.0;
//     const DASH_DURATION: FrameNumber = 20;
//     const AIRDODGE_INITIAL_SPEED: f32 = 10.0;
//     const AIRDODGE_DURATION_FRAMES: FrameNumber = 30;
//     const AIRDODGE_INTANGIBLE_START: FrameNumber = 4;
//     const AIRDODGE_INTANGIBLE_END: FrameNumber = 20;
//     const TURNAROUND_DURATION_FRAMES: FrameNumber = 14;
//     const TURNAROUND_THRESHOLD_FRAME: FrameNumber = Self::TURNAROUND_DURATION_FRAMES / 2;
//     const RUN_TURNAROUND_DURATION_FRAMES: FrameNumber = 14;
//     const RUN_TURNAROUND_THRESHOLD_FRAME: FrameNumber = Self::TURNAROUND_DURATION_FRAMES / 2;

//     fn is_intangible(&self) -> bool {
//         match self.state {
//             BasicState::Airdodge => {
//                 Self::AIRDODGE_INTANGIBLE_START <= self.frame_count
//                     && self.frame_count <= Self::AIRDODGE_INTANGIBLE_END
//             }
//             _ => false,
//         }
//     }

//     fn set_new_state(&mut self, state: &BasicState) {
//         info!("{:?} -> {:?}", self.state, state);
//         self.state = *state;
//         self.frame_count = 0;
//     }

//     fn advance_frame(&mut self) {
//         self.frame_count += 1;
//         debug!("{:?}", (self.state, self.frame_count));
//         match (self.state, self.frame_count) {
//             (BasicState::LandCrouch, Self::LANDING_LAG) => self.set_new_state(&BasicState::Idle),
//             // Blinky blinky
//             (BasicState::Idle, Self::IDLE_CYCLE) => {
//                 self.frame_count = 0;
//             }
//             (BasicState::Turnaround, Self::TURNAROUND_DURATION_FRAMES) => {
//                 self.set_new_state(&BasicState::Idle);
//             }
//             (BasicState::Turnaround, Self::TURNAROUND_THRESHOLD_FRAME) => {
//                 self.facing = self.facing.flip();
//             }
//             (BasicState::RunTurnaround, Self::RUN_TURNAROUND_DURATION_FRAMES) => {
//                 self.set_new_state(&BasicState::Run);
//             }
//             (BasicState::RunTurnaround, Self::RUN_TURNAROUND_THRESHOLD_FRAME) => {
//                 self.facing = self.facing.flip();
//             }
//             (BasicState::Airdodge, Self::AIRDODGE_DURATION_FRAMES) => {
//                 self.set_new_state(&BasicState::IdleAirborne);
//             }
//             (BasicState::Dash, Self::DASH_DURATION) => {
//                 self.set_new_state(&BasicState::Run);
//             }
//             (BasicState::RunEnd, 2) => {
//                 self.set_new_state(&BasicState::Idle);
//             }
//             _ => {}
//         }
//     }

//     fn update_physics(&self, velocity: &mut Velocity, input: &PlayerInput) {
//         match (self.state, self.frame_count) {
//             (BasicState::JumpSquat, Self::JUMPSQUAT) => {
//                 velocity.0.y += 10.0;
//                 // pos.0 += Vec2::new(0.0, 1.0);
//             }
//             (BasicState::Airdodge, 0) => {
//                 let control = if input.control.length() > CONTROL_STICK_DEADZONE {
//                     input.control.normalize_or_zero()
//                 } else {
//                     Vec2::ZERO
//                 };
//                 velocity.0 = control * Self::AIRDODGE_INITIAL_SPEED;
//             }
//             (BasicState::Airdodge, _) => {
//                 let speed_reduction_per_frame =
//                     Self::AIRDODGE_INITIAL_SPEED / (Self::AIRDODGE_DURATION_FRAMES as f32);
//                 let current_speed = velocity.0.length();
//                 if current_speed == 0.0 {
//                     return;
//                 }
//                 let desired_speed = current_speed - speed_reduction_per_frame;
//                 let ratio = desired_speed / current_speed;
//                 velocity.0 *= ratio;
//             }
//             (BasicState::Dash, 0) => {
//                 velocity.0.x = if input.control.x.is_sign_negative() {
//                     -Self::DASH_SPEED
//                 } else {
//                     Self::DASH_SPEED
//                 };
//             }
//             _ => {}
//         }
//     }

//     fn land(&mut self, pushback: &Vec2, velocity: &mut Velocity) {
//         let normal = pushback.normalize();
//         let modified_pushback = normal * (normal.dot(*pushback));
//         velocity.0 += modified_pushback;
//         match self.state {
//             BasicState::IdleAirborne => {
//                 self.set_new_state(&BasicState::LandCrouch);
//             }
//             BasicState::Airdodge => {
//                 self.set_new_state(&BasicState::LandCrouch);
//             }
//             _ => {}
//         }
//     }

//     fn go_airborne(&mut self) {
//         if !self.state.is_grounded() {
//             return;
//         }
//         self.set_new_state(&BasicState::IdleAirborne);
//     }

//     fn apply_input(&mut self, input: &PlayerInput) -> BufferResult {
//         let button = input.buffered_action.map(|x| x.action);
//         match (self.state, button, input.dash) {
//             (BasicState::Idle, _, Some(direction))
//             | (BasicState::Turnaround, _, Some(direction)) => {
//                 self.facing = direction;
//                 self.set_new_state(&BasicState::Dash);
//                 BufferResult::Rejected
//             }
//             (BasicState::Dash, _, Some(direction)) => {
//                 if direction != self.facing {
//                     self.facing = direction;
//                     self.set_new_state(&BasicState::Dash);
//                 }
//                 BufferResult::Rejected
//             }
//             (BasicState::Idle, Some(PlayerButton::Jump), _)
//             | (BasicState::Dash, Some(PlayerButton::Jump), _)
//             | (BasicState::Run, Some(PlayerButton::Jump), _)
//             | (BasicState::Walk, Some(PlayerButton::Jump), _)
//             | (BasicState::Turnaround, Some(PlayerButton::Jump), _) => {
//                 self.set_new_state(&BasicState::JumpSquat);
//                 BufferResult::Accepted
//             }
//             (BasicState::Idle, None, _) => {
//                 let control_x = input.control.x;
//                 if control_x.abs() < HORIZONTAL_DEADZONE {
//                     return BufferResult::Rejected;
//                 }
//                 let control_direction = if control_x < 0.0 {
//                     LeftRight::Left
//                 } else {
//                     LeftRight::Right
//                 };
//                 if control_direction != self.facing {
//                     self.set_new_state(&BasicState::Turnaround);
//                     BufferResult::Rejected
//                 } else {
//                     self.set_new_state(&BasicState::Walk);
//                     BufferResult::Rejected
//                 }
//             }
//             (BasicState::Run, None, _) => {
//                 let control_x = input.control.x;
//                 if control_x.abs() < HORIZONTAL_DEADZONE {
//                     self.set_new_state(&BasicState::RunEnd);
//                     return BufferResult::Rejected;
//                 }
//                 let control_direction = if control_x < 0.0 {
//                     LeftRight::Left
//                 } else {
//                     LeftRight::Right
//                 };
//                 if control_direction != self.facing {
//                     self.set_new_state(&BasicState::RunTurnaround);
//                 }
//                 BufferResult::Rejected
//             }
//             (BasicState::RunEnd, None, _) => {
//                 let control_x = input.control.x;
//                 if control_x.abs() < HORIZONTAL_DEADZONE {
//                     return BufferResult::Rejected;
//                 }
//                 let control_direction = if control_x < 0.0 {
//                     LeftRight::Left
//                 } else {
//                     LeftRight::Right
//                 };
//                 if control_direction != self.facing {
//                     self.set_new_state(&BasicState::RunTurnaround);
//                 }
//                 BufferResult::Rejected
//             }
//             (BasicState::Walk, _, Some(direction)) => {
//                 if self.facing == direction {
//                     self.set_new_state(&BasicState::Dash);
//                 }
//                 BufferResult::Rejected
//             }
//             (BasicState::Walk, None, _) => {
//                 let control_x = input.control.x;
//                 if control_x.abs() < HORIZONTAL_DEADZONE {
//                     self.set_new_state(&BasicState::Idle);
//                     return BufferResult::Rejected;
//                 }
//                 let control_direction = if control_x < 0.0 {
//                     LeftRight::Left
//                 } else {
//                     LeftRight::Right
//                 };
//                 if control_direction != self.facing {
//                     self.set_new_state(&BasicState::Turnaround);
//                 }
//                 BufferResult::Rejected
//             }
//             (BasicState::IdleAirborne, Some(PlayerButton::Shield), _)
//             | (BasicState::JumpSquat, Some(PlayerButton::Shield), _) => {
//                 self.set_new_state(&BasicState::Airdodge);
//                 BufferResult::Accepted
//             }
//             _ => BufferResult::Rejected,
//         }
//     }

//     // Empty if no new data is to be given
//     fn get_animation_data(&self, velocity: &Velocity) -> AnimationUpdate {
//         match (self.state, self.frame_count) {
//             (BasicState::Idle, 200) => AnimationUpdate::MultiFrame {
//                 first: 0,
//                 last: 2,
//                 seconds_per_frame: 0.1,
//             },
//             (BasicState::Idle, 0) => AnimationUpdate::SingleFrame(0),
//             (BasicState::Turnaround, 0) => AnimationUpdate::SingleFrame(74),
//             (BasicState::IdleAirborne, _) => {
//                 let y = velocity.0.y;
//                 if y > 1.5 {
//                     AnimationUpdate::SingleFrame(18)
//                 } else if y > -1.5 {
//                     AnimationUpdate::SingleFrame(19)
//                 } else {
//                     AnimationUpdate::MultiFrame {
//                         first: 20,
//                         last: 21,
//                         seconds_per_frame: 0.15,
//                     }
//                 }
//             }
//             (BasicState::LandCrouch, 0) => AnimationUpdate::SingleFrame(133),
//             (BasicState::JumpSquat, 0) => AnimationUpdate::SingleFrame(133),
//             (BasicState::Walk, 0) => AnimationUpdate::MultiFrame {
//                 first: 5,
//                 last: 14,
//                 seconds_per_frame: 0.1,
//             },
//             (BasicState::Airdodge, 0) => AnimationUpdate::SingleFrame(33),
//             (BasicState::Dash, 0) => AnimationUpdate::SingleFrame(24),
//             (BasicState::RunTurnaround, 0) => AnimationUpdate::SingleFrame(30),
//             (BasicState::Run, 0) => AnimationUpdate::MultiFrame {
//                 first: 5,
//                 last: 14,
//                 seconds_per_frame: 0.1,
//             },
//             _ => AnimationUpdate::None,
//         }
//     }
// }

// #[derive(Bundle)]
// struct PlayerBundle {
//     tag: PlayerId,
//     position: Position,
//     velocity: Velocity,
//     facing: Facing,
//     target_horizontal_velocity: TargetHorizontalVelocity,
//     gravity: Gravity,
//     state_machine: PlayerStateMachine,
//     sprite_sheet_bundle: SpriteSheetBundle,
//     animation_indices: AnimationIndices,
//     animation_timer: AnimationTimer,
// }

// const INTANGIBLE_FLASH_PERIOD: FrameNumber = 4;

// fn update_intangible_flash(mut query: Query<(&mut Sprite, &PlayerStateMachine), With<Intangible>>) {
//     for (mut sprite, psm) in &mut query {
//         let period_number = psm.frame_count / INTANGIBLE_FLASH_PERIOD;
//         let lightened = period_number % 2 == 0;
//         if lightened {
//             sprite.color = Color::rgb(1.5, 1.5, 1.5);
//         } else {
//             sprite.color = Color::WHITE;
//         }
//     }
// }

// fn update_intangible_no_flash(mut query: Query<&mut Sprite, Without<Intangible>>) {
//     for mut sprite in &mut query {
//         sprite.color = Color::WHITE;
//     }
// }
