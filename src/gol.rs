use std::collections::HashSet;

use rand::{rngs::SmallRng, SeedableRng};

use crate::consts::{FILTER_WINDOW_SIZE, SEED};

struct GOL {
    current_board: HashSet<(i32, i32)>,
    rng: SmallRng,
    running: bool,
}

impl GOL {
    pub fn new() -> Self {
        let current_board = HashSet::<(i32, i32)>::new();
        let rng = SmallRng::seed_from_u64(SEED);

        let gol = GOL {
            current_board,
            rng,
            running: false,
        };

        gol.build_random();

        gol
    }

    fn build_random(&self) {
        todo!()
    }

    pub fn start(&self) {
        let mut dying: Vec<(i32, i32)> = Vec::new();
        let mut born: Vec<(i32, i32)> = Vec::new();

        loop {
            match self.running {
                true => {
                    let mut alive_cells = self.current_board;

                    for cell in alive_cells.iter() {
                        let neighbors = Self::find_neighbors(&cell);
                        let mut living_neighbors: u8 = 0;
                        for neighbor in neighbors.iter() {
                            if alive_cells.contains(neighbor) {
                                living_neighbors += 1;
                            } else if !born.contains(neighbor) {
                                let neighbors_neighbors = Self::find_neighbors(neighbor);
                                let mut neighbors_living_neighbors: u8 = 0;
                                for neighbor_neighbor in neighbors_neighbors.iter() {
                                    if alive_cells.contains(neighbor_neighbor) {
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
                        alive_cells.remove(cell);
                    }
                    for cell in born.iter() {
                        alive_cells.insert(*cell);
                    }
                    dying.clear();
                    born.clear();

                    // let ir = build_ir(&alive_cells);
                    // match ir_producer.write_chunk_uninit(FILTER_WINDOW_SIZE) {
                    //     Ok(chunk) => {
                    //         chunk.fill_from_iter(ir.into_iter());
                    //     }
                    //     Err(_) => {
                    //         todo!();
                    //     }
                    // }

                    dying.clear();
                    born.clear();
                }
                false => {}
            }
        }
    }

    fn find_neighbors(pos: &(i32, i32)) -> Vec<(i32, i32)> {
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
}

// pub fn game_loop(gui_reciever: Receiver<GUIEvent>, mut ir_producer: Producer<Complex<f32>>) {
//     let mut alive_cells =
//         HashSet::<(i32, i32)>::with_capacity(FILTER_WINDOW_SIZE * FILTER_WINDOW_SIZE);
//     let mut game_running = false;
//     let mut rng = SmallRng::seed_from_u64(69);

//     build_random(&mut alive_cells, &mut rng, FILTER_WINDOW_SIZE);

//     loop {
//         let message = gui_reciever.try_recv();
//         match message {
//             Ok(GUIEvent::PlayPause) => {
//                 game_running = !game_running;
//             }
//             Ok(GUIEvent::Reset) => {
//                 game_running = false;
//                 alive_cells.drain();
//                 build_random(&mut alive_cells, &mut rng, FILTER_WINDOW_SIZE);
//             }
//             Err(TryRecvError::Empty) => {}
//             Err(TryRecvError::Disconnected) => panic!("gui disconnected from game loop"),
//         }

//         if game_running {
//             let mut dying: Vec<(i32, i32)> = Vec::new();
//             let mut born: Vec<(i32, i32)> = Vec::new();

//             for cell in alive_cells.iter() {
//                 let neighbors = find_neighbors(&cell);
//                 let mut living_neighbors: u8 = 0;
//                 for neighbor in neighbors.iter() {
//                     if alive_cells.contains(neighbor) {
//                         living_neighbors += 1;
//                     } else if !born.contains(neighbor) {
//                         let neighbors_neighbors = find_neighbors(neighbor);
//                         let mut neighbors_living_neighbors: u8 = 0;
//                         for neighbor_neighbor in neighbors_neighbors.iter() {
//                             if alive_cells.contains(neighbor_neighbor) {
//                                 neighbors_living_neighbors += 1;
//                             }
//                         }
//                         if neighbors_living_neighbors == 3 && !born.contains(neighbor) {
//                             born.push(*neighbor);
//                         }
//                     }
//                 }
//                 if living_neighbors > 3 || living_neighbors < 2 && !dying.contains(cell) {
//                     dying.push(*cell);
//                 }
//             }
//             for cell in dying.iter() {
//                 alive_cells.remove(cell);
//             }
//             for cell in born.iter() {
//                 alive_cells.insert(*cell);
//             }
//             dying.clear();
//             born.clear();

//             let ir = build_ir(&alive_cells);
//             match ir_producer.write_chunk_uninit(FILTER_WINDOW_SIZE) {
//                 Ok(chunk) => {
//                     chunk.fill_from_iter(ir.into_iter());
//                 }
//                 Err(_) => {
//                     todo!();
//                 }
//             }
//         }
//     }
// }

// fn find_neighbors(pos: &(i32, i32)) -> Vec<(i32, i32)> {
//     let mut neighbors: Vec<(i32, i32)> = Vec::new();
//     for x in -1..2 {
//         for y in -1..2 {
//             if x != 0 || y != 0 {
//                 neighbors.push((pos.0 + x, pos.1 + y));
//             }
//         }
//     }
//     neighbors
// }

// pub fn build_random(board: &mut HashSet<(i32, i32)>, rng: &mut SmallRng, size: usize) {
//     for i in 0..size {
//         for j in 0..size {
//             if rng.gen_bool(0.5) {
//                 board.insert((i as i32, j as i32));
//             }
//         }
//     }
// }

// // TODO rewrite this
// pub fn build_ir(board: &HashSet<(i32, i32)>, size: usize) -> Vec<Complex<f32>> {
//     let mut out = vec![Complex::<f32> { re: 0.0, im: 0.0 }; size];
//     let mut i = 0;
//     for cell in board.iter() {
//         if i % 2 == 0 {
//             out[cell.0 as usize].re += 1 as f32 / size as f32;
//             out[cell.1 as usize].im += 1 as f32 / size as f32;
//         } else {
//             out[cell.0 as usize].re -= 1 as f32 / size as f32;
//             out[cell.1 as usize].im -= 1 as f32 / size as f32;
//         }
//         i += 1;
//     }
//     out
// }

// fn build_ir_but_actually() {
//     let mut planner = RealFftPlanner::new();
//     let real_to_complex = planner.plan_fft_forward(FFT_WINDOW_SIZE);
//     let complex_to_real = planner.plan_fft_inverse(FFT_WINDOW_SIZE);

//     let mut comp_buff = real_to_complex.make_output_vec();
//     let mut real_buff = real_to_complex.make_input_vec();

//     let mut ir: Vec<f32> = vec![0.0; FILTER_WINDOW_SIZE];
//     for i in 0..FILTER_WINDOW_SIZE {
//         ir[i] = {
//             let mut out = 0.0;
//             for j in 0..FILTER_WINDOW_SIZE {
//                 let b_ij = match alive_cells.contains(&(i as i32, j as i32)) {
//                     true => 1.0,
//                     false => -1.0,
//                 };
//                 let b_ji = match alive_cells.contains(&(j as i32, i as i32)) {
//                     true => 1.0,
//                     false => -1.0,
//                 };

//                 out += b_ij + b_ji;
//             }

//             out /= FILTER_WINDOW_SIZE as f32;
//             out
//         }
//     }

//     real_buff[0..FILTER_WINDOW_SIZE].copy_from_slice(&ir[0..FILTER_WINDOW_SIZE]);

//     real_to_complex
//         .process_with_scratch(&mut real_buff, &mut comp_buff, &mut [])
//         .unwrap();
// }
