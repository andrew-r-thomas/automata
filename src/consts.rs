pub const WINDOW_SIZE: usize = 512;
pub const FILTER_WINDOW_SIZE: usize = 257;
pub const GAME_BOARD_SIZE: usize = (FFT_WINDOW_SIZE / 2) + 1;
pub const SMOOVE: [f32; FILTER_WINDOW_SIZE] =
    [1 as f32 / FILTER_WINDOW_SIZE as f32; FILTER_WINDOW_SIZE];
pub const FFT_WINDOW_SIZE: usize = WINDOW_SIZE + FILTER_WINDOW_SIZE - 1;

pub const GAIN_COMP: f32 = 16.0 / FFT_WINDOW_SIZE as f32;

pub const SEED: u64 = 420;
