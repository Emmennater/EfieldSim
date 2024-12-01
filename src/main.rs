use std::sync::atomic::Ordering;

mod utils;
mod body;
mod renderer;
mod simulation;
mod quadtree;
mod plate;

use renderer::Renderer;
use simulation::Simulation;

fn main() {
    let config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(900, 900),
    };

    let mut simulation = Simulation::new();

    std::thread::spawn(move || {
        loop {
            if renderer::PAUSED.load(Ordering::Relaxed) {
                std::thread::yield_now();
            } else {
                simulation.step();
            }
            send_sim_data_to_renderer(&mut simulation);

            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    });

    quarkstrom::run::<Renderer>(config);
}

// Send the simulation data to the renderer
fn send_sim_data_to_renderer(simulation: &mut Simulation) {
    let mut lock = renderer::SIM_TO_RENDERER_UPDATE_LOCK.lock();
    {
        // Update the bodies
        let mut lock = renderer::BODIES.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.bodies);
    }
    {
        // Update the plates
        let mut lock = renderer::PLATES.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.plates);
    }
    {
        // Update the quadtree
        let mut lock = renderer::QUADTREE.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.quadtree.nodes);
    }
    {
        // Update the time step
        let lock = renderer::DT.lock();
        simulation.dt = *lock;
    }
    {
        // Update electron charge
        let lock = renderer::QE.lock();
        simulation.qe = *lock;
    }
    {
        // Update plate charge
        let lock = renderer::QP.lock();
        simulation.qp = *lock;
    }

    // Trigger update
    *lock |= true;
}
