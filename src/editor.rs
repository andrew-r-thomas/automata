use crate::{consts::*, AutomataParams};

use std::sync::mpsc::{self, Sender};
use std::sync::Arc;
use std::thread;

use nih_plug::editor::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};
use realfft::num_complex::Complex;
use rtrb::*;

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
        event.map(|gui_event: &GUIEvent, _| match gui_event {
            GUIEvent::PlayPause => {
                self.running = !self.running;
                match self.game_loop_sender.send(GUIEvent::PlayPause) {
                    Ok(_) => {}
                    Err(_) => {
                        todo!()
                    }
                }
            }
            GUIEvent::Reset => {
                self.running = false;
                match self.game_loop_sender.send(GUIEvent::Reset) {
                    Ok(_) => {}
                    Err(_) => {
                        todo!()
                    }
                }
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
) -> (Consumer<Complex<f32>>, Option<Box<dyn Editor>>) {
    let (s, r) = mpsc::channel::<GUIEvent>();

    // TODO we are probably fine with this size being times 2
    // but we will have issues if our audio thread is popping slower
    // than our game thread pushes
    let (prod, cons) = RingBuffer::<Complex<f32>>::new(FILTER_WINDOW_SIZE * 2);

    thread::spawn(move || game_loop(r, prod));

    let e = create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
            game_loop_sender: s.clone(),
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
            Button::new(cx, |_| {}, |cx| Label::new(cx, "button"));
            Button::new(cx, |_| {}, |cx| Label::new(cx, "other button"));
        })
        .row_between(Pixels(0.0))
        .child_left(Stretch(1.0))
        .child_right(Stretch(1.0));
    });

    (cons, e)
}
