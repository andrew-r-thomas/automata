extern crate glutin_window;
extern crate graphics;
extern crate opengl_graphics;
extern crate piston;

use std::collections::HashSet;

use glutin_window::GlutinWindow as Window;
use graphics::color::{BLACK, WHITE};
use opengl_graphics::{GlGraphics, OpenGL};
use piston::event_loop::{EventSettings, Events};
use piston::input::{RenderArgs, RenderEvent, UpdateArgs, UpdateEvent};
use piston::window::WindowSettings;
use rand::rngs::SmallRng;
use rand::Rng;
use rand::SeedableRng;

pub struct App {
    gl: GlGraphics, // OpenGL drawing backend.
    current_board: HashSet<(i32, i32)>,
    born_vec: Vec<(i32, i32)>,
    dying_vec: Vec<(i32, i32)>,
}

impl App {
    fn render(&mut self, args: &RenderArgs) {
        use graphics::*;

        let square = rectangle::square(0.0, 0.0, 10.0);

        self.gl.draw(args.viewport(), |c, gl| {
            clear(BLACK, gl);

            for cell in &self.current_board {
                println!("drawing cell: {:?}\n", cell);

                let (x, y) = (
                    (args.window_size[0] / 4 as f64) + cell.0 as f64 * 12.0,
                    (args.window_size[1] / 4 as f64) + cell.1 as f64 * 12.0,
                );

                let transform = c.transform.trans(x, y).trans(-25.0, -25.0);

                // Draw a box rotating around the middle of the screen.
                rectangle(WHITE, square, transform, gl);
            }
        });
    }

    fn update(&mut self, _args: &UpdateArgs) {
        step(
            &mut self.current_board,
            &mut self.born_vec,
            &mut self.dying_vec,
        );
    }
}

fn main() {
    // Change this to OpenGL::V2_1 if not working.
    let opengl = OpenGL::V3_2;

    // Create a Glutin window.
    let mut window: Window = WindowSettings::new("spinning-square", [1600, 1200])
        .graphics_api(opengl)
        .exit_on_esc(true)
        .build()
        .unwrap();

    let mut current_board = HashSet::with_capacity(64 * 64);
    let mut rng = SmallRng::seed_from_u64(69);

    build_random(&mut current_board, &mut rng);

    // Create a new game and run it.
    let mut app = App {
        gl: GlGraphics::new(opengl),
        current_board,
        born_vec: Vec::with_capacity(64 * 64),
        dying_vec: Vec::with_capacity(64 * 64),
    };

    let mut events = Events::new(EventSettings::new());
    while let Some(e) = events.next(&mut window) {
        if let Some(args) = e.render_args() {
            app.render(&args);
        }

        if let Some(args) = e.update_args() {
            app.update(&args);
        }
    }
}

pub fn build_random(board: &mut HashSet<(i32, i32)>, rng: &mut SmallRng) {
    board.clear();

    for i in 0..64 {
        for j in 0..64 {
            if rng.gen() {
                board.insert((i as i32, j as i32));
            }
        }
    }
}
pub fn step(
    current_board: &mut HashSet<(i32, i32)>,
    born: &mut Vec<(i32, i32)>,
    dying: &mut Vec<(i32, i32)>,
) {
    for cell in current_board.iter() {
        let neighbors = find_neighbors(&cell);

        let mut living_neighbors: u8 = 0;
        for neighbor in neighbors.iter() {
            if current_board.contains(neighbor) {
                living_neighbors += 1;
            } else if !born.contains(neighbor) {
                let neighbors_neighbors = find_neighbors(neighbor);
                let mut neighbors_living_neighbors: u8 = 0;
                for neighbor_neighbor in neighbors_neighbors.iter() {
                    if current_board.contains(neighbor_neighbor) {
                        neighbors_living_neighbors += 1;
                    }
                }
                if neighbors_living_neighbors == 3 && !born.contains(neighbor) {
                    born.push(*neighbor);
                }
            }
        }
        if living_neighbors > 3 || living_neighbors < 2 && !dying.contains(cell) {
            dying.push(*cell);
        }
    }
    for cell in dying.iter() {
        current_board.remove(cell);
    }
    for cell in born.iter() {
        current_board.insert(*cell);
    }
    dying.clear();
    born.clear();
}

pub fn find_neighbors(pos: &(i32, i32)) -> [(i32, i32); 8] {
    let mut neighbors = [(0, 0); 8];
    let mut i = 0;
    for x in -1..2 {
        for y in -1..2 {
            if x != 0 || y != 0 {
                neighbors[i] = (pos.0 + x, pos.1 + y);
                i += 1;
            }
        }
    }

    neighbors
}
