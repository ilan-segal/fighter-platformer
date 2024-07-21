use bevy::{
    input::{gamepad::*, keyboard::*},
    prelude::*,
};
use enumset::{EnumSet, EnumSetType};
use std::collections::{HashMap, VecDeque};

use crate::{fighter::Player, utils::FrameNumber};

const BUFFER_SIZE: FrameNumber = 8;
const CONTROL_STICK_DEADZONE_SIZE: f32 = 0.5;
const CONTROL_STICK_LIVEZONE_SIZE: f32 = 1.0 - CONTROL_STICK_DEADZONE_SIZE;
const SMASH_INPUT_MAX_DURATION: FrameNumber = 4;
const SMASH_INPUT_THRESHOLD_DISTANCE_FROM_CENTRE: f32 = 0.99;

#[derive(EnumSetType, Debug)]
pub enum Action {
    Attack,
    Special,
    Shield,
    Grab,
    Jump,
    Taunt,
}

#[derive(Component, Default, Debug)]
pub struct Control {
    pub stick: Vec2,
    pub held_actions: EnumSet<Action>,
    previous_stick_positions: VecDeque<Vec2>,
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

fn get_clamped_control_stick(x: f32, y: f32) -> Vec2 {
    if x == 0.0 && y == 0.0 {
        return Vec2::ZERO;
    }
    let length = (x * x + y * y).sqrt();
    if length < CONTROL_STICK_DEADZONE_SIZE {
        return Vec2::ZERO;
    }
    let length_outsize_deadzone = length - CONTROL_STICK_DEADZONE_SIZE;
    let adjusted_length = (length_outsize_deadzone / CONTROL_STICK_LIVEZONE_SIZE).clamp(0.0, 1.0);
    return Vec2::new(x, y) / length * adjusted_length;
}

fn update_control_state_from_gamepad(
    gamepads: Res<Gamepads>,
    axes: Res<Axis<GamepadAxis>>,
    buttons: Res<ButtonInput<GamepadButton>>,
    mut control: Query<(&Player, &mut Control, Option<&GamepadButtonMapping>)>,
) {
    for (p, mut control, mapping) in control.iter_mut() {
        // Get gamepad for player
        let Some(gamepad) = gamepads
            .iter()
            .filter(|g| g.id == p.0)
            .next()
        else {
            continue;
        };

        // Update control stick
        let cur_stick = control.stick;
        control
            .previous_stick_positions
            .push_back(cur_stick);
        if control.previous_stick_positions.len() > SMASH_INPUT_MAX_DURATION {
            control
                .previous_stick_positions
                .pop_front();
        }
        let axis_lx = GamepadAxis {
            gamepad,
            axis_type: GamepadAxisType::LeftStickX,
        };
        let axis_ly = GamepadAxis {
            gamepad,
            axis_type: GamepadAxisType::LeftStickY,
        };
        if let (Some(x), Some(y)) = (axes.get(axis_lx), axes.get(axis_ly)) {
            let clamped = get_clamped_control_stick(x, y);
            control.stick = clamped;
        }

        // Update buttons
        for action in buttons
            .get_just_pressed()
            .filter(|gamepad_button| gamepad_button.gamepad.id == gamepad.id)
            .map(|gamepad_button| gamepad_button.button_type)
            .filter_map(|button| mapping.map_button(&button))
        {
            control.held_actions.insert(action);
        }
        for action in buttons
            .get_just_released()
            .filter(|gamepad_button| gamepad_button.gamepad.id == gamepad.id)
            .map(|gamepad_button| gamepad_button.button_type)
            .filter_map(|button| mapping.map_button(&button))
        {
            control.held_actions.remove(action);
        }
    }
}

#[derive(Component)]
pub struct KeyboardButtonMapping(HashMap<KeyCode, Action>);

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
                commands
                    .entity(e)
                    .insert(Buffer { action, age: 0 });
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

#[derive(Component, Debug)]
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

#[derive(Debug, PartialEq, Eq)]
pub enum DirectionalActionType {
    Smash,
    Clockwise,
    CounterClockwise,
}

#[derive(Component, Debug)]
pub struct DirectionalAction {
    pub action_type: DirectionalActionType,
    pub direction: Vec2,
    age: FrameNumber,
}

fn age_directional_action(mut q: Query<(Entity, &mut DirectionalAction)>, mut commands: Commands) {
    for (e, mut da) in q.iter_mut() {
        da.age += 1;
        if da.age >= BUFFER_SIZE {
            commands
                .entity(e)
                .remove::<DirectionalAction>();
        }
    }
}

fn detect_smash_input(mut q: Query<(Entity, &mut Control)>, mut commands: Commands) {
    for (e, mut c) in q.iter_mut() {
        if c.stick.length() < SMASH_INPUT_THRESHOLD_DISTANCE_FROM_CENTRE {
            continue;
        }
        let is_smash_input = c
            .previous_stick_positions
            .iter()
            // Stick travelled at least half of the active zone
            .any(|stick| (*stick - c.stick).length() >= 0.5);
        if is_smash_input {
            commands
                .entity(e)
                .insert(DirectionalAction {
                    action_type: DirectionalActionType::Smash,
                    direction: c.stick,
                    age: 0,
                });
        } else {
            // TODO: Other kinds of inputs
            return;
        }
        // Remove all but the most recent position
        c.previous_stick_positions.clear();
    }
}

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            FixedUpdate,
            ((age_buffer, age_directional_action), detect_smash_input)
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
        );
    }
}
