use bevy::{ecs::schedule::SystemSet, prelude::*};

#[derive(Component, Default)]
pub struct Position(pub Vec2);

#[derive(Component, Default)]
pub struct Velocity(pub Vec2);

#[derive(Event)]
pub struct AddVelocity(pub Entity, pub Vec2);

fn add_velocity(mut ev_add_velocity: EventReader<AddVelocity>, mut query: Query<&mut Velocity>) {
    for event in ev_add_velocity.read() {
        if let Ok(mut v) = query.get_mut(event.0) {
            v.0 += event.1;
        }
    }
}

#[derive(Event)]
pub struct SetVelocity(pub Entity, pub Vec2);

fn set_velocity(mut ev_add_velocity: EventReader<SetVelocity>, mut query: Query<&mut Velocity>) {
    for event in ev_add_velocity.read() {
        if let Ok(mut v) = query.get_mut(event.0) {
            v.0 = event.1;
        }
    }
}

#[derive(Event)]
pub struct AccelerateTowards {
    pub entity: Entity,
    pub target: Vec2,
    pub acceleration: f32,
}

fn accelerate_towards(
    mut ev_accelerate: EventReader<AccelerateTowards>,
    mut query: Query<&mut Velocity>,
) {
    for event in ev_accelerate.read() {
        if let Ok(mut v) = query.get_mut(event.entity) {
            let difference = event.target - v.0;
            if difference.length() <= event.acceleration {
                v.0 = event.target;
            } else {
                v.0 += difference.normalize() * event.acceleration;
            }
        }
    }
}

#[derive(Component)]
pub struct Gravity(pub f32);

fn accelerate_from_gravity(mut query: Query<(&mut Velocity, &Gravity)>) {
    for (mut v, g) in &mut query {
        v.0.y += g.0;
    }
}

#[derive(Component)]
pub struct Collider {
    pub normal: Vec2,
    pub breadth: f32,
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
    pub normal: Vec2,
}

#[derive(Component)]
pub struct Airborne;

fn apply_velocity(
    mut objects: Query<(Entity, &mut Position, &mut Velocity)>,
    colliders: Query<(&Collider, &Position), Without<Velocity>>,
    mut ev_collision: EventWriter<Collision>,
    mut commands: Commands,
) {
    for (entity, mut p, mut v) in &mut objects {
        let pushback = displace_and_return_pushback(&mut p, &v.0, colliders.iter());
        if (pushback.length()) == 0.0 {
            if let Some(mut e) = commands.get_entity(entity) {
                e.insert(Airborne);
            }
            continue;
        }
        let normal = pushback.normalize();
        let modified_pushback = normal * normal.dot(pushback);
        v.0 += modified_pushback;
        ev_collision.send(Collision { entity, normal });
        if let Some(mut e) = commands.get_entity(entity) {
            e.remove::<Airborne>();
        }
    }
}

fn displace_and_return_pushback<'a>(
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

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct PhysicsSet;

pub struct PhysicsPlugin;
impl Plugin for PhysicsPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.add_systems(
            FixedUpdate,
            (
                accelerate_towards,
                add_velocity,
                accelerate_from_gravity,
                set_velocity,
                apply_velocity,
            )
                .chain()
                .in_set(PhysicsSet),
        )
        .add_event::<AccelerateTowards>()
        .add_event::<AddVelocity>()
        .add_event::<SetVelocity>()
        .add_event::<Collision>();
    }
}
