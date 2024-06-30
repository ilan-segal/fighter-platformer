use bevy::{
    input::gamepad::{
        GamepadAxisChangedEvent, GamepadButtonChangedEvent, GamepadConnection,
        GamepadConnectionEvent, GamepadEvent,
    },
    prelude::*,
};
use enumset::{EnumSet, EnumSetType};

use crate::fighter::Player;

#[derive(EnumSetType)]
pub enum Button {
    Attack,
    Special,
    Shield,
    Grab,
}

#[derive(Component, Default, Clone, Copy)]
pub struct ControlStick(pub Vec2);

const MAX_NUM_PLAYERS: usize = 8;
#[derive(Resource)]
struct Controllers([Option<ControlStick>; MAX_NUM_PLAYERS]);

fn read_input_events(
    mut ev_gamepad: EventReader<GamepadEvent>,
    mut controls: ResMut<Controllers>,
    // mut control_sticks: Query<(&Player, &mut ControlStick)>,
) {
    for ev in ev_gamepad.read() {
        match ev {
            GamepadEvent::Connection(GamepadConnectionEvent {
                gamepad,
                connection: GamepadConnection::Disconnected,
            }) => {
                controls.0[gamepad.id] = None;
            }
            GamepadEvent::Connection(GamepadConnectionEvent {
                gamepad,
                connection: GamepadConnection::Connected(..),
            }) => {
                controls.0[gamepad.id] = Some(ControlStick::default());
            }
            GamepadEvent::Axis(GamepadAxisChangedEvent {
                axis_type,
                value,
                gamepad,
            }) => {
                let id = gamepad.id;
                if let Some(mut control) = &controls.0[id] {
                    match axis_type {
                        GamepadAxisType::LeftStickX => {
                            control.0.x = *value;
                        }
                        GamepadAxisType::LeftStickY => {
                            control.0.y = *value;
                        }
                        _ => {}
                    }
                }
            }
            GamepadEvent::Button(event) => {
                log::info!("Button event: {:?}", event);
            }
        }
    }
}

fn map_input_to_players(controls: Res<Controllers>, mut q: Query<(&Player, &mut ControlStick)>) {
    for (player, mut control_stick) in &mut q {
        let player_id = player.0;
        if let Some(control) = &controls.0[player_id] {
            control_stick.0 = control.0;
        }
    }
}

#[derive(Component)]
pub struct HeldButtons(EnumSet<Button>);

pub enum FighterAction {
    Jab,
    UpTilt,
    ForwardTilt,
    DownTilt,
    NeutralAir,
    UpAir,
    ForwardAir,
    BackAir,
    DownAir,
    UpSmash,
    ForwardSmash,
    DownSmash,
    Grab,
    NeutralSpecial,
    UpSpecial,
    ForwardSpecial,
    DownSpecial,
    Shield,
    Spotdodge,
    Roll,
    Dash,
    Jump,
    FastFall,
    HitFall,
    Airdodge,
}

#[derive(Event)]
pub struct FighterInput(Entity, FighterAction);

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct InputSet;

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            FixedUpdate,
            (read_input_events, map_input_to_players)
                .chain()
                .in_set(InputSet),
        );
    }
}
