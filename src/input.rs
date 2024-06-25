use bevy::{
    app::{Plugin, Update},
    input::gamepad::{GamepadAxisChangedEvent, GamepadEvent},
    prelude::{Component, Entity, Event, EventReader, GamepadAxisType, Query, Vec2},
};
use enumset::{EnumSet, EnumSetType};

#[derive(EnumSetType)]
pub enum Button {
    Attack,
    Special,
    Shield,
    Grab,
}

#[derive(Component)]
pub struct ControlStick(pub Vec2);

fn read_input_events(
    mut ev_gamepad: EventReader<GamepadEvent>,
    mut control_sticks: Query<&mut ControlStick>,
) {
    // TODO: Handle multiple controllers
    for ev in ev_gamepad.read() {
        match ev {
            GamepadEvent::Axis(GamepadAxisChangedEvent {
                axis_type, value, ..
            }) => match axis_type {
                GamepadAxisType::RightStickX => {
                    control_sticks.single_mut().0.x = *value;
                }
                GamepadAxisType::RightStickY => {
                    control_sticks.single_mut().0.y = *value;
                }
                _ => {}
            },
            _ => {}
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

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(Update, read_input_events);
    }
}
