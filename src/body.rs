use ultraviolet::Vec2;

use crate::simulation;

#[derive(Clone, Copy)]
pub struct Body {
    pub pos: Vec2,
    pub efield: Vec2,
    pub radius: f32,
    pub resist: f32,
}

impl Body {
    pub fn new(pos: Vec2, radius: f32) -> Self {
        Self {
            pos,
            efield: Vec2::zero(),
            radius,
            resist: 1.0,
        }
    }

    pub fn get_new_pos(&mut self, dt: f32) -> Vec2 {
        return self.pos + self.efield * dt * self.resist;
    }
}