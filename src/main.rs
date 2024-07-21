#![feature(if_let_guard)]
#![feature(let_chains)]

use bevy::{log::LogPlugin, prelude::*, sprite::Anchor};
use input::{Control, InputSet};
use iyes_perf_ui::prelude::*;

mod fighter;
mod input;
mod physics;
mod utils;
mod view;

use fighter::{megaman::MegaMan, FighterBundle, FighterEventSet, Player as PlayerId};
use physics::*;
use utils::{Facing, FrameCount, FrameNumber, LeftRight};
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
                    update_subscriber: None,
                }),
            bevy::diagnostic::FrameTimeDiagnosticsPlugin,
            bevy::diagnostic::EntityCountDiagnosticsPlugin,
            bevy::diagnostic::SystemInformationDiagnosticsPlugin,
            PerfUiPlugin,
            input::InputPlugin,
            view::ViewPlugin,
            fighter::FighterPlugin,
            physics::PhysicsPlugin,
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
            velocity: Velocity(Vec2::new(5.0, 0.0)),
            state: fighter::FighterState::default(),
            sprite_sheet_bundle,
            animation_indices,
            animation_timer,
            control: Control::default(),
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
