use crate::utils::LeftRight;
use bevy::prelude::{Component, Entity, Event, Vec2};
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
