use bevy::{input::gamepad::*, prelude::*, utils::petgraph::matrix_graph::Zero};
use enumset::{EnumSet, EnumSetType};
use std::collections::HashMap;

use crate::{fighter::Player, utils::FrameNumber};

const BUFFER_SIZE: FrameNumber = 4;

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

#[derive(Event, Debug)]
pub struct ActionEvent(pub Entity, pub Action);

fn buffer_actions(
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
            if event.value < 1.0 {
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
                mapping
                    .map_button(&button_type)
                    .map(|action| (e, action))
            })
            .next()
        {
            commands
                .entity(e)
                .insert(Buffer { action, age: 0 });
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
            (
                (update_control_state, buffer_actions),
                age_buffer,
                emit_action_events,
            )
                .chain()
                .in_set(InputSet),
        )
        .add_event::<ActionEvent>();
    }
}
