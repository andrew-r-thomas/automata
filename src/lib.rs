pub mod editor;

use editor::DEFAULT_IR_SIZE;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use rtrb::*;
use std::sync::Arc;

// This is a shortened version of the gain example with most comments removed, check out
// https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs to get
// started

struct Automata {
    params: Arc<AutomataParams>,
    ir_consumer: Option<Consumer<Complex<f32>>>,
    // TODO might want to replace vecs with arrays
    current_ir: Vec<Complex<f32>>,
    fft: Option<Arc<dyn RealToComplex<f32>>>,
    ifft: Option<Arc<dyn ComplexToReal<f32>>>,
    fft_input: Vec<f32>,
    fft_output: Vec<Complex<f32>>,
    ifft_input: Vec<Complex<f32>>,
    ifft_output: Vec<f32>,
    // TODO figure out what the channels are like
    output_buff: Option<Vec<&'static mut [f32]>>,
}

#[derive(Params)]
struct AutomataParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Automata {
    fn default() -> Self {
        Self {
            params: Arc::new(AutomataParams::default()),
            ir_consumer: None,
            current_ir: Vec::with_capacity(DEFAULT_IR_SIZE),
            fft: None,
            ifft: None,
            fft_input: Vec::with_capacity(DEFAULT_IR_SIZE * 2),
            fft_output: Vec::with_capacity(DEFAULT_IR_SIZE),
            ifft_input: Vec::with_capacity(DEFAULT_IR_SIZE),
            ifft_output: Vec::with_capacity(DEFAULT_IR_SIZE * 2),
            output_buff: None,
        }
    }
}

impl Default for AutomataParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
        }
    }
}

impl Plugin for Automata {
    const NAME: &'static str = "Automata";
    const VENDOR: &'static str = "Andrew Thomas";
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "andrew.r.j.thomas@gmail.com";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // The first audio IO layout is used as the default. The other layouts may be selected either
    // explicitly or automatically by the host or the user depending on the plugin API/backend.
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],

        // Individual ports and the layout as a whole can be named here. By default these names
        // are generated as needed. This layout will be called 'Stereo', while a layout with
        // only one input and output channel would be called 'Mono'.
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    // If the plugin can send or receive SysEx messages, it can define a type to wrap around those
    // messages here. The type implements the `SysExMessage` trait, which allows conversion to and
    // from plain byte buffers.
    type SysExMessage = ();
    // More advanced plugins can use this to run expensive background tasks. See the field's
    // documentation for more information. `()` means that the plugin does not have any background
    // tasks.
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let (cons, e) = editor::create(self.params.clone(), self.params.editor_state.clone());
        self.ir_consumer = Some(cons);
        e
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        self.current_ir = Vec::with_capacity(DEFAULT_IR_SIZE);

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(DEFAULT_IR_SIZE * 2);
        let ifft = planner.plan_fft_inverse(DEFAULT_IR_SIZE * 2);

        let fft_input = fft.make_input_vec();
        let fft_output = fft.make_output_vec();
        let ifft_input = ifft.make_input_vec();
        let ifft_output = ifft.make_output_vec();

        self.fft = Some(fft);
        self.ifft = Some(ifft);
        self.fft_input = fft_input;
        self.fft_output = fft_output;
        self.ifft_input = ifft_input;
        self.ifft_output = ifft_output;

        self.output_buff = Some(Vec::with_capacity(
            buffer_config.max_buffer_size as usize + DEFAULT_IR_SIZE,
        ));

        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // TODO figure out how to handle panic here
        // TODO might want to make ir a vec with capacity instead of array
        // because of how realfft handles things
        match self.ir_consumer.as_mut() {
            Some(c) => match c.read_chunk(DEFAULT_IR_SIZE) {
                Ok(ir) => {
                    self.current_ir = ir.into_iter().collect();
                }
                Err(_) => {
                    todo!()
                }
            },
            None => panic!("ir consumer has not been initialized"),
        }
        let raw = buffer.as_slice();

        for channel_index in 0..buffer.channels() {
            // Do 0 padding
            self.fft_input.fill(0.0);
            self.fft_input[0..(DEFAULT_IR_SIZE / 2)].copy_from_slice(slice);

            let _ = self
                .fft
                .as_ref()
                .unwrap()
                .process(&mut self.fft_input, &mut self.fft_output);

            // TODO see if we can simd
            for i in 0..self.fft_output.len() {
                self.ifft_input[i] = self.fft_output[i] * self.current_ir[i];
            }

            let _ = self
                .ifft
                .as_ref()
                .unwrap()
                .process(&mut self.ifft_input, &mut self.ifft_output);
        }

        // TODO reset output buff

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Automata {
    const CLAP_ID: &'static str = "com.diy!studios.automata";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A short description of your plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Automata {
    const VST3_CLASS_ID: [u8; 16] = *b"diy!studiosatmta";

    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

// nih_export_clap!(Automata);
nih_export_vst3!(Automata);
