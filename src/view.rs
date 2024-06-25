use bevy::{
    app::{Plugin, Update},
    prelude::{
        Component, Deref, DerefMut, Entity, Event, EventReader, Query, Res, Sprite, TextureAtlas,
        Time, Timer, TimerMode, Transform,
    },
};

use crate::physics::Position;
use crate::utils::{Facing, FrameNumber, LeftRight};

#[derive(Component, Clone)]
pub struct AnimationIndices {
    pub first: FrameNumber,
    pub last: FrameNumber,
}
pub enum AnimationUpdate {
    SingleFrame(FrameNumber),
    MultiFrame {
        indices: AnimationIndices,
        seconds_per_frame: f32,
    },
}

#[derive(Event)]
pub struct AnimationUpdateEvent(pub Entity, pub AnimationUpdate);

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (indices, mut timer, mut atlas) in &mut query {
        timer.tick(time.delta());
        if timer.just_finished() {
            atlas.index = if atlas.index == indices.last as usize {
                indices.first as usize
            } else {
                atlas.index + 1
            };
        }
    }
}

fn update_animation_data(
    mut ev_update: EventReader<AnimationUpdateEvent>,
    mut q: Query<(
        &mut AnimationIndices,
        &mut AnimationTimer,
        &mut TextureAtlas,
    )>,
) {
    for event in ev_update.read() {
        let id = event.0;
        let Ok((mut idx, mut timer, mut atlas)) = q.get_mut(id) else {
            continue;
        };
        match &event.1 {
            AnimationUpdate::SingleFrame(frame) => {
                idx.first = *frame;
                idx.last = *frame;
                *timer = AnimationTimer(Timer::from_seconds(0.0, TimerMode::Once));
            }
            AnimationUpdate::MultiFrame {
                indices,
                seconds_per_frame,
            } => {
                *idx = indices.clone();
                *timer = AnimationTimer(Timer::from_seconds(
                    *seconds_per_frame,
                    TimerMode::Repeating,
                ));
                atlas.index = indices.first as usize;
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

fn align_sprites_with_facing(mut query: Query<(&Facing, &mut Sprite)>) {
    for (facing, mut sprite) in &mut query {
        sprite.flip_x = facing.0 == LeftRight::Left;
    }
}

pub struct ViewPlugin;
impl Plugin for ViewPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            Update,
            (
                animate_sprite,
                update_animation_data,
                snap_texture_to_position,
                align_sprites_with_facing,
            ),
        )
        .add_event::<AnimationUpdateEvent>();
    }
}
