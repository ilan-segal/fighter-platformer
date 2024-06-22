use bevy::{
    ecs::system::Command,
    prelude::{Component, Entity, Event, EventReader, EventWriter, Query, Vec2, Without},
    transform::commands,
};
pub const MAX_FLOOR_SLOPE: f32 = 0.1;

#[derive(Component, Default)]
pub struct Position(pub Vec2);

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

#[derive(Event)]
pub struct AddVelocity(pub Entity, pub Vec2);

pub fn add_velocity(
    mut ev_add_velocity: EventReader<AddVelocity>,
    mut query: Query<&mut Velocity>,
) {
    for event in ev_add_velocity.read() {
        if let Ok(mut v) = query.get_mut(event.0) {
            v.0 += event.1;
        }
    }
}

#[derive(Event)]
pub struct SetVelocity(pub Entity, pub Vec2);

pub fn set_velocity(
    mut ev_add_velocity: EventReader<SetVelocity>,
    mut query: Query<&mut Velocity>,
) {
    for event in ev_add_velocity.read() {
        if let Ok(mut v) = query.get_mut(event.0) {
            v.0 = event.1;
        }
    }
}

#[derive(Component)]
pub struct Gravity(f32);

pub fn accelerate_from_gravity(mut query: Query<(&mut Velocity, &Gravity)>) {
    for (mut v, g) in &mut query {
        v.0.y -= g.0;
    }
}

#[derive(Component)]
pub struct Collider {
    normal: Vec2,
    breadth: f32,
}

impl Collider {
    fn get_pushback(&self, p: &Vec2, d: &Vec2, c: &Vec2) -> Option<Vec2> {
        let denominator = self.normal.dot(*d);
        // If denominator is 0, velocity is parallel to collider
        // If denominator is greater than 0, we're moving away from the collider
        if denominator >= 0.0 {
            return None;
        }
        let numerator = self.normal.dot(*c - *p);
        let t = numerator / denominator;
        if t < 0.0 || t > 1.0 {
            return None;
        }
        let b_0 = *p + t * *d;
        let distance_from_centre = (b_0 - *c).length();
        if distance_from_centre > self.breadth * 0.5 {
            return None;
        }
        Some((t - 1.0) * d.dot(self.normal) * self.normal)
    }
}

#[derive(Event)]
pub struct Collision {
    pub entity: Entity,
    pub slope: f32,
}

pub fn apply_velocity(
    mut objects: Query<(Entity, &mut Position, &mut Velocity)>,
    colliders: Query<(&Collider, &Position), Without<Velocity>>,
    mut ev_collision: EventWriter<Collision>,
) {
    for (entity, mut p, mut v) in &mut objects {
        let pushback = displace_and_return_pushback(&mut p, &v.0, colliders.iter());
        if (pushback.length()) == 0.0 {
            continue;
        }
        let normal = pushback.normalize();
        let modified_pushback = normal * normal.dot(pushback);
        v.0 += modified_pushback;
        let slope = (pushback.x / pushback.y).abs();
        ev_collision.send(Collision { entity, slope });
    }
}

pub fn displace_and_return_pushback<'a>(
    position: &mut Position,
    displacement: &Vec2,
    colliders: impl Iterator<Item = (&'a Collider, &'a Position)>,
) -> Vec2 {
    let pushback = colliders
        .into_iter()
        .filter_map(|(collider, centre)| {
            collider.get_pushback(&position.0, displacement, &centre.0)
        })
        // .filter(|p| p.length() > 1.0)
        .next()
        .unwrap_or_default();
    position.0 += *displacement + pushback;
    return pushback;
}
