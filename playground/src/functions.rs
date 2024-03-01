use std::collections::HashSet;
use std::{thread::sleep, time::Duration};

use nannou::color::{Srgb, BLACK, WHITE};
use nannou::event::Update;
use nannou::rand::rand;
use nannou::{App, Frame};

use crate::patterns::*;

pub struct Model {
    pub alive_hash: HashSet<(i32, i32)>,
    pub speed: u32,
    pub zoom_scale: f32,
    pub movement_offset: [i32; 2],
    pub colors: [Srgb<u8>; 2],
}

pub fn model(app: &App) -> Model {
    let mut alive_hash: HashSet<(i32, i32)> = HashSet::new();
    let movement_offset = [0; 2];
    let colors = [BLACK, WHITE];
    let zoom_scale = 10.0;
    let speed = 300;

    let random = generate_random(128);
    load_pattern(&mut alive_hash, random.as_slice());

    app.new_window()
        .size(1005, 1005)
        .fullscreen()
        .view(view)
        .build()
        .unwrap();

    Model {
        alive_hash,
        movement_offset,
        speed,
        zoom_scale,
        colors,
    }
}

pub fn load_pattern(alive_hash: &mut HashSet<(i32, i32)>, pattern: &[(i32, i32)]) {
    alive_hash.clear();
    for cell in pattern {
        alive_hash.insert(*cell);
    }
}

pub fn generate_random(size: usize) -> Vec<(i32, i32)> {
    let mut out = vec![];
    for i in 0..size {
        for j in 0..size {
            if rand::random() {
                out.push((i as i32 - 100, j as i32 - 100));
            }
        }
    }
    out
}

pub fn update(_app: &App, _model: &mut Model, _update: Update) {
    //Runs during the update phase of the program.
    let mut dying: Vec<(i32, i32)> = Vec::new();
    let mut born: Vec<(i32, i32)> = Vec::new();

    for cell in _model.alive_hash.iter() {
        let neighbors = find_neighbors(&cell);
        let mut living_neighbors: u8 = 0;
        for neighbor in neighbors.iter() {
            if _model.alive_hash.contains(neighbor) {
                living_neighbors += 1;
            } else if !born.contains(neighbor) {
                let neighbors_neighbors = find_neighbors(neighbor);
                let mut neighbors_living_neighbors: u8 = 0;
                for neighbor_neighbor in neighbors_neighbors.iter() {
                    if _model.alive_hash.contains(neighbor_neighbor) {
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
        _model.alive_hash.remove(cell);
    }
    for cell in born.iter() {
        _model.alive_hash.insert(*cell);
    }
    dying.clear();
    born.clear();

    sleep(Duration::from_millis(_model.speed.into()));
}

pub fn find_neighbors(pos: &(i32, i32)) -> Vec<(i32, i32)> {
    let mut neighbors: Vec<(i32, i32)> = Vec::new();
    for x in -1..2 {
        for y in -1..2 {
            if x != 0 || y != 0 {
                neighbors.push((pos.0 + x, pos.1 + y));
            }
        }
    }
    neighbors
}

pub fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();

    draw.background().color(model.colors[0]);

    //loop through every alive cell and draw a rectangle at that coordinate
    for i in &model.alive_hash {
        draw.rect()
            .x_y(
                (i.0 + model.movement_offset[0]) as f32 * model.zoom_scale,
                (i.1 + model.movement_offset[1]) as f32 * model.zoom_scale,
            )
            .w_h(model.zoom_scale, model.zoom_scale)
            .color(model.colors[1]);
    }

    //draw frame
    draw.to_frame(app, &frame).unwrap();
}
