use std::ops::Add;
use std::{collections::HashSet, f32::consts::PI};

use crate::fighter::FighterEventSet;
use crate::utils::VisibleDuringDebug;
use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle},
};

struct NearestPass {
    midpoint: Vec2,
    distance: f32,
}

impl NearestPass {
    fn is_collision(&self) -> bool {
        self.distance <= 0.0
    }
}

impl PartialEq for NearestPass {
    fn eq(&self, other: &Self) -> bool {
        self.distance.eq(&other.distance)
    }
}

impl PartialOrd for NearestPass {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.distance
            .partial_cmp(&other.distance)
    }
}

impl Eq for NearestPass {}

impl Ord for NearestPass {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.distance
            .partial_cmp(&other.distance)
            .unwrap()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Shape {
    Circle(f32),
    Pill {
        major_radius: f32,
        minor_radius: f32,
    },
}

impl Default for Shape {
    fn default() -> Self {
        Shape::Circle(1.0)
    }
}

/*
"Radius" and two endpoints
*/
fn get_pill_geometric_data(
    major_axis: f32,
    minor_axis: f32,
    transform: &Transform,
) -> (f32, Vec2, Vec2) {
    let a = Vec3::new(0.0, major_axis, 0.0) * transform.scale;
    let rotated_a = transform.rotation.mul_vec3(a);
    let rotated_b = -rotated_a;
    return (
        minor_axis,
        (rotated_a + transform.translation).xy(),
        (rotated_b + transform.translation).xy(),
    );
}

impl Shape {
    fn nearest_pass(s1: &Self, t1: &Transform, s2: &Self, t2: &Transform) -> NearestPass {
        match (s1, s2) {
            (Shape::Circle(r1), Shape::Circle(r2)) => {
                let p1 = t1.translation.xy();
                let p2 = t2.translation.xy();
                return NearestPass {
                    midpoint: 0.5 * (p1 + p2),
                    distance: (p1 - p2).length() - r1 - r2,
                };
            }
            (
                Shape::Circle(r1),
                Shape::Pill {
                    major_radius,
                    minor_radius,
                },
            ) => {
                let c = t1.translation.xy();
                let (r2, a, b) = get_pill_geometric_data(*major_radius, *minor_radius, t2);
                // Distance to endpoints
                let d_a = (c - a).length();
                let d_b = (c - b).length();
                // Perpendicular distance (see GDD)
                let t = (c - a).dot(b - a) / (b - a).length_squared();
                let point_on_line_closest_to_c = c - (a + (b - a) * t);
                let d_p = if 0.0 <= t && t <= 1.0 {
                    point_on_line_closest_to_c.length()
                } else {
                    f32::INFINITY
                };
                let distance = [d_a, d_b, d_p]
                    .into_iter()
                    .reduce(f32::min)
                    .expect("Circle-Pill distances should exist");
                let midpoint = if distance == d_a {
                    0.5 * (a + c)
                } else if distance == d_b {
                    0.5 * (b + c)
                } else {
                    0.5 * (c + point_on_line_closest_to_c)
                };
                NearestPass {
                    midpoint,
                    distance: distance - r1 - r2,
                }
            }
            // No need to re-implement the wheel
            (Shape::Pill { .. }, Shape::Circle { .. }) => Self::nearest_pass(s2, t2, s1, t1),
            (
                Shape::Pill {
                    major_radius: major_1,
                    minor_radius: minor_1,
                },
                Shape::Pill {
                    major_radius: major_2,
                    minor_radius: minor_2,
                },
            ) => {
                let (r1, a1, b1) = get_pill_geometric_data(*major_1, *minor_1, t1);
                let (r2, a2, b2) = get_pill_geometric_data(*major_2, *minor_2, t2);
                // Easy case: pill's "core" lines intersect
                if let Some(intersection) = intersection_of_line_segments(&a1, &b1, &a2, &b2) {
                    return NearestPass {
                        midpoint: intersection,
                        distance: -r1 - r2,
                    };
                }
                // Harder case: no direct intersection, each endpoint gets treated as a circle
                [
                    (
                        Shape::Circle(r1),
                        Transform::from_xyz(a1.x, a1.y, 0.0),
                        s2,
                        t2,
                    ),
                    (
                        Shape::Circle(r1),
                        Transform::from_xyz(b1.x, b1.y, 0.0),
                        s2,
                        t2,
                    ),
                    (
                        Shape::Circle(r2),
                        Transform::from_xyz(a2.x, a2.y, 0.0),
                        s1,
                        t1,
                    ),
                    (
                        Shape::Circle(r2),
                        Transform::from_xyz(b2.x, b2.y, 0.0),
                        s1,
                        t1,
                    ),
                ]
                .into_iter()
                .map(|(shape_1, transform_1, shape_2, transform_2)| {
                    Self::nearest_pass(&shape_1, &transform_1, shape_2, transform_2)
                })
                .reduce(std::cmp::min)
                .expect("Pill-pill distance")
            }
        }
    }
}

fn cross_product(v: &Vec2, w: &Vec2) -> f32 {
    v.x * w.y - v.y * w.x
}

// https://stackoverflow.com/a/565282/5046693
fn intersection_of_line_segments(p1: &Vec2, p2: &Vec2, q1: &Vec2, q2: &Vec2) -> Option<Vec2> {
    let p = *p1;
    let r = *p2 - p;
    let q = *q1;
    let s = *q2 - q;

    let r_cross_s = cross_product(&r, &s);
    let s_cross_r = cross_product(&s, &r);

    let t = cross_product(&(q - p), &(s / r_cross_s));
    let u = cross_product(&(p - q), &(r / s_cross_r));

    let q_minus_p_cross_r = cross_product(&(q - p), &r);

    if r_cross_s == 0.0 && q_minus_p_cross_r == 0.0 {
        // Collinear
        let t0 = (q - p).dot(r / r.dot(r));
        let t1 = (q + s - p).dot(r / r.dot(r));
        if 0.0 <= t0 && t0 <= 1.0 {
            Some(p + t0 * r)
        } else if 0.0 <= t1 && t1 <= 1.0 {
            Some(p + t1 * r)
        } else if (t0 < 0.0 && t1 > 1.0) || (t1 < 0.0 && t0 > 1.0) {
            // First line segment is fully contained in second one
            Some(p)
        } else {
            None
        }
    } else if r_cross_s == 0.0 && q_minus_p_cross_r != 0.0 {
        // Parallel and non-intersecting
        None
    } else if r_cross_s != 0.0 && 0.0 <= t && t <= 1.0 && 0.0 <= u && u <= 1.0 {
        // Divergent and intersecting
        Some(p + t * r)
    } else {
        // Divergent and non-intersecting
        None
    }
}

#[derive(Default)]
pub enum HitboxPurpose {
    #[default]
    Body,
}

#[derive(Component, Default)]
pub struct Hitbox {
    pub shape: Shape,
    pub purpose: HitboxPurpose,
}

#[derive(Bundle, Default)]
pub struct HitboxBundle {
    pub hitbox: Hitbox,
    pub transform: TransformBundle,
}

#[derive(Component, Default)]
pub struct HitboxGroup;

#[derive(Bundle, Default)]
pub struct HitboxGroupBundle {
    pub hitbox_group: HitboxGroup,
    pub transform: TransformBundle,
}

fn clear_empty_hitbox_groups(
    mut commands: Commands,
    query: Query<(Entity, &Children), With<HitboxGroup>>,
) {
    for (entity, children) in query.iter() {
        if children.iter().len() == 0 {
            commands.entity(entity).despawn();
        }
    }
}

fn add_mesh_to_hitboxes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    query: Query<(Entity, &Hitbox, &Transform), Without<Mesh2dHandle>>,
) {
    for (e, hitbox, transform) in query.iter() {
        let mesh_handle = match hitbox.shape {
            Shape::Circle(r) => Mesh2dHandle(meshes.add(Circle { radius: r })),
            Shape::Pill {
                major_radius,
                minor_radius,
            } => {
                let (r, a, b) = get_pill_geometric_data(major_radius, minor_radius, transform);
                let r_major = (a - b).length() * 0.5;
                let mesh = Capsule2d::new(r, r_major);
                Mesh2dHandle(meshes.add(mesh))
            }
        };
        let colour = match hitbox.purpose {
            HitboxPurpose::Body => Color::rgba(0.5, 0.5, 0.2, 0.5),
        };

        commands.entity(e).insert((
            MaterialMesh2dBundle {
                mesh: mesh_handle,
                material: materials.add(colour),
                transform: *transform,
                global_transform: GlobalTransform::default(),
                ..default()
            },
            VisibleDuringDebug,
        ));
    }
}

pub struct HitboxPlugin;

impl Plugin for HitboxPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, add_mesh_to_hitboxes)
            .add_systems(
                FixedUpdate,
                clear_empty_hitbox_groups.after(FighterEventSet::React),
            );
    }
}
