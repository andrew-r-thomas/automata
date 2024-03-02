use nih_plug::editor::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use rand::rngs::ThreadRng;
use rand::Rng;
use realfft::num_complex::Complex;
use rtrb::*;
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::sync::{mpsc, Arc};
use std::thread;

use crate::AutomataParams;

#[derive(Lens)]
struct Data {
    params: Arc<AutomataParams>,
    game_loop_sender: Sender<GUIEvent>,
    running: bool,
}

pub enum GUIEvent {
    PlayPause,
    Reset,
}

impl Model for Data {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|gui_event, _| match gui_event {
            GUIEvent::PlayPause => {
                if self.running {
                    self.running = false;
                }
            }
            GUIEvent::Reset => {}
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
    ir_producer: Producer<Vec<(i32, i32)>>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        let (s, r) = mpsc::channel::<GUIEvent>();
        thread::spawn(move || game_loop(r, ir_producer.push(vec![(1, 1)])));

        Data {
            params: params.clone(),
            game_loop_sender: s,
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
            Button::new(cx, |_| {}, |cx| Label::new(cx, "start"));
            Button::new(cx, |_| {}, |cx| Label::new(cx, "reset"));
        })
        .row_between(Pixels(0.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    })
}

const DEFAULT_SIZE: usize = 64;
fn game_loop(gui_reciever: Receiver<GUIEvent>, ir_producer: Producer<Vec<(i32, i32)>>) {
    let mut alive_cells = HashSet::<(i32, i32)>::with_capacity(DEFAULT_SIZE * DEFAULT_SIZE);
    let mut game_running = false;
    let mut rng = rand::thread_rng();
    build_random(&mut alive_cells, &mut rng, DEFAULT_SIZE);

    loop {
        let message = gui_reciever.try_recv();
        match message {
            Ok(GUIEvent::PlayPause) => {
                game_running = !game_running;
            }
            Ok(GUIEvent::Reset) => {
                game_running = false;
                alive_cells.drain();
                build_random(&mut alive_cells, &mut rng, DEFAULT_SIZE);
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => panic!("gui disconnected from game loop"),
        }

        if game_running {
            let mut dying: Vec<(i32, i32)> = Vec::new();
            let mut born: Vec<(i32, i32)> = Vec::new();

            for cell in alive_cells.iter() {
                let neighbors = find_neighbors(&cell);
                let mut living_neighbors: u8 = 0;
                for neighbor in neighbors.iter() {
                    if alive_cells.contains(neighbor) {
                        living_neighbors += 1;
                    } else if !born.contains(neighbor) {
                        let neighbors_neighbors = find_neighbors(neighbor);
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
fn build_random(board: &mut HashSet<(i32, i32)>, rng: &mut ThreadRng, size: usize) {
    for i in 0..size {
        for j in 0..size {
            if rng.gen_bool(0.5) {
                board.insert((i as i32, j as i32));
            }
        }
    }
}
