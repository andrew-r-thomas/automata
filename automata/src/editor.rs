use nih_plug::editor::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use rand::rngs::ThreadRng;
use rand::Rng;
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
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        let (s, r) = mpsc::channel::<GUIEvent>();
        thread::spawn(move || game_loop(r));

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
fn game_loop(gui_reciever: Receiver<GUIEvent>) {
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
            todo!()
        }
    }
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
