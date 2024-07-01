use bevy::{input::gamepad::*, prelude::*};
use enumset::{EnumSet, EnumSetType};
use std::collections::HashMap;

use crate::fighter::Player;

#[derive(EnumSetType)]
pub enum Action {
    Attack,
    Special,
    Shield,
    Grab,
    Jump,
    Taunt,
}

#[derive(Component, Default, Clone, Copy)]
pub struct Control {
    pub stick: Vec2,
    pub held_actions: EnumSet<Action>,
}

#[derive(Component)]
pub struct GamepadButtonMapping(HashMap<GamepadButtonType, Action>);

impl GamepadButtonMapping {
    fn map(&self, button: &GamepadButtonType) -> Option<Action> {
        self.0.get(button).copied()
    }

    fn default_map(button: &GamepadButtonType) -> Option<Action> {
        match button {
            GamepadButtonType::North | GamepadButtonType::West => Some(Action::Jump),
            GamepadButtonType::East => Some(Action::Attack),
            GamepadButtonType::South => Some(Action::Special),
            GamepadButtonType::LeftTrigger | GamepadButtonType::RightTrigger => {
                Some(Action::Shield)
            }
            GamepadButtonType::LeftTrigger2
            | GamepadButtonType::RightTrigger2
            | GamepadButtonType::Z => Some(Action::Grab),
            GamepadButtonType::DPadUp
            | GamepadButtonType::DPadDown
            | GamepadButtonType::DPadLeft
            | GamepadButtonType::DPadRight => Some(Action::Taunt),
            _ => None,
        }
    }
}

fn read_input_events(
    mut ev_gamepad: EventReader<GamepadEvent>,
    mut control: Query<(&Player, &mut Control, Option<&GamepadButtonMapping>)>,
) {
    for ev in ev_gamepad.read() {
        match ev {
            GamepadEvent::Axis(GamepadAxisChangedEvent {
                axis_type,
                value,
                gamepad,
            }) => {
                let id = gamepad.id;
                if let Some((_, mut control, _)) = control
                    .iter_mut()
                    .filter(|(player, ..)| player.0 == id)
                    .next()
                {
                    match axis_type {
                        GamepadAxisType::LeftStickX => {
                            control.stick.x = *value;
                        }
                        GamepadAxisType::LeftStickY => {
                            control.stick.y = *value;
                        }
                        _ => {}
                    }
                }
            }
            GamepadEvent::Button(event) => {
                log::info!("Button event: {:?}", event);
                let id = event.gamepad.id;
                if let Some((_, mut control, maybe_mapping)) = control
                    .iter_mut()
                    .filter(|(player, ..)| player.0 == id)
                    .next()
                {
                    if let Some(action) = maybe_mapping
                        .and_then(|m| m.map(&event.button_type))
                        .or_else(|| GamepadButtonMapping::default_map(&event.button_type))
                    {
                        if event.value.is_sign_positive() {
                            control.held_actions.insert(action);
                        } else {
                            control.held_actions.remove(action);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

// pub enum FighterAction {
//     Jab,
//     UpTilt,
//     ForwardTilt,
//     DownTilt,
//     NeutralAir,
//     UpAir,
//     ForwardAir,
//     BackAir,
//     DownAir,
//     UpSmash,
//     ForwardSmash,
//     DownSmash,
//     Grab,
//     NeutralSpecial,
//     UpSpecial,
//     ForwardSpecial,
//     DownSpecial,
//     Shield,
//     Spotdodge,
//     Roll,
//     Dash,
//     Jump,
//     FastFall,
//     HitFall,
//     Airdodge,
// }

// #[derive(Event)]
// pub struct FighterInput(Entity, FighterAction);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(FixedUpdate, read_input_events.in_set(InputSet));
    }
}
