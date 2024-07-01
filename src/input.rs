use bevy::{input::gamepad::*, prelude::*, utils::petgraph::matrix_graph::Zero};
use enumset::{EnumSet, EnumSetType};
use std::collections::HashMap;

use crate::fighter::Player;

#[derive(EnumSetType, Debug)]
pub enum Action {
    Attack,
    Special,
    Shield,
    Grab,
    Jump,
    Taunt,
}

#[derive(Component, Default, Clone, Copy, Debug)]
pub struct Control {
    pub stick: Vec2,
    pub held_actions: EnumSet<Action>,
}

#[derive(Component)]
pub struct GamepadButtonMapping(HashMap<GamepadButtonType, Action>);

trait ButtonMapper {
    fn map_button(&self, button: &GamepadButtonType) -> Option<Action>;
}

impl ButtonMapper for GamepadButtonMapping {
    fn map_button(&self, button: &GamepadButtonType) -> Option<Action> {
        self.0.get(button).copied()
    }
}

impl ButtonMapper for Option<&GamepadButtonMapping> {
    fn map_button(&self, button: &GamepadButtonType) -> Option<Action> {
        if let Some(mapping) = self {
            return mapping.map_button(button);
        }
        match button {
            GamepadButtonType::North | GamepadButtonType::West => Some(Action::Jump),
            GamepadButtonType::East => Some(Action::Attack),
            GamepadButtonType::South => Some(Action::Special),
            GamepadButtonType::LeftTrigger
            | GamepadButtonType::RightTrigger
            | GamepadButtonType::Z => Some(Action::Grab),
            GamepadButtonType::LeftTrigger2 | GamepadButtonType::RightTrigger2 => {
                Some(Action::Shield)
            }
            GamepadButtonType::DPadUp
            | GamepadButtonType::DPadDown
            | GamepadButtonType::DPadLeft
            | GamepadButtonType::DPadRight => Some(Action::Taunt),
            _ => None,
        }
    }
}

fn update_control_state(
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
                debug!("Button event: {:?}", event);
                let id = event.gamepad.id;
                if let Some((_, mut control, mapping)) = control
                    .iter_mut()
                    .filter(|(player, ..)| player.0 == id)
                    .next()
                {
                    if let Some(action) = mapping.map_button(&event.button_type) {
                        if event.value.is_zero() {
                            control.held_actions.remove(action);
                        } else {
                            control.held_actions.insert(action);
                        }
                    }
                    debug!("{:?}", control);
                }
            }
            _ => {}
        }
    }
}

#[derive(Event, Debug)]
pub struct ActionEvent(pub Entity, pub Action);

fn emit_action_events(
    mut ev_gamepad: EventReader<GamepadEvent>,
    mut ev_action: EventWriter<ActionEvent>,
    player: Query<(Entity, &Player, Option<&GamepadButtonMapping>)>,
) {
    for (player_id, button_type) in ev_gamepad
        .read()
        .filter_map(|event| match event {
            GamepadEvent::Button(e) => Some(e),
            _ => None,
        })
        .filter_map(|event| {
            if event.value < 1.0 {
                None
            } else {
                Some((event.gamepad.id, event.button_type))
            }
        })
    {
        if let Some((entity, _, mapping)) = player
            .iter()
            .filter(|(_, p, _)| p.0 == player_id)
            .next()
        {
            if let Some(action) = mapping.map_button(&button_type) {
                ev_action.send(ActionEvent(entity, action));
            }
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
        app.add_systems(
            FixedUpdate,
            (update_control_state, emit_action_events)
                .chain()
                .in_set(InputSet),
        )
        .add_event::<ActionEvent>();
    }
}
