use std::{collections::HashSet, sync::Arc};

use nih_plug::prelude::nih_log;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use realfft::{num_complex::Complex, RealFftPlanner, RealToComplex};
use rtrb::Producer;

pub struct GOL {
    current_board: HashSet<(i32, i32)>,
    prod: Producer<Complex<f32>>,
    rng: SmallRng,
    fft: Arc<dyn RealToComplex<f32>>,
    real_buff: Vec<f32>,
    comp_buff: Vec<Complex<f32>>,
    size: usize,
}

impl GOL {
    pub fn new(prod: Producer<Complex<f32>>, size: usize, fft_size: usize, seed: u64) -> Self {
        let mut planner = RealFftPlanner::new();
        let fft = planner.plan_fft_forward(fft_size);
        let real_buff = fft.make_input_vec();
        let comp_buff = fft.make_output_vec();

        let rng = SmallRng::seed_from_u64(seed);

        let mut gol = Self {
            current_board: HashSet::with_capacity(size * size),
            prod,
            size,
            rng,
            fft,
            real_buff,
            comp_buff,
        };

        gol.build_random();

        gol.build_ir();

        match gol
            .fft
            .process_with_scratch(&mut gol.real_buff, &mut gol.comp_buff, &mut [])
        {
            Ok(_) => {}
            Err(_) => nih_log!("error with game fft"),
        }

        match gol.prod.write_chunk(gol.comp_buff.len()) {
            Ok(mut p) => {
                let (s1, s2) = p.as_mut_slices();
                let len1 = s1.len();
                let len2 = s2.len();

                s1.copy_from_slice(&gol.comp_buff[0..len1]);
                s2.copy_from_slice(&gol.comp_buff[len1..len1 + len2]);

                p.commit_all();
            }
            Err(_) => nih_log!("error writing chunk"),
        }

        gol
    }

    pub fn start(&mut self, len: usize) {
        for _ in 0..len {
            self.advance();
        }
    }

    pub fn advance(&mut self) {
        self.step();
        self.build_ir();

        match self
            .fft
            .process_with_scratch(&mut self.real_buff, &mut self.comp_buff, &mut [])
        {
            Ok(_) => {}
            Err(_) => nih_log!("error with game fft"),
        }

        match self.prod.write_chunk(self.comp_buff.len()) {
            Ok(mut p) => {
                let (s1, s2) = p.as_mut_slices();
                let len1 = s1.len();
                let len2 = s2.len();

                s1.copy_from_slice(&self.comp_buff[0..len1]);
                s2.copy_from_slice(&self.comp_buff[0..len2]);

                p.commit_all();
            }
            Err(_) => nih_log!("error writing chunk"),
        }
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

    fn build_ir(&mut self) {
        self.real_buff.fill(0.0);

        for i in 0..self.size {
            self.real_buff[i] = {
                let mut out = 0.0;
                for j in 0..self.size {
                    let b_ij = match self.current_board.contains(&(i as i32, j as i32)) {
                        true => 1.0,
                        false => 0.0,
                    };
                    let b_ji = match self.current_board.contains(&(j as i32, i as i32)) {
                        true => 1.0,
                        false => 0.0,
                    };

                    out += b_ij + b_ji;
                }

                out /= self.size as f32;
                out
            }
        }

        let filter_normalization_factor = self.real_buff.iter().sum::<f32>().recip();

        for sample in &mut self.real_buff {
            *sample *= filter_normalization_factor;
        }
    }
}
