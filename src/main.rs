#![feature(if_let_guard)]
#![feature(let_chains)]

use bevy::{log::LogPlugin, prelude::*, sprite::Anchor};
use input::{Control, InputSet};
use iyes_perf_ui::prelude::*;

mod fighter;
mod fighter_state;
mod hitbox;
mod input;
mod physics;
mod projectile;
mod utils;
mod view;

use fighter::{
    megaman::MegaMan, DashSpeed, FighterBundle, FighterEventSet, JumpSpeed, Percent,
    Player as PlayerId, RunSpeed, Traction, WalkSpeed, Weight,
};
use fighter_state::FighterStateTransition;
use physics::*;
use utils::{DebugMode, Facing, FrameCount, FrameNumber, LeftRight, VisibleDuringDebug};
use view::*;

const FRAMES_PER_SECOND: FrameNumber = 60;

fn main() {
    debug!("Starting...");
    App::new()
        .add_plugins((
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(LogPlugin {
                    level: bevy::log::Level::INFO,
                    filter: "fighter_platformer=debug".to_string(),
                    ..Default::default()
                }),
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
            PerfUiPlugin,
            input::InputPlugin,
            view::ViewPlugin,
            fighter::FighterPlugin,
            physics::PhysicsPlugin,
            hitbox::HitboxPlugin,
            utils::DebugPlugin,
            utils::LifetimePlugin,
            projectile::ProjectilePlugin,
        ))
        .insert_resource(Time::<Fixed>::from_hz(FRAMES_PER_SECOND as f64))
        .add_systems(Startup, setup)
        .add_systems(FixedUpdate, increment_frame_number)
        .configure_sets(
            FixedUpdate,
            (
                InputSet,
                FighterEventSet::Act,
                PhysicsSet,
                FighterEventSet::React,
                ViewSet,
            )
                .chain()
                .before(increment_frame_number),
        )
        .run();
}

fn increment_frame_number(mut query: Query<&mut FrameCount>) {
    query
        .iter_mut()
        .for_each(|mut frame_count| {
            frame_count.0 += 1;
        });
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut texture_atlas_layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut debug_mode: ResMut<DebugMode>,
) {
    debug_mode.0 = true;
    let texture = asset_server.load("spritesheet/x3_2.png");
    let layout = TextureAtlasLayout::from_grid(UVec2::new(80, 73), 12, 12, None, None);
    let texture_atlas_layout = texture_atlas_layouts.add(layout);
    // Use only the subset of sprites in the sheet that make up the run animation
    let animation_indices = AnimationIndices {
        first: 1,
        last: 137,
    };
    let animation_timer = AnimationTimer(Timer::from_seconds(0.1, TimerMode::Repeating));
    let sprite_sheet_bundle = (
        SpriteBundle {
            texture,
            sprite: Sprite {
                anchor: Anchor::BottomCenter,
                ..default()
            },
            transform: Transform::from_scale(Vec3::splat(2.0)),
            ..default()
        },
        TextureAtlas {
            layout: texture_atlas_layout,
            ..default()
        },
    );
    // let state_machine = PlayerStateMachine::default();

    commands.spawn(Camera2dBundle::default());
    commands
        .spawn((
            FighterBundle {
                tag: PlayerId(0),
                frame: FrameCount(0),
                facing: Facing(LeftRight::Right),
                velocity: Velocity(Vec2::new(5.0, 0.0)),
                state: fighter_state::FighterState::default(),
                state_transition_properties: FighterStateTransition::default(),
                animation_indices: animation_indices.clone(),
                animation_timer: view::AnimationTimer(animation_timer.clone()),
                control: Control::default(),
                properties: MegaMan::get_properties(),
                percent: Percent::default(),
                weight: Weight::default(),
                traction: Traction(fighter::megaman::MEGAMAN_TRACTION),
                jump_speed: JumpSpeed(fighter::megaman::MEGAMAN_JUMP_SPEED),
                dash_speed: DashSpeed(fighter::megaman::MEGAMAN_DASH_SPEED),
                run_speed: RunSpeed(fighter::megaman::MEGAMAN_DASH_SPEED),
                walk_speed: WalkSpeed(fighter::megaman::MEGAMAN_WALK_SPEED),
            },
            sprite_sheet_bundle.clone(),
            MegaMan,
        ))
        .with_children(MegaMan::spawn_body_hitboxes);
    commands.spawn((
        SpriteBundle {
            transform: Transform {
                scale: Vec3::new(800.0, 1.0, 0.0),
                translation: Vec3::new(0.0, -200.0, 0.0),
                ..default()
            },
            sprite: Sprite {
                color: Color::WHITE,
                ..default()
            },
            ..default()
        },
        Collider {
            normal: Vec2::new(0.0, 1.0),
            breadth: 800.0,
        },
    ));
    commands.spawn((PerfUiCompleteBundle::default(), VisibleDuringDebug));
}
