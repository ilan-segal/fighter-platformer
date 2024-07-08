use bevy::{
    input::{gamepad::*, keyboard::*},
    prelude::*,
};
use enumset::{EnumSet, EnumSetType};
use std::collections::HashMap;

use crate::{fighter::Player, utils::FrameNumber};

const BUFFER_SIZE: FrameNumber = 16;

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

trait ButtonMapper<T> {
    fn map_button(&self, button: &T) -> Option<Action>;
}

impl ButtonMapper<GamepadButtonType> for Option<&GamepadButtonMapping> {
    fn map_button(&self, button: &GamepadButtonType) -> Option<Action> {
        if let Some(mapping) = self {
            return mapping.0.get(button).copied();
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

#[derive(Component)]
pub struct KeyboardButtonMapping(HashMap<KeyCode, Action>);

impl ButtonMapper<KeyCode> for Option<&KeyboardButtonMapping> {
    fn map_button(&self, button: &KeyCode) -> Option<Action> {
        if let Some(mapping) = self {
            return mapping.0.get(button).copied();
        }
        match button {
            KeyCode::Space => Some(Action::Jump),
            _ => None,
        }
    }
}

fn update_control_state_from_gamepad(
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
                        if event.value == 0.0 {
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

fn update_control_state_from_keyboard(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut control: Query<(
        Entity,
        &Player,
        &mut Control,
        Option<&KeyboardButtonMapping>,
    )>,
    mut commands: Commands,
) {
    if let Ok((e, _, mut control, mapping)) = control.get_single_mut() {
        keyboard
            .get_just_pressed()
            .filter_map(|k| mapping.map_button(k))
            .for_each(|action| {
                control.held_actions.insert(action);
                debug!("{:?}", control);
                commands.entity(e).insert(Buffer { action, age: 0 });
            });
        keyboard
            .get_just_released()
            .filter_map(|k| mapping.map_button(k))
            .for_each(|action| {
                control.held_actions.remove(action);
                debug!("{:?}", control);
            })
    }
}

#[derive(Component)]
pub struct Buffer {
    pub action: Action,
    pub age: FrameNumber,
}

fn age_buffer(mut q: Query<(Entity, &mut Buffer)>, mut commands: Commands) {
    for (e, mut b) in &mut q {
        b.age += 1;
        if b.age == BUFFER_SIZE {
            commands.entity(e).remove::<Buffer>();
        }
    }
}

#[derive(Event)]
pub struct ClearBuffer(pub Entity);

fn consume_buffer(mut ev: EventReader<ClearBuffer>, mut commands: Commands) {
    ev.read().map(|event| event.0).for_each(|e| {
        commands.entity(e).remove::<Buffer>();
        debug!("Removed buffer for {:?}", e);
    });
}

#[derive(Event, Debug)]
pub struct ActionEvent(pub Entity, pub Action);

fn buffer_actions_from_gamepad(
    mut commands: Commands,
    mut ev_gamepad: EventReader<GamepadEvent>,
    player: Query<(Entity, &Player, Option<&GamepadButtonMapping>)>,
) {
    for (player_id, button_type) in ev_gamepad
        .read()
        .filter_map(|event| match event {
            GamepadEvent::Button(e) => Some(e),
            _ => None,
        })
        .filter_map(|event| {
            if event.value == 0.0 {
                None
            } else {
                Some((event.gamepad.id, event.button_type))
            }
        })
    {
        if let Some((e, action)) = player
            .iter()
            .filter(|(_, p, _)| p.0 == player_id)
            .filter_map(|(e, _, mapping)| {
                mapping.map_button(&button_type).map(|action| (e, action))
            })
            .next()
        {
            commands.entity(e).insert(Buffer { action, age: 0 });
        }
    }
}

fn emit_action_events(mut ev_action: EventWriter<ActionEvent>, player: Query<(Entity, &Buffer)>) {
    player
        .iter()
        .map(|(entity, buffer)| ActionEvent(entity, buffer.action))
        .for_each(|event| {
            ev_action.send(event);
        });
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            FixedUpdate,
            (consume_buffer, age_buffer, emit_action_events)
                .chain()
                .in_set(InputSet),
        )
        .add_systems(
            Update,
            (
                update_control_state_from_gamepad,
                update_control_state_from_keyboard,
                buffer_actions_from_gamepad,
            ),
        )
        .add_event::<ActionEvent>()
        .add_event::<ClearBuffer>();
    }
}
