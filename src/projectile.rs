use crate::{fighter::FighterEventSet, hitbox::HitboxCollision};
use bevy::prelude::*;

#[derive(Component)]
pub struct Projectile;

fn despawn_collided_projectiles(
    mut commands: Commands,
    q: Query<Entity, With<Projectile>>,
    mut ev_hitbox_collision: EventReader<HitboxCollision>,
) {
    for event in ev_hitbox_collision.read() {
        let entity = event.target;
        if let Err(..) = q.get(entity) {
            continue;
        }
        commands
            .entity(entity)
            .despawn_recursive();
    }
}

pub struct ProjectilePlugin;

impl Plugin for ProjectilePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            FixedUpdate,
            despawn_collided_projectiles.after(FighterEventSet::React),
        );
    }
}
