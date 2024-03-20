pub mod consts;
pub mod editor;
pub mod gol_utils;

use crate::gol_utils::{build_random, step};
use std::collections::HashSet;
use std::sync::Arc;

use consts::*;

use editor::GUIEvent;
use gol_utils::build_ir;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use rand::rngs::SmallRng;
use rand::SeedableRng;
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};

struct Automata {
    params: Arc<AutomataParams>,

    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,

    stft: util::StftHelper,

    comp_buff: Vec<Complex<f32>>,
    real_buff: Vec<f32>,

    current_ir: Vec<Complex<f32>>,
    current_board: HashSet<(i32, i32)>,
    dying_buff: Vec<(i32, i32)>,
    born_buff: Vec<(i32, i32)>,
}

#[derive(Params)]
struct AutomataParams {
    #[id = "running"]
    running: BoolParam,

    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Automata {
    fn default() -> Self {
        let mut planner = RealFftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_WINDOW_SIZE);
        let ifft = planner.plan_fft_inverse(FFT_WINDOW_SIZE);

        let mut comp_buff = ifft.make_input_vec();
        let mut real_buff = fft.make_input_vec();

        let mut current_board: HashSet<(i32, i32)> =
            HashSet::with_capacity(FILTER_WINDOW_SIZE * FILTER_WINDOW_SIZE);
        let mut rng = SmallRng::seed_from_u64(SEED);
        let born_buff = Vec::with_capacity(FILTER_WINDOW_SIZE * FILTER_WINDOW_SIZE);
        let dying_buff = Vec::with_capacity(FILTER_WINDOW_SIZE * FILTER_WINDOW_SIZE);

        build_random(&mut current_board, &mut rng);
        build_ir(&mut current_board, &mut real_buff);

        fft.process_with_scratch(&mut real_buff, &mut comp_buff, &mut [])
            .unwrap();

        Self {
            params: Arc::new(AutomataParams::default()),

            current_ir: comp_buff.clone(),
            current_board: HashSet::with_capacity(FILTER_WINDOW_SIZE * FILTER_WINDOW_SIZE),

            fft,
            ifft,

            stft: util::StftHelper::new(2, WINDOW_SIZE, FFT_WINDOW_SIZE - WINDOW_SIZE),

            comp_buff,
            real_buff,
            born_buff,
            dying_buff,
        }
    }
}

impl Default for AutomataParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),
            running: BoolParam::new("running", false),
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

    type BackgroundTask = GUIEvent;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // TODO we might not want to do this in the editor funtion
        let e = editor::create(
            self.params.clone(),
            self.params.editor_state.clone(),
            async_executor.clone(),
        );
        e
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("initializing");

        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        context.set_latency_samples(self.stft.latency_samples() + (FILTER_WINDOW_SIZE as u32 / 2));

        true
    }

    fn reset(&mut self) {
        // Reset buffers and envelopes here. This can be called from the audio thread and may not
        // allocate. You can remove this function if you do not need it.

        self.stft.set_block_size(WINDOW_SIZE);
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        nih_log!("doing a process block");

        if self.params.running.value() {
            step(
                &mut self.current_board,
                &mut self.born_buff,
                &mut self.dying_buff,
                &mut self.real_buff,
            );

            self.fft
                .process_with_scratch(&mut self.real_buff, &mut self.comp_buff, &mut [])
                .unwrap();

            self.current_ir.copy_from_slice(&self.comp_buff);
        }

        self.stft
            .process_overlap_add(buffer, 1, |_channel, real_buff| {
                self.fft
                    .process_with_scratch(real_buff, &mut self.comp_buff, &mut [])
                    .unwrap();

                for (fft_bin, kernel_bin) in self.comp_buff.iter_mut().zip(&self.current_ir) {
                    *fft_bin *= *kernel_bin * GAIN_COMP;
                }

                self.ifft
                    .process_with_scratch(&mut self.comp_buff, real_buff, &mut [])
                    .unwrap();
            });

        ProcessStatus::Normal
    }
}

// NOTE just testing this
impl ClapPlugin for Automata {
    const CLAP_ID: &'static str = "com.diy!studios.automata";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("A short description of your plugin");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    // Don't forget to change these features
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for Automata {
    const VST3_CLASS_ID: [u8; 16] = *b"diy!studios_auto";

    // TODO
    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

// nih_export_clap!(Automata);
nih_export_vst3!(Automata);
