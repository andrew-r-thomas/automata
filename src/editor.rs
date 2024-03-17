use crate::AutomataParams;

use std::sync::Arc;

use nih_plug::editor::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::{assets, create_vizia_editor, ViziaState, ViziaTheming};

#[derive(Lens)]
struct Data {
    params: Arc<AutomataParams>,
}

pub enum GUIEvent {
    PlayPause,
    Reset,
}

impl Model for Data {
    fn event(&mut self, _: &mut EventContext, event: &mut Event) {
        event.map(|gui_event: &GUIEvent, _| match gui_event {
            GUIEvent::PlayPause => {}
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
    let e = create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            params: params.clone(),
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

    e
}
