use std::sync::atomic::{AtomicBool, Ordering};

use crate::{
    utils,
    body::{self, Body},
    plate::{Plate, PlateType},
    quadtree::{Node, Quadtree},
};

use quarkstrom::{egui, winit::event::VirtualKeyCode, winit_input_helper::WinitInputHelper};

use palette::{rgb::Rgba, white_point::E, Hsluv, IntoColor};
use ultraviolet::{Vec2, Vec4};

use once_cell::sync::Lazy;
use parking_lot::Mutex;

pub static PAUSED: Lazy<AtomicBool> = Lazy::new(|| false.into());
pub static SIM_TO_RENDERER_UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
pub static RENDERER_TO_SIM_UPDATE_LOCK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
pub static BODIES: Lazy<Mutex<Vec<Body>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static PLATES: Lazy<Mutex<Vec<Plate>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static QUADTREE: Lazy<Mutex<Vec<Node>>> = Lazy::new(|| Mutex::new(Vec::new()));
pub static DT: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(1.0));
// pub static QE: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(4.5e-1));
// pub static QP: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(1.0e-2));
pub static QE: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(0.56e0));
pub static QP: Lazy<Mutex<f32>> = Lazy::new(|| Mutex::new(4.5e-2));


pub struct Renderer {
    pos: Vec2,
    scale: f32,
    settings_window_open: bool,
    
    show_bodies: bool,
    show_plates: bool,
    show_quadtree: bool,

    depth_range: (usize, usize),

    pub bodies: Vec<Body>,
    pub plates: Vec<Plate>,
    quadtree: Vec<Node>,
    
    // Editing
    remove_selection: bool,
    setting_plate: Option<PlateType>,
    battery_strength: f32,
    resistor_strength: f32,
    selected_plate_indicies: Vec<usize>,
    body_density: usize,

    // Selection
    grid_size: f32,
    hovered_cell: Vec2,
    cell_start: Vec2,
    cell_end: Vec2,
    selection_active: bool,
    mouse_down: bool,
}

impl Renderer {
    fn get_selection(&self) -> (Vec2, Vec2) {
        let min = Vec2::new(
            self.cell_start.x.min(self.cell_end.x),
            self.cell_start.y.min(self.cell_end.y),
        );
        let max = Vec2::new(
            self.cell_start.x.max(self.cell_end.x),
            self.cell_start.y.max(self.cell_end.y),
        ) + Vec2::one() * self.grid_size;
        return (min, max);
    }

    fn get_selected_plate_indicies(&self) -> Vec<usize> {
        let mut selected = Vec::new();
        let margin = 1.0;

        let (mut min, mut max) = self.get_selection();

        min += Vec2::new(margin, margin);
        max -= Vec2::new(margin, margin);

        for i in 0..self.plates.len() {
            let plate = &self.plates[i];

            // Check for overlap
            if plate.min.x < max.x && plate.max.x > min.x &&
                plate.min.y < max.y && plate.max.y > min.y {
                selected.push(i);
            }
        }

        return selected;
    }

    fn deselect_all(&mut self) {
        self.selected_plate_indicies.clear();
        self.remove_selection = false;
        self.selection_active = false;
    }

    fn update_objects(&mut self) -> bool {
        let mut updated = false;
        let plate_type = self.setting_plate.take(); // take the value out of self.setting_plate

        // Removing plates
        if self.remove_selection {
            for i in self.selected_plate_indicies.iter().rev() {
                let plate = &self.plates[*i];

                // Remove bodies in plates
                for j in (0..self.bodies.len()).rev() {
                    let body = &mut self.bodies[j];
                    if plate.contains_point(body.pos) {
                        self.bodies.remove(j);
                    }
                }

                // Remove plate
                self.plates.remove(*i);
            }
            
            self.deselect_all();
            updated = true;
        }

        // Adding plates / Changing plate type
        if let Some(plate_type) = plate_type {
            if self.selected_plate_indicies.len() > 0 {
                // Change the type of the plate
                for i in 0..self.selected_plate_indicies.len() {
                    let idx = self.selected_plate_indicies[i];
                    let plate = &mut self.plates[idx];
                    match plate_type {
                        PlateType::Normal => plate.make_normal(),
                        PlateType::Battery => plate.make_battery(self.battery_strength),
                        PlateType::Resistor => plate.make_resistor(self.resistor_strength),
                    }
                }
            } else {
                // Create a new plate
                let (min, mut max) = self.get_selection();
                let mut plate = Plate::new(min, max);
                match plate_type {
                    PlateType::Normal => plate.make_normal(),
                    PlateType::Battery => plate.make_battery(self.battery_strength),
                    PlateType::Resistor => plate.make_resistor(self.resistor_strength),
                }
                self.plates.push(plate);

                let area = (max.x - min.x) * (max.y - min.y) / (self.grid_size * self.grid_size);
                let bodies_to_add = area * self.body_density as f32;
                let margin = self.grid_size * 0.1;

                for _ in 0..(bodies_to_add as usize) {
                    let pos = Vec2::new(
                        utils::random_in_range(min.x + margin, max.x - margin),
                        utils::random_in_range(min.y + margin, max.y - margin),
                    );
                    let mut body = Body::new(pos, 1.0);
                    self.bodies.push(body);
                }
            }
            
            self.deselect_all();
            updated = true;
        }

        // Changing plate strengths
        if self.selection_active {
            for i in 0..self.selected_plate_indicies.len() {
                let idx = self.selected_plate_indicies[i];
                let plate = &mut self.plates[idx];
                
                match plate.plate_type {
                    PlateType::Battery => {
                        let old_efield = plate.efield.clone();
                        plate.make_battery(self.battery_strength);
                        updated |= old_efield != plate.efield;
                    },
                    PlateType::Resistor => {
                        let old_resist = plate.resist;
                        plate.make_resistor(self.resistor_strength);
                        updated |= old_resist != plate.resist;
                    },
                    _ => {}
                }
            }
        }

        return updated;
    }
}

impl quarkstrom::Renderer for Renderer {
    fn new() -> Self {
        Self {
            pos: Vec2::zero(),
            scale: 100.0,
            settings_window_open: false,
            show_bodies: true,
            show_plates: true,
            show_quadtree: false,
            depth_range: (0, 0),
            bodies: Vec::new(),
            plates: Vec::new(),
            quadtree: Vec::new(),
            remove_selection: false,
            setting_plate: None,
            battery_strength: 1.0,
            resistor_strength: 0.5,
            selected_plate_indicies: Vec::new(),
            body_density: 4,
            grid_size: 10.0,
            hovered_cell: Vec2::zero(),
            cell_start: Vec2::zero(),
            cell_end: Vec2::zero(),
            selection_active: false,
            mouse_down: false,
        }
    }

    fn input(&mut self, input: &WinitInputHelper, width: u16, height: u16) {
        self.settings_window_open ^= input.key_pressed(VirtualKeyCode::E);

        if input.key_pressed(VirtualKeyCode::Space) {
            let val = PAUSED.load(Ordering::Relaxed);
            PAUSED.store(!val, Ordering::Relaxed)
        }

        if let Some((mx, my)) = input.mouse() {
            // Scroll steps to double/halve the scale
            let steps = 5.0;

            // Modify input
            let zoom = (-input.scroll_diff() / steps).exp2();

            // Screen space -> view space
            let target =
                Vec2::new(mx * 2.0 - width as f32, height as f32 - my * 2.0) / height as f32;

            // Move view position based on target
            self.pos += target * self.scale * (1.0 - zoom);

            // Zoom
            self.scale *= zoom;
        }

        // Grab
        if input.mouse_held(2) {
            let (mdx, mdy) = input.mouse_diff();
            self.pos.x -= mdx / height as f32 * self.scale * 2.0;
            self.pos.y += mdy / height as f32 * self.scale * 2.0;
        }

        let world_mouse = || -> Vec2 {
            let (mx, my) = input.mouse().unwrap_or_default();
            let mut mouse = Vec2::new(mx, my);
            mouse *= 2.0 / height as f32;
            mouse.y -= 1.0;
            mouse.y *= -1.0;
            mouse.x -= width as f32 / height as f32;
            mouse * self.scale + self.pos
        };

        self.hovered_cell = Vec2::new(
            (world_mouse().x / self.grid_size).floor() * self.grid_size,
            (world_mouse().y / self.grid_size).floor() * self.grid_size,
        );

        // Selection
        if input.mouse_pressed(0) {
            self.mouse_down = true;
            self.cell_start.x = self.hovered_cell.x;
            self.cell_start.y = self.hovered_cell.y;
            self.selection_active = true;
            self.selected_plate_indicies = Vec::new();
        }

        if input.mouse_pressed(1) {
            self.selection_active = false;
            self.selected_plate_indicies = Vec::new();
        }

        if input.mouse_released(0) {
            self.mouse_down = false;

            if self.selection_active {
                self.selected_plate_indicies = self.get_selected_plate_indicies();

                if self.selected_plate_indicies.len() == 1 {
                    let plate = self.plates[self.selected_plate_indicies[0]];

                    match plate.plate_type {
                        PlateType::Battery => {
                            if plate.efield.x == 0.0 {
                                self.battery_strength = plate.efield.y;
                            } else {
                                self.battery_strength = plate.efield.x;
                            }
                        },
                        PlateType::Resistor => {
                            self.resistor_strength = plate.resist;
                        },
                        _ => {}
                    }
                }
            }
        }

        if input.mouse_held(0) {
            self.cell_end.x = self.hovered_cell.x;
            self.cell_end.y = self.hovered_cell.y;
        }

        if input.key_pressed(VirtualKeyCode::Back) {
            if self.selection_active {
                self.remove_selection = true;
                self.selection_active = false;
            }
        }

        if input.key_pressed(VirtualKeyCode::Key1) {
            self.setting_plate = Some(PlateType::Normal);
        }

        if input.key_pressed(VirtualKeyCode::Key2) {
            self.setting_plate = Some(PlateType::Battery);
        }

        if input.key_pressed(VirtualKeyCode::Key3) {
            self.setting_plate = Some(PlateType::Resistor);
        }
    }

    fn render(&mut self, ctx: &mut quarkstrom::RenderContext) {
        {
            let mut lock = SIM_TO_RENDERER_UPDATE_LOCK.lock();
            if *lock {
                let mut body_lock = BODIES.lock();
                let mut plate_lock = PLATES.lock();

                // Get bodies from the simulation
                std::mem::swap(&mut self.bodies, &mut body_lock);

                // Get plates from the simulation
                std::mem::swap(&mut self.plates, &mut plate_lock);

                // Get quadtree from the simulation
                std::mem::swap(&mut self.quadtree, &mut QUADTREE.lock());

                // Update objects
                if self.update_objects() {
                    *body_lock = self.bodies.clone();
                    *plate_lock = self.plates.clone();

                    let mut lock = RENDERER_TO_SIM_UPDATE_LOCK.lock();
                    *lock |= true;
                }
            }

            // Update complete
            *lock = false;
        }

        ctx.clear_circles();
        ctx.clear_lines();
        ctx.clear_rects();
        ctx.set_view_pos(self.pos);
        ctx.set_view_scale(self.scale);

        let mut show_selection = true;

        if !self.bodies.is_empty() {
            if self.show_bodies {
                for i in 0..self.bodies.len() {
                    // Draw body
                    ctx.draw_circle(self.bodies[i].pos, self.bodies[i].radius, [50, 180, 240, 255]);
                
                    // Draw acceleration
                    // ctx.draw_line(
                    //     self.bodies[i].pos,
                    //     self.bodies[i].pos + self.bodies[i].acc.normalized() * 5.0,
                    //     [0xff, 0x00, 0x00, 0xff],
                    // );
                }
            }
        }

        if !self.plates.is_empty() {
            if self.show_plates {
                for i in 0..self.plates.len() {
                    // Draw plate
                    match self.plates[i].plate_type {
                        PlateType::Normal => {
                            ctx.draw_rect(self.plates[i].min, self.plates[i].max, [50, 50, 50, 255]);
                        },
                        PlateType::Battery => {
                            ctx.draw_rect(self.plates[i].min, self.plates[i].max, [30, 100, 30, 255]);
                        },
                        PlateType::Resistor => {
                            ctx.draw_rect(self.plates[i].min, self.plates[i].max, [120, 70, 10, 255]);
                        }
                    }
                }
                
                if self.selection_active {
                    if !self.selected_plate_indicies.is_empty() {
                        show_selection = false;
                    }

                    for i in 0..self.selected_plate_indicies.len() {
                        let idx = self.selected_plate_indicies[i];
                        let plate = &self.plates[idx];
                        let min = plate.min;
                        let max = plate.max;
    
                        // Draw outline
                        ctx.draw_line(min, Vec2::new(min.x, max.y), [255, 255, 255, 255]);
                        ctx.draw_line(min, Vec2::new(max.x, min.y), [255, 255, 255, 255]);
                        ctx.draw_line(max, Vec2::new(min.x, max.y), [255, 255, 255, 255]);
                        ctx.draw_line(max, Vec2::new(max.x, min.y), [255, 255, 255, 255]);
                    }
                }
            }
        }

        if self.show_quadtree && !self.quadtree.is_empty() {
            let mut depth_range = self.depth_range;
            if depth_range.0 >= depth_range.1 {
                let mut stack = Vec::new();
                stack.push((Quadtree::ROOT, 0));

                let mut min_depth = usize::MAX;
                let mut max_depth = 0;
                while let Some((node, depth)) = stack.pop() {
                    let node = &self.quadtree[node];

                    if node.is_leaf() {
                        if depth < min_depth {
                            min_depth = depth;
                        }
                        if depth > max_depth {
                            max_depth = depth;
                        }
                    } else {
                        for i in 0..4 {
                            stack.push((node.children + i, depth + 1));
                        }
                    }
                }

                depth_range = (min_depth, max_depth);
            }
            let (min_depth, max_depth) = depth_range;

            let mut stack = Vec::new();
            stack.push((Quadtree::ROOT, 0));
            while let Some((node, depth)) = stack.pop() {
                let node = &self.quadtree[node];

                if node.is_branch() && depth < max_depth {
                    for i in 0..4 {
                        stack.push((node.children + i, depth + 1));
                    }
                } else if depth >= min_depth {
                    let quad = node.quad;
                    let half = Vec2::new(0.5, 0.5) * quad.size;
                    let min = quad.center - half;
                    let max = quad.center + half;

                    let t = ((depth - min_depth + !node.is_empty() as usize) as f32)
                        / (max_depth - min_depth + 1) as f32;

                    let start_h = -100.0;
                    let end_h = 80.0;
                    let h = start_h + (end_h - start_h) * t;
                    let s = 100.0;
                    let l = t * 100.0;

                    let c = Hsluv::new(h, s, l);
                    let rgba: Rgba = c.into_color();
                    let color: [u8; 4] = rgba.into_format().into();

                    ctx.draw_rect(min, max, [color[0], color[1], color[2], 0x80]);
                }
            }
        }
    
        // Draw hovered cell
        if self.selection_active {
            if show_selection || self.mouse_down {
                let min = Vec2::new(
                    self.cell_start.x.min(self.cell_end.x),
                    self.cell_start.y.min(self.cell_end.y),
                );
                let max = Vec2::new(
                    self.cell_start.x.max(self.cell_end.x),
                    self.cell_start.y.max(self.cell_end.y),
                );
    
                let beg = min;
                let end = max + Vec2::one() * self.grid_size;
    
                ctx.draw_line(beg, Vec2::new(beg.x, end.y), [0xff, 0xff, 0xff, 0xff]);
                ctx.draw_line(beg, Vec2::new(end.x, beg.y), [0xff, 0xff, 0xff, 0xff]);
                ctx.draw_line(end, Vec2::new(beg.x, end.y), [0xff, 0xff, 0xff, 0xff]);
                ctx.draw_line(end, Vec2::new(end.x, beg.y), [0xff, 0xff, 0xff, 0xff]);
            }
        } else {
            let beg = self.hovered_cell;
            let end = self.hovered_cell + Vec2::one() * self.grid_size;
        
            ctx.draw_line(beg, Vec2::new(beg.x, end.y), [0xff, 0xff, 0xff, 0xff]);
            ctx.draw_line(beg, Vec2::new(end.x, beg.y), [0xff, 0xff, 0xff, 0xff]);
            ctx.draw_line(end, Vec2::new(beg.x, end.y), [0xff, 0xff, 0xff, 0xff]);
            ctx.draw_line(end, Vec2::new(end.x, beg.y), [0xff, 0xff, 0xff, 0xff]);
        }
    }

    fn gui(&mut self, ctx: &quarkstrom::egui::Context) {
        egui::Window::new("")
            .open(&mut self.settings_window_open)
            .show(ctx, |ui| {
                // Number of bodies
                ui.label(format!("Bodies: {}", self.bodies.len()));

                ui.checkbox(&mut self.show_bodies, "Show Bodies");
                ui.checkbox(&mut self.show_quadtree, "Show Quadtree");
                ui.checkbox(&mut self.show_plates, "Show Plates");
                
                {
                    let mut dt = DT.lock();
                    ui.add(egui::Slider::new(&mut *dt, 0.1..=1.0).text("Time Step"));
                }
                {
                    let mut qe = QE.lock();
                    ui.add(egui::Slider::new(&mut *qe, 1e-2..=1.0).text("Electron Charge"));
                }
                {
                    let mut qp = QP.lock();
                    ui.add(egui::Slider::new(&mut *qp, 1e-3..=1.0e-1).text("Plate Charge"));
                }

                ui.add(egui::Slider::new(&mut self.body_density, 1..=6).text("Electron Density"));
                ui.add(egui::Slider::new(&mut self.battery_strength, -5.0..=5.0).text("Battery Strength"));
                ui.add(egui::Slider::new(&mut self.resistor_strength, 0.0..=1.0).text("Resistor Strength"));
    
                if self.show_quadtree {
                    let range = &mut self.depth_range;
                    ui.horizontal(|ui| {
                        ui.label("Depth Range:");
                        ui.add(egui::DragValue::new(&mut range.0).speed(0.05));
                        ui.label("to");
                        ui.add(egui::DragValue::new(&mut range.1).speed(0.05));
                    });
                }
            });
    }
    
}
