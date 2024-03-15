use nih_plug::editor::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use rand::rngs::ThreadRng;
use rand::Rng;
use realfft::num_complex::Complex;
// use rtrb::*;
use std::collections::HashSet;
// use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::Arc;
// use std::thread;

use crate::AutomataParams;

#[derive(Lens)]
struct Data {
    params: Arc<AutomataParams>,
    // game_loop_sender: Sender<GUIEvent>,
    running: bool,
}

pub enum GUIEvent {
    PlayPause,
    Reset,
}

impl Model for Data {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|gui_event: &GUIEvent, _| match gui_event {
            GUIEvent::PlayPause => {
                self.running = !self.running;
                // match self.game_loop_sender.send(GUIEvent::PlayPause) {
                //     Ok(_) => {}
                //     Err(_) => {
                //         todo!()
                //     }
                // }
            }
            GUIEvent::Reset => {
                self.running = false;
                // match self.game_loop_sender.send(GUIEvent::Reset) {
                //     Ok(_) => {}
                //     Err(_) => {
                //         todo!()
                //     }
                // }
            }
        });
    }
}

// Makes sense to also define this here, makes it a bit easier to keep track of
pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (400, 300))
}

pub(crate) fn create(
    params: Arc<AutomataParams>,
    editor_state: Arc<ViziaState>,
) -> Option<Box<dyn Editor>> {
    // let (s, r) = mpsc::channel::<GUIEvent>();

    // TODO we are probably fine with this size being times 2
    // but we will have issues if our audio thread is popping slower
    // than our game thread pushes
    // let (prod, cons) = RingBuffer::<Complex<f32>>::new(DEFAULT_IR_SPECTRUM_SIZE * 2);

    // thread::spawn(move || game_loop(r, prod));

    let e = create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
            // game_loop_sender: s.clone(),
            running: false,
        }
        .build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Automata")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_size(30.0)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0));
            Button::new(cx, |_| {}, |cx| Label::new(cx, "start/stop"));
            Button::new(cx, |_| {}, |cx| Label::new(cx, "reset"));
        })
        .row_between(Pixels(0.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    });

    // (cons, e)
    e
}

// fn game_loop(gui_reciever: Receiver<GUIEvent>, mut ir_producer: Producer<Complex<f32>>) {
//     let mut alive_cells =
//         HashSet::<(i32, i32)>::with_capacity(DEFAULT_IR_SPECTRUM_SIZE * DEFAULT_IR_SPECTRUM_SIZE);
//     let mut game_running = false;
//     let mut rng = rand::thread_rng();
//     build_random(&mut alive_cells, &mut rng, DEFAULT_IR_SPECTRUM_SIZE);

//     loop {
//         let message = gui_reciever.try_recv();
//         match message {
//             Ok(GUIEvent::PlayPause) => {
//                 game_running = !game_running;
//             }
//             Ok(GUIEvent::Reset) => {
//                 game_running = false;
//                 alive_cells.drain();
//                 build_random(&mut alive_cells, &mut rng, DEFAULT_IR_SPECTRUM_SIZE);
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
//             match ir_producer.write_chunk_uninit(DEFAULT_IR_SPECTRUM_SIZE) {
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
