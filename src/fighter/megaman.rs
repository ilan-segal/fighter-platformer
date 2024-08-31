use super::{update_fighter_state, FighterProperties, FighterState, FighterStateTransition};
use bevy::prelude::*;

use crate::{
    fighter::{FighterEventSet, FighterStateUpdate},
    fighter_state::{StateEnd, IASA},
    hitbox::{
        Hitbox, HitboxBundle, HitboxGroup, HitboxGroupBundle, HitboxPurpose, KnockbackAngle, Shape,
    },
    input::Action,
    projectile::Projectile,
    utils::{Facing, FrameCount, FrameNumber, LeftRight, Lifetime},
    AnimationIndices, AnimationUpdate, AnimationUpdateEvent, Velocity,
};

const ATTACK_DURATION: FrameNumber = 20;
const ATTACK_SHOOT_FRAME: FrameNumber = 5;
const ATTACK_IASA: FrameNumber = 10;

pub const MEGAMAN_TRACTION: f32 = 0.5;
pub const MEGAMAN_JUMP_SPEED: f32 = 10.0;
pub const MEGAMAN_DASH_SPEED: f32 = 5.0;
pub const MEGAMAN_WALK_SPEED: f32 = 3.0;

#[derive(Component)]
pub struct MegaMan;

impl MegaMan {
    pub fn get_properties() -> FighterProperties {
        FighterProperties {
            walk_speed: 3.0,
            ground_friction: 0.3,
            gravity: -0.3,
            dash_duration: 10,
            land_crouch_duration: 6,
            jumpsquat_duration: 4,
            ..Default::default()
        }
    }

    pub fn spawn_body_hitboxes(child_builder: &mut ChildBuilder) {
        child_builder
            .spawn(HitboxGroupBundle {
                hitbox_group: HitboxGroup::default(),
                transform: TransformBundle::default(),
            })
            .with_children(|hitbox_group| {
                hitbox_group.spawn(HitboxBundle {
                    hitbox: Hitbox {
                        shape: Shape::Pill {
                            major_radius: 6.0,
                            minor_radius: 15.5,
                        },
                        purpose: HitboxPurpose::Body,
                    },
                    transform: TransformBundle {
                        local: Transform::from_xyz(-1.0, 20.75, 1.0),
                        ..Default::default()
                    },
                });
            });
    }
}

// fn update_state_for_frame_count(
//     q: Query<(Entity, &FighterState, &FrameCount), With<MegaMan>>,
//     mut ev_state: EventWriter<FighterStateUpdate>,
// ) {
//     for (e, state, FrameCount(frame)) in q.iter() {
//         match (state, *frame) {
//             (FighterState::Attack, ATTACK_DURATION) => {
//                 ev_state.send(FighterStateUpdate(e, FighterState::Idle));
//             }
//             _ => {}
//         }
//     }
// }

fn get_attack_transition<const STAGE: u8>() -> FighterStateTransition {
    FighterStateTransition {
        end: StateEnd::OnFrame {
            frame: ATTACK_DURATION,
            next_state: FighterState::Idle,
        },
        iasa: if STAGE < MAX_ATTACK_STAGE {
            Some(IASA {
                frame: ATTACK_IASA,
                state_getter: |data| {
                    if data.control.has_action(&Action::Attack) {
                        Some(FighterState::Attack(STAGE + 1))
                    } else {
                        None
                    }
                },
            })
        } else {
            None
        },
    }
}

fn update_state_transition_rules(
    mut q: Query<
        (&mut FighterStateTransition, &FighterState),
        (With<MegaMan>, Changed<FighterState>),
    >,
) {
    for (mut transition, state) in q.iter_mut() {
        *transition = match state {
            FighterState::Attack(0) => get_attack_transition::<0>(),
            FighterState::Attack(1) => get_attack_transition::<1>(),
            FighterState::Attack(..) => get_attack_transition::<MAX_ATTACK_STAGE>(),
            _ => FighterStateTransition::default_for_state(state),
        };
        debug!("{:?}", transition);
    }
}

fn emit_animation_update(
    q: Query<(Entity, &FighterState, &FrameCount, &Velocity), With<MegaMan>>,
    mut ev_animation: EventWriter<AnimationUpdateEvent>,
    mut ev_state: EventWriter<FighterStateUpdate>,
) {
    for (e, state, frame, velocity) in &q {
        if let Some(basic_update) = animation_for_state(state).map(|u| AnimationUpdateEvent(e, u)) {
            ev_animation.send(basic_update);
            continue;
        }
        if let Some(update) = match (state, frame.0) {
            // Blinky blinky
            (FighterState::Idle, 200) => Some(AnimationUpdate::MultiFrame {
                indices: AnimationIndices { first: 0, last: 2 },
                seconds_per_frame: 0.1,
            }),
            (FighterState::Idle, 240) => {
                ev_state.send(FighterStateUpdate(e, FighterState::Idle));
                None
            }
            (FighterState::IdleAirborne, _) => {
                let y = velocity.0.y;
                if y > 1.5 {
                    Some(AnimationUpdate::SingleFrame(18))
                } else if y > -1.5 {
                    Some(AnimationUpdate::SingleFrame(19))
                } else {
                    Some(AnimationUpdate::MultiFrame {
                        indices: AnimationIndices {
                            first: 20,
                            last: 21,
                        },
                        seconds_per_frame: 0.15,
                    })
                }
            }
            _ => None,
        } {
            let event = AnimationUpdateEvent(e, update);
            ev_animation.send(event);
        }
    }
}

fn animation_for_state(state: &FighterState) -> Option<AnimationUpdate> {
    match state {
        FighterState::Idle => Some(AnimationUpdate::SingleFrame(0)),
        FighterState::JumpSquat
        | FighterState::LandCrouch
        | FighterState::EnterCrouch
        | FighterState::ExitCrouch => Some(AnimationUpdate::SingleFrame(133)),
        FighterState::Crouch => Some(AnimationUpdate::SingleFrame(134)),
        FighterState::Walk => Some(AnimationUpdate::MultiFrame {
            indices: AnimationIndices { first: 5, last: 14 },
            seconds_per_frame: 0.1,
        }),
        FighterState::Airdodge(..) => Some(AnimationUpdate::SingleFrame(33)),
        FighterState::Dash => Some(AnimationUpdate::SingleFrame(24)),
        FighterState::Turnaround => Some(AnimationUpdate::SingleFrame(74)),
        FighterState::RunTurnaround => Some(AnimationUpdate::SingleFrame(30)),
        FighterState::Run => Some(AnimationUpdate::MultiFrame {
            indices: AnimationIndices { first: 5, last: 14 },
            seconds_per_frame: 0.1,
        }),
        FighterState::Attack(..) => Some(AnimationUpdate::SingleFrame(43)),
        _ => None,
    }
}

#[derive(Resource)]
struct LemonSprite(Option<Handle<Image>>);

fn load_lemon_sprite(mut res: ResMut<LemonSprite>, asset_server: Res<AssetServer>) {
    res.0 = Some(asset_server.load("sprites/megaman/lemon.png"));
}

#[derive(Bundle)]
struct LemonBundle {
    sprite: SpriteBundle,
    velocity: Velocity,
    lifetime: Lifetime,
    hitbox_group: HitboxGroup,
    projectile: Projectile,
}

const LEMON_VELOCITY: f32 = 10.0;
const LEMON_DISTANCE: f32 = 450.0;

impl LemonBundle {
    fn new(owner: Entity, texture: Handle<Image>, facing: &Facing, transform: Transform) -> Self {
        let vx = match facing.0 {
            LeftRight::Left => -LEMON_VELOCITY,
            LeftRight::Right => LEMON_VELOCITY,
        };
        let lifetime = (LEMON_DISTANCE / LEMON_VELOCITY) as FrameNumber;
        LemonBundle {
            sprite: SpriteBundle {
                texture,
                transform,
                ..Default::default()
            },
            velocity: Velocity(Vec2::new(vx, 0.0)),
            lifetime: Lifetime(lifetime),
            hitbox_group: HitboxGroup::ignoring(&owner),
            projectile: Projectile,
        }
    }
}

const MAX_ATTACK_STAGE: u8 = 2;

fn shoot_lemon(
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &FighterState,
        &FrameCount,
        &GlobalTransform,
        &Facing,
    )>,
    lemon_sprite: Res<LemonSprite>,
) {
    for (entity, state, FrameCount(frame), global_transform, facing) in q.iter_mut() {
        let FighterState::Attack(..) = state else {
            continue;
        };

        if frame != &ATTACK_SHOOT_FRAME {
            continue;
        }

        let lemon_position = Vec3::new(20.0, 23.0, 10.0);
        let mut transform = global_transform.compute_transform();
        transform.translation += lemon_position * transform.scale;
        // transform.scale.x *= facing.0.get_sign();

        commands
            .spawn(LemonBundle::new(
                entity,
                lemon_sprite.0.clone().unwrap(),
                facing,
                transform,
            ))
            .with_children(|parent| {
                parent.spawn(HitboxBundle {
                    // transform: TransformBundle {
                    //     local: Transform::from_scale(Vec3::new(
                    //         match facing.0 {
                    //             LeftRight::Left => -1.0,
                    //             LeftRight::Right => 1.0,
                    //         },
                    //         1.0,
                    //         1.0,
                    //     )),
                    //     ..Default::default()
                    // },
                    hitbox: Hitbox {
                        shape: Shape::Circle(5.0),
                        purpose: HitboxPurpose::Damage {
                            percent: 3.0,
                            base_knockback: 0.1,
                            scale_knockback: 5.0,
                            angle: KnockbackAngle::Fixed(45.0),
                        },
                    },
                    ..Default::default()
                });
            });
    }
}

pub struct MegaManPlugin;
impl Plugin for MegaManPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(LemonSprite(None))
            .add_systems(Startup, load_lemon_sprite)
            .add_systems(
                FixedUpdate,
                (
                    (
                        // update_state_for_frame_count,
                        shoot_lemon,
                        emit_animation_update,
                    )
                        .chain()
                        .in_set(FighterEventSet::Act),
                    update_state_transition_rules.after(update_fighter_state),
                ),
            );
    }
}
