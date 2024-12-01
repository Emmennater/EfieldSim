use crate::{
    body::Body, plate::Plate, quadtree::{Quad, Quadtree}, renderer, utils
};

use ultraviolet::Vec2;

pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub plates: Vec<Plate>,
    pub quadtree: Quadtree,
    pub qe: f32,
    pub qp: f32,
}

impl Simulation {
    pub fn new() -> Self {
        let theta = 0.75;
        let epsilon = 1.0;

        let quadtree = Quadtree::new(theta, epsilon);
        // let (bodies, plates) = utils::large_plate(60000, Vec2::new(-400.0, -400.0), Vec2::new(400.0, 400.0));
        let bodies = Vec::new();
        let plates = Vec::new();

        Self {
            dt: 1.0,
            frame: 0,
            bodies,
            plates,
            quadtree,
            qe: -1.0,
            qp: 1.0,
        }
    }

    pub fn step(&mut self) {
        self.refresh_objects();
        self.iterate();
        self.attract();
        self.frame += 1;
    }

    pub fn refresh_objects(&mut self) {
        let mut lock = renderer::RENDERER_TO_SIM_UPDATE_LOCK.lock();
        if *lock {
            std::mem::swap(&mut self.bodies, &mut renderer::BODIES.lock());
            std::mem::swap(&mut self.plates, &mut renderer::PLATES.lock());
            *lock = false;
        }
    }

    pub fn attract(&mut self) {
        let quad = Quad::new_containing(&self.bodies);
        self.quadtree.clear(quad);

        for body in &mut self.bodies {
            self.quadtree.insert(body.pos, 1.0);
        }

        self.quadtree.propagate();

        for body in &mut self.bodies {
            body.efield = self.quadtree.efield(body.pos) * self.qe;
        }

        for body in &mut self.bodies {
            for plate in &mut self.plates {
                body.efield += plate.efield_at(body.pos) * self.qp;

                if plate.contains_point(body.pos) {
                    let w = plate.max.x - plate.min.x;
                    let h = plate.max.y - plate.min.y;
                    
                    // Battery
                    let strength_x = 1.0 - (body.pos.x - (plate.min.x + plate.max.x) / 2.0).abs() / (w / 2.0);
                    let strength_y = 1.0 - (body.pos.y - (plate.min.y + plate.max.y) / 2.0).abs() / (h / 2.0);

                    body.efield.x += plate.efield.x * strength_x;
                    body.efield.y += plate.efield.y * strength_y;

                    // Resistor
                    body.resist = plate.resist;
                }
            }
        }
    }

    pub fn iterate(&mut self) {
        let bodies_len = self.bodies.len();
        for i in 0..bodies_len {
            let new_pos = self.bodies[i].get_new_pos(self.dt);
    
            if self.on_plate(new_pos) {
                self.bodies[i].pos = new_pos;
            } else if self.on_plate(Vec2::new(new_pos.x, self.bodies[i].pos.y)) {
                self.bodies[i].pos.x = new_pos.x;
            } else if self.on_plate(Vec2::new(self.bodies[i].pos.x, new_pos.y)) {
                self.bodies[i].pos.y = new_pos.y;
            } else if !self.on_plate(Vec2::new(self.bodies[i].pos.x, self.bodies[i].pos.y)) {
                self.bodies[i].pos = new_pos;
            }
        }
    }

    fn on_plate(&self, pos: Vec2) -> bool {
        for plate in &self.plates {
            if plate.is_in_plate(pos) {
                return true;
            }
        }
        return false;
    }
}