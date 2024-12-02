use std::fmt::Debug;

use ultraviolet::Vec2;

#[derive(Clone, Copy)]
pub struct Plate {
    pub min: Vec2,
    pub max: Vec2,
    pub efield: Vec2,
    pub resist: f32,
    pub plate_type: PlateType,
}

impl Plate {
    pub fn new(min: Vec2, max: Vec2) -> Self {
        Self {
            min,
            max,
            efield: Vec2::zero(),
            resist: 1.0,
            plate_type: PlateType::Normal,
        }
    }

    pub fn is_in_plate(&self, pos: Vec2) -> bool {
        return pos.x > self.min.x && pos.x < self.max.x && pos.y > self.min.y && pos.y < self.max.y;
    }

    pub fn efield_at(&self, pos: Vec2) -> Vec2 {
        let a = self.max.y - pos.y;
        let b = self.min.y - pos.y;
        let c = self.min.x - pos.x;
        let d = self.max.x - pos.x;

        let xac = 0.5 * a * (a * a + c * c).ln() + c * (a / c).atan();
        let xad = 0.5 * a * (a * a + d * d).ln() + d * (a / d).atan();
        let xbc = 0.5 * b * (b * b + c * c).ln() + c * (b / c).atan();
        let xbd = 0.5 * b * (b * b + d * d).ln() + d * (b / d).atan();

        let yca = 0.5 * c * (c * c + a * a).ln() + a * (c / a).atan();
        let ycb = 0.5 * c * (c * c + b * b).ln() + b * (c / b).atan();
        let yda = 0.5 * d * (d * d + a * a).ln() + a * (d / a).atan();
        let ydb = 0.5 * d * (d * d + b * b).ln() + b * (d / b).atan();

        let xa = xad - xac;
        let xb = xbd - xbc;
        let yc = ycb - yca;
        let yd = ydb - yda;

        let e_field = Vec2::new(xb - xa, yd - yc) / 2.0;

        if e_field.x.is_nan() || e_field.y.is_nan() {
            return Vec2::new(0.0, 0.0);
        } else {
            return -e_field;
        }
    }

    pub fn contains_point(&self, pos: Vec2) -> bool {
        return pos.x >= self.min.x && pos.x <= self.max.x && pos.y >= self.min.y && pos.y <= self.max.y;
    }

    pub fn make_normal(&mut self) {
        self.plate_type = PlateType::Normal;
        self.resist = 1.0;
        self.efield = Vec2::new(0.0, 0.0);
    }

    pub fn make_battery(&mut self, efield: f32) {
        self.plate_type = PlateType::Battery;
        self.resist = 1.0;
        
        // Find direction by taking the longest side
        let x = self.max.x - self.min.x;
        let y = self.max.y - self.min.y;

        if x > y {
            self.efield = Vec2::new(efield, 0.0);
        } else {
            self.efield = Vec2::new(0.0, efield);
        }
    }

    pub fn make_resistor(&mut self, resist: f32) {
        self.plate_type = PlateType::Resistor;
        self.resist = resist;
    }
}

impl PartialEq for Plate {
    fn eq(&self, other: &Self) -> bool {
        self.min == other.min && self.max == other.max
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum PlateType {
    Normal,
    Battery,
    Resistor
}

impl Debug for PlateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlateType::Normal => write!(f, "Normal"),
            PlateType::Battery => write!(f, "Battery"),
            PlateType::Resistor => write!(f, "Resistor"),
        }
    }
}
