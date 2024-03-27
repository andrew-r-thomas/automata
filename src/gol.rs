use std::collections::HashSet;

use rand::{rngs::SmallRng, Rng, SeedableRng};

pub struct GOL {
    current_board: HashSet<(i32, i32)>,
    rng: SmallRng,
    size: usize,
}

enum IRMode {
    Gemini,
    Top,
}

impl GOL {
    pub fn new(size: usize, seed: u64) -> Self {
        let rng = SmallRng::seed_from_u64(seed);

        let mut gol = Self {
            current_board: HashSet::with_capacity(size * size),
            size,
            rng,
        };

        gol.build_random();

        gol.build_ir(IRMode::Top);

        gol
    }

    pub fn advance(&mut self) -> Vec<f32> {
        self.step();
        self.build_ir(IRMode::Top)
    }

    fn build_random(&mut self) {
        self.current_board.clear();

        for i in 0..self.size {
            for j in 0..self.size {
                if self.rng.gen() {
                    self.current_board.insert((i as i32, j as i32));
                }
            }
        }
    }

    fn step(&mut self) {
        let mut born = vec![];
        let mut dying = vec![];

        for cell in self.current_board.iter() {
            let neighbors = self.find_neighbors(&cell);

            let mut living_neighbors: u8 = 0;
            for neighbor in neighbors.iter() {
                if self.current_board.contains(neighbor) {
                    living_neighbors += 1;
                } else if !born.contains(neighbor) {
                    let neighbors_neighbors = self.find_neighbors(neighbor);
                    let mut neighbors_living_neighbors: u8 = 0;
                    for neighbor_neighbor in neighbors_neighbors.iter() {
                        if self.current_board.contains(neighbor_neighbor) {
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
            self.current_board.remove(cell);
        }
        for cell in born.iter() {
            self.current_board.insert(*cell);
        }
        dying.clear();
        born.clear();
    }

    fn find_neighbors(&self, pos: &(i32, i32)) -> [(i32, i32); 8] {
        let mut neighbors = [(0, 0); 8];
        let mut i = 0;
        for x in -1..2 {
            for y in -1..2 {
                if x != 0 || y != 0 {
                    let true_x = match () {
                        _ if pos.0 + x > self.size as i32 => x,
                        _ if pos.0 + x < 0 => self.size as i32 - x,
                        _ => pos.0 + x,
                    };
                    let true_y = match () {
                        _ if pos.1 + y > self.size as i32 => y,
                        _ if pos.1 + y < 0 => self.size as i32 - y,
                        _ => pos.1 + y,
                    };

                    neighbors[i] = (true_x, true_y);
                    i += 1;
                }
            }
        }

        neighbors
    }

    fn build_ir(&mut self, mode: IRMode) -> Vec<f32> {
        let mut buff = vec![0.0; self.size];

        for i in 0..self.size {
            buff[i] = {
                let mut out = 0.0;
                for j in 0..self.size {
                    let mut b_ij = 0.0;
                    let mut b_ji = 0.0;

                    match mode {
                        IRMode::Gemini => {
                            b_ij = match (
                                self.current_board.contains(&(i as i32, j as i32)),
                                i % 2 == 0,
                            ) {
                                (true, true) => 1.0,
                                (true, false) => -1.0,
                                _ => 0.0,
                            };
                            b_ji = match (
                                self.current_board.contains(&(j as i32, i as i32)),
                                i % 2 == 0,
                            ) {
                                (true, true) => 1.0,
                                (true, false) => -1.0,
                                _ => 0.0,
                            };
                        }
                        IRMode::Top => {
                            b_ij = match self.current_board.contains(&(i as i32, j as i32)) {
                                true => 1.0,
                                false => 0.0,
                            };
                            b_ji = match self.current_board.contains(&(j as i32, i as i32)) {
                                true => 1.0,
                                false => 0.0,
                            };
                        }
                    }

                    out += b_ij + b_ji;
                }

                out /= self.size as f32;
                out
            }
        }

        let filter_normalization_factor = buff.iter().sum::<f32>().recip();

        for sample in &mut buff {
            *sample *= filter_normalization_factor;
        }

        buff
    }
}
