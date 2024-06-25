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
enum Shape {
    Circle { r: f32, p: Vec2 },
    Pill { r: f32, a: Vec2, b: Vec2 },
}

impl Shape {
    fn nearest_pass(s1: &Self, s2: &Self) -> NearestPass {
        match (s1, s2) {
            (Shape::Circle { r: r1, p: p1 }, Shape::Circle { r: r2, p: p2 }) => NearestPass {
                midpoint: 0.5 * (*p1 + *p2),
                distance: (*p1 - *p2).length() - r1 - r2,
            },
            (Shape::Circle { r: r1, p: c }, Shape::Pill { r: r2, a, b }) => {
                // Distance to endpoints
                let d_a = (*c - *a).length();
                let d_b = (*c - *b).length();
                // Perpendicular distance (see GDD)
                let t = (*c - *a).dot(*b - *a) / (*b - *a).length_squared();
                let point_on_line_closest_to_c = *c - (*a + (*b - *a) * t);
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
                    0.5 * (*a + *c)
                } else if distance == d_b {
                    0.5 * (*b + *c)
                } else {
                    0.5 * (*c + point_on_line_closest_to_c)
                };
                NearestPass {
                    midpoint,
                    distance: distance - r1 - r2,
                }
            }
            // No need to re-implement the wheel
            (Shape::Pill { .. }, Shape::Circle { .. }) => Self::nearest_pass(s2, s1),
            (
                Shape::Pill {
                    r: r1,
                    a: a1,
                    b: b1,
                },
                Shape::Pill {
                    r: r2,
                    a: a2,
                    b: b2,
                },
            ) => {
                if let Some(intersection) = intersection_of_line_segments(a1, b1, a2, b2) {
                    return NearestPass {
                        midpoint: intersection,
                        distance: -r1 - r2,
                    };
                }
                [
                    (Shape::Circle { r: *r1, p: *a1 }, s2),
                    (Shape::Circle { r: *r1, p: *b1 }, s2),
                    (Shape::Circle { r: *r2, p: *a2 }, s1),
                    (Shape::Circle { r: *r2, p: *b2 }, s1),
                ]
                .into_iter()
                .map(|(a, b)| Self::nearest_pass(&a, b))
                .reduce(std::cmp::min)
                .expect("Pill-pill distances should exist")
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

impl Add<Vec2> for Shape {
    type Output = Self;

    fn add(self, rhs: Vec2) -> Self::Output {
        match self {
            Shape::Circle { r, p } => Shape::Circle { r, p: p + rhs },
            Shape::Pill { r, a, b } => Shape::Pill {
                r,
                a: a + rhs,
                b: b + rhs,
            },
        }
    }
}

enum HitboxPurpose {
    Body,
    Damage {
        percent: u16,
        base_knockback: f32,
        scaling_knockback: f32,
    },
}

struct Hitbox {
    shape: Shape,
    purpose: HitboxPurpose,
    priority: Option<u16>,
}

struct HitboxGroup {
    id: u16,
    hitboxes: Vec<Hitbox>,
    ignored_group_ids: HashSet<u16>,
}

impl HitboxGroup {}
