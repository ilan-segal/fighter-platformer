use super::{
    FacingUpdate, FighterState, FighterStateMachine, FighterStateUpdate, IntangibleUpdate,
};
use bevy::{
    app::{FixedUpdate, Plugin},
    prelude::{Component, Entity, EventWriter, IntoSystemConfigs, Query, With},
};

use crate::{
    fighter,
    utils::{FrameCount, FrameNumber},
    AnimationIndices, AnimationUpdate, AnimationUpdateEvent, Facing,
};

const LANDING_LAG: FrameNumber = 12;
const IDLE_CYCLE: FrameNumber = 240;
const JUMPSQUAT: FrameNumber = 5;
const FRICTION: f32 = 0.3;
const WALK_SPEED: f32 = 3.0;
const DASH_SPEED: f32 = 5.0;
const DASH_DURATION: FrameNumber = 20;

#[derive(Component)]
pub struct MegaMan;

impl FighterStateMachine for MegaMan {
    fn dash_duration(&self) -> FrameNumber {
        DASH_DURATION
    }

    fn dash_speed(&self) -> f32 {
        DASH_SPEED
    }

    fn land_crouch_duration(&self) -> FrameNumber {
        LANDING_LAG
    }

    fn idle_cycle_duration(&self) -> FrameNumber {
        IDLE_CYCLE
    }

    fn jumpsquat(&self) -> FrameNumber {
        JUMPSQUAT
    }

    fn jump_speed(&self) -> f32 {
        10.0
    }
}

// pub fn compute_side_effects(
//     query: Query<(Entity, &FighterState, &FrameCount, &Facing), With<MegaMan>>,
//     mut ev_state: EventWriter<FighterStateUpdate>,
//     mut ev_facing: EventWriter<FacingUpdate>,
// ) {
//     for (entity, state, frame, facing) in query.iter() {}
// }

fn emit_animation_update(
    q: Query<(Entity, &FighterState, &FrameCount), With<MegaMan>>,
    mut ev_animation: EventWriter<AnimationUpdateEvent>,
) {
    for (e, state, frame) in &q {
        if let Some(update) = match (state, frame.0) {
            (FighterState::Idle, 0) => Some(AnimationUpdate::SingleFrame(0)),
            // Blinky blinky
            (FighterState::Idle, 200) => Some(AnimationUpdate::MultiFrame {
                indices: AnimationIndices { first: 0, last: 2 },
                seconds_per_frame: 0.1,
            }),
            (FighterState::LandCrouch, 0) => Some(AnimationUpdate::SingleFrame(133)),

            _ => None,
        } {
            let event = AnimationUpdateEvent(e, update);
            log::info!("{:?}", event);
            ev_animation.send(event);
        }
    }
}

pub struct MegaManPlugin;
impl Plugin for MegaManPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        use bevy_trait_query::RegisterExt;

        app.register_component_as::<dyn FighterStateMachine, MegaMan>()
            .add_systems(
                FixedUpdate,
                emit_animation_update
                    .after(fighter::update_fighter_state)
                    .before(crate::update_frame_count)
                    .before(crate::view::update_animation_data),
            );
    }
}
