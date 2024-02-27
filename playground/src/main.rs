use nannou::prelude::*;

fn main() {
    nannou::app(model).update(update).run();
}

struct Model {
    window: window::Id,
    board: Vec<Vec<bool>>,
}

fn model(app: &App) -> Model {
    let window = app.new_window().fullscreen().view(view).build().unwrap();
    // TODO make patterns const (probably need arrays for this)
    let blank = vec![vec![false; 128]];
    let blinker = vec![
        vec![vec![false; 128]; 64],
        vec![vec![vec![false; 64], vec![true; 3], vec![false; 61]].concat()],
        vec![vec![false; 128]; 63],
    ]
    .concat();

    Model {
        window,
        board: blinker,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let mut board = model.board.clone();
    for row_idx in 0..board.len() {
        for col_idx in 0..board.len() {
            let cell = board[row_idx][col_idx];
            let adj: u8 = {
                let tl = match board.get(row_idx - 1) {
                    Some(x) => match x.get(col_idx - 1) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };

                let tc = match board.get(row_idx - 1) {
                    Some(x) => match x.get(col_idx) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };

                let tr = match board.get(row_idx - 1) {
                    Some(x) => match x.get(col_idx + 1) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };

                let cl = match board.get(row_idx) {
                    Some(x) => match x.get(col_idx - 1) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };

                let cr = match board.get(row_idx) {
                    Some(x) => match x.get(col_idx + 1) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };

                let bl = match board.get(row_idx + 1) {
                    Some(x) => match x.get(col_idx - 1) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };

                let bc = match board.get(row_idx + 1) {
                    Some(x) => match x.get(col_idx) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };
                let br = match board.get(row_idx + 1) {
                    Some(x) => match x.get(col_idx + 1) {
                        Some(x) => *x as u8,
                        None => 0,
                    },
                    None => 0,
                };

                tl + tc + tr + cl + cr + bl + bc + br
            };

            if cell {
                if adj < 2 || adj > 3 {
                    board[row_idx][col_idx] = false;
                }
            }

            if !cell && adj == 3 {
                board[row_idx][col_idx] = true;
            }
        }
    }

    model.board = board;
}

fn view(app: &App, model: &Model, frame: Frame) {
    let draw = app.draw();
    draw.background().color(BLACK);

    let mut row_offset = 0.0;
    let mut column_offset = 0.0;
    let x_start = 800.0;
    let y_start = 500.0;

    for row in model.board.iter() {
        row_offset -= 7.5;
        for cell in row.iter() {
            column_offset -= 7.5;
            if *cell {
                draw.ellipse()
                    .x_y(x_start + column_offset, y_start + row_offset)
                    .radius(2.5)
                    .color(WHITE);
            } else {
                draw.ellipse()
                    .x_y(x_start + column_offset, y_start + row_offset)
                    .radius(2.5)
                    .color(BLACK);
            }
        }
        column_offset = 0.0;
    }

    draw.to_frame(app, &frame).unwrap();
}