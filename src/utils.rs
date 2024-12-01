use crate::{
    body::Body,
    plate::Plate,
};

use ultraviolet::Vec2;

pub fn uniform_disc(n: usize) -> Vec<Body> {
    fastrand::seed(0);
    let inner_radius = 25.0;
    let outer_radius = (n as f32).sqrt() * 5.0;

    let mut bodies: Vec<Body> = Vec::with_capacity(n);

    // let m = 1e6;
    // let center = Body::new(Vec2::zero(), m as f32, inner_radius);
    // bodies.push(center);

    while bodies.len() < n {
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let t = inner_radius / outer_radius;
        let r = fastrand::f32() * (1.0 - t * t) + t * t;
        let pos = Vec2::new(cos, sin) * outer_radius * r.sqrt();
        let mass = 1.0f32;
        let radius = mass.cbrt();

        bodies.push(Body::new(pos, radius));
    }

    bodies.sort_by(|a, b| a.pos.mag_sq().total_cmp(&b.pos.mag_sq()));
    let mut mass = 0.0;
    for i in 0..n {
        mass += 1.0;
        if bodies[i].pos == Vec2::zero() {
            continue;
        }
    }

    bodies
}

pub fn uniform_rect(n: usize, min: Vec2, max: Vec2, qe: f32) -> Vec<Body> {
    fastrand::seed(0);
    let mut bodies: Vec<Body> = Vec::with_capacity(n);

    for i in 0..n {
        let x = min.x + (max.x - min.x) * fastrand::f32();
        let y = min.y + (max.y - min.y) * fastrand::f32();
        bodies.push(Body::new(Vec2::new(x, y), 1.0));
    }

    bodies
}

pub fn two_body() -> Vec<Body> {
    let n = 2;
    let mut bodies: Vec<Body> = Vec::with_capacity(n);

    bodies.push(Body::new(Vec2::new(5.0, 0.0), 1.0));
    bodies.push(Body::new(Vec2::new(-5.0, 0.0), 1.0));

    bodies
}

pub fn three_body() -> (Vec<Body>, Vec<Plate>) {
    let n = 10;
    let qe = -1.0;
    let qp = 1.0e-2;
    let mut bodies: Vec<Body> = Vec::with_capacity(n);
    let mut plates: Vec<Plate> = Vec::with_capacity(n);

    bodies.push(Body::new(Vec2::new(5.0, 0.0), 1.0));
    bodies.push(Body::new(Vec2::new(-5.0, 0.0), 1.0));
    bodies.push(Body::new(Vec2::new(0.0, 5.0), 1.0));

    plates.push(Plate::new(Vec2::new(-40.0, -10.0), Vec2::new(40.0, 10.0)));

    return (bodies, plates);
}

pub fn large_plate(n: usize, min: Vec2, max: Vec2) -> (Vec<Body>, Vec<Plate>) {
    let qe = -1.0;
    let qp = 2.0e-2;
    let bodies: Vec<Body> = uniform_rect(n, min * 0.9, max * 0.9, qe);
    let mut plates: Vec<Plate> = Vec::with_capacity(1);

    plates.push(Plate::new(min, max));

    return (bodies, plates);
}

pub fn random_in_range(min: f32, max: f32) -> f32 {
    fastrand::f32() * (max - min) + min
}
