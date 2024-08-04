use bevy::prelude::*;

use crate::fighter::Intangible;
use crate::utils::{Facing, FrameCount, FrameNumber, LeftRight};

#[derive(Component, Clone, Debug, PartialEq, Eq)]
pub struct AnimationIndices {
    pub first: FrameNumber,
    pub last: FrameNumber,
}

#[derive(Debug)]
pub enum AnimationUpdate {
    SingleFrame(FrameNumber),
    MultiFrame {
        indices: AnimationIndices,
        seconds_per_frame: f32,
    },
}

#[derive(Event, Debug)]
pub struct AnimationUpdateEvent(pub Entity, pub AnimationUpdate);

#[derive(Component, Deref, DerefMut)]
pub struct AnimationTimer(pub Timer);

fn animate_sprite(
    time: Res<Time>,
    mut query: Query<(&AnimationIndices, &mut AnimationTimer, &mut TextureAtlas)>,
) {
    for (indices, mut timer, mut atlas) in &mut query {
        if atlas.index < indices.first {
            atlas.index = indices.first;
        } else if atlas.index > indices.last {
            atlas.index = indices.last;
        }
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

pub fn update_animation_data(
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
            warn!("No valid entity {:?}", id);
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
            } if *indices != *idx || timer.0.duration().as_secs_f32() != *seconds_per_frame => {
                *idx = indices.clone();
                *timer = AnimationTimer(Timer::from_seconds(
                    *seconds_per_frame,
                    TimerMode::Repeating,
                ));
                atlas.index = indices.first as usize;
            }
            _ => {}
        }
    }
}

fn align_sprites_with_facing(mut query: Query<(&Facing, &mut Transform)>) {
    for (facing, mut transform) in &mut query {
        let desired_sign = match facing.0 {
            LeftRight::Left => -1.0,
            LeftRight::Right => 1.0,
        };
        let current_sign = transform.scale.x.signum();
        if current_sign != desired_sign {
            transform.scale.x *= -1.0;
        }
    }
}

fn update_intangibility_flash(mut query: Query<(&mut Sprite, &FrameCount, Option<&Intangible>)>) {
    for (mut sprite, frame, maybe_intangible) in query.iter_mut() {
        if maybe_intangible.is_some() && (frame.0 / 3) % 2 == 0 {
            sprite.color = Color::WHITE * 1.25;
        } else {
            sprite.color = Color::WHITE;
        }
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct ViewSet;

pub struct ViewPlugin;
impl Plugin for ViewPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            FixedUpdate,
            (
                update_intangibility_flash,
                update_animation_data,
                align_sprites_with_facing,
            )
                .chain()
                .in_set(ViewSet),
        )
        .add_systems(Update, animate_sprite)
        .add_event::<AnimationUpdateEvent>();
    }
}
