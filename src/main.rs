use nih_plug::prelude::*;
use rand::rngs::ThreadRng;
use rand::Rng;
use realfft::num_complex::Complex;
use realfft::*;
use std::collections::HashSet;

const WINDOW_SIZE: usize = 64;
const FILTER_WINDOW_SIZE: usize = 33;
const GAME_BOARD_SIZE: usize = (FFT_WINDOW_SIZE / 2) + 1;
pub const SMOOVE: [f32; FILTER_WINDOW_SIZE] =
    [1 as f32 / FILTER_WINDOW_SIZE as f32; FILTER_WINDOW_SIZE];
const FFT_WINDOW_SIZE: usize = WINDOW_SIZE + FILTER_WINDOW_SIZE + 1;

const GAIN_COMP: f32 = 1.0 / FFT_WINDOW_SIZE as f32;

fn main() {
    let mut planner = RealFftPlanner::new();
    let real_to_complex = planner.plan_fft_forward(FFT_WINDOW_SIZE);
    let complex_to_real = planner.plan_fft_inverse(FFT_WINDOW_SIZE);

    let mut comp_buff = real_to_complex.make_output_vec();

    let mut alive_cells = HashSet::<(i32, i32)>::with_capacity(GAME_BOARD_SIZE * GAME_BOARD_SIZE);
    let mut rng = rand::thread_rng();
    build_random(&mut alive_cells, &mut rng, GAME_BOARD_SIZE);
    let ir = build_ir(&alive_cells, GAME_BOARD_SIZE);
    assert!(ir.len() == GAME_BOARD_SIZE);

    let mut real_buff = real_to_complex.make_input_vec();
    let mut filter_window = util::window::hann(FILTER_WINDOW_SIZE);
    real_buff[0..FILTER_WINDOW_SIZE].copy_from_slice(&filter_window);

    real_to_complex
        .process_with_scratch(&mut real_buff, &mut comp_buff, &mut [])
        .unwrap();

    println!("hann filter len: {}", comp_buff.len());
    println!("game board len: {}", ir.len());

    println!("hann filter numbers");
    for num in comp_buff {
        println!("{}", num);
    }

    println!("\ngame board numbers");
    for num in ir {
        println!("{}", num);
    }
}

pub fn build_random(board: &mut HashSet<(i32, i32)>, rng: &mut ThreadRng, size: usize) {
    for i in 0..size {
        for j in 0..size {
            if rng.gen_bool(0.5) {
                board.insert((i as i32, j as i32));
            }
        }
    }
}

pub fn build_ir(board: &HashSet<(i32, i32)>, size: usize) -> Vec<Complex<f32>> {
    let mut out = vec![Complex::<f32> { re: 0.0, im: 0.0 }; size];
    let mut i = 0;
    for cell in board.iter() {
        if i % 2 == 0 {
            out[cell.0 as usize].re += 1 as f32 / size as f32;
            out[cell.1 as usize].im += 1 as f32 / size as f32;
        } else {
            out[cell.0 as usize].re -= 1 as f32 / size as f32;
            out[cell.1 as usize].im -= 1 as f32 / size as f32;
        }
        i += 1;
    }
    out
}
