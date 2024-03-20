use std::collections::HashSet;

use rand::{rngs::SmallRng, Rng};

use crate::consts::FILTER_WINDOW_SIZE;

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

pub fn build_random(board: &mut HashSet<(i32, i32)>, rng: &mut SmallRng) {
    board.clear();

    for i in 0..FILTER_WINDOW_SIZE {
        for j in 0..FILTER_WINDOW_SIZE {
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
    real_buff: &mut Vec<f32>,
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

    // build impluse response and send it to audio thread
    build_ir(current_board, real_buff);
}

pub fn build_ir(board: &mut HashSet<(i32, i32)>, real_buff: &mut Vec<f32>) {
    for i in 0..FILTER_WINDOW_SIZE {
        real_buff[i] = {
            let mut out = 0.0;
            for j in 0..FILTER_WINDOW_SIZE {
                let b_ij = match board.contains(&(i as i32, j as i32)) {
                    true => 1.0,
                    false => -1.0,
                };
                let b_ji = match board.contains(&(j as i32, i as i32)) {
                    true => 1.0,
                    false => -1.0,
                };

                out += b_ij + b_ji;
            }

            out /= FILTER_WINDOW_SIZE as f32;
            out
        }
    }
}
