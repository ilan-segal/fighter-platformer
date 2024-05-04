# Fighter Platformer
I intend to implement a platforming/fighting game with similar gameplay as Super Smash Bros. Melee and Rivals of Aether.

This is a personal project with the goal of learning:
- [The Bevy game engine](https://bevyengine.org/) for Rust
- Game design for multiplayer fighting games
- State machine implementation
## Setup
The setup process should be very simple. You just need to run `cargo run` in the project root.
## Current Progress
The program does not currently resemble a game of any kind, really. You are able to control a single character using your favourite gamepad. This character can move in the following ways:
- Walk
- Dash/Run
- Jump
- Airdodge
- Wavedash
## Goals
The project is still in its very early stages and has various goals in the short & long term.
### Short-term
- Air control
- Only one airdodge before landing
- A full moveset for the playable character, including:
	- Neutral/tilt/smash attacks
	- Specials
	- Grab, pummel, and throws
	- Double jump
	- Shield
	- Roll
	- Spotdodge
### Long-term
- Re-mappable controls
- Multiplayer
- Multiple playable characters
- Combat mechanics
	- Player damage
	- Knockback
	- Shield damage
	- Shieldbreaking
- Hitboxes for players and moves
	- The shapes for these hitboxes is unclear; they might be shaped as rectangles or pills.
- Very minimal particle system (not too distracting or visually noisy)
### Very long-term
- Multiple stages
- Menus
- Online play