// TODO for some reason we are crashing when using the
// game board, it's probably some sizing stuff being wrong
// because we are trying to fit the spectrum into the filter
// and doing it incorrectly
// it could also be not doing the normalizing, which
// probably should be done game side

pub mod editor;

use std::collections::HashSet;
use std::sync::Arc;

use editor::{build_ir, build_random};
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use rand::{Rng, SeedableRng};
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use rtrb::*;

const WINDOW_SIZE: usize = 64;
const FILTER_WINDOW_SIZE: usize = 33;
const GAME_BOARD_SIZE: usize = (FFT_WINDOW_SIZE / 2) + 1;
pub const SMOOVE: [f32; FILTER_WINDOW_SIZE] =
    [1 as f32 / FILTER_WINDOW_SIZE as f32; FILTER_WINDOW_SIZE];
const FFT_WINDOW_SIZE: usize = WINDOW_SIZE + FILTER_WINDOW_SIZE + 1;

const GAIN_COMP: f32 = 1.0 / FFT_WINDOW_SIZE as f32;

struct Automata {
    params: Arc<AutomataParams>,

    _ir_consumer: Option<Consumer<Complex<f32>>>,
    current_ir: Vec<Complex<f32>>,

    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,

    stft: util::StftHelper,

    comp_buff: Vec<Complex<f32>>,
}

#[derive(Params)]
struct AutomataParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Automata {
    fn default() -> Self {
        let mut planner = RealFftPlanner::new();
        let real_to_complex = planner.plan_fft_forward(FFT_WINDOW_SIZE);
        let complex_to_real = planner.plan_fft_inverse(FFT_WINDOW_SIZE);

        let mut comp_buff = real_to_complex.make_output_vec();
        let mut real_buff = real_to_complex.make_input_vec();

        let mut rng = rand::rngs::SmallRng::seed_from_u64(42);
        let mut rand_vec: [Complex<f32>; 50] = [Complex { re: 0.0, im: 0.0 }; 50];

        for s in &mut rand_vec {
            let re = rng.gen_range(-1.0..1.0);
            let im = rng.gen_range(-1.0..1.0);
            *s += Complex { re, im };
        }

        // let filter_normalization_factor = rand_vec.iter().sum::<f32>().recip();
        // for sample in &mut rand_vec {
        //     *sample *= filter_normalization_factor;
        // }

        // real_buff[0..FILTER_WINDOW_SIZE].copy_from_slice(&rand_vec[0..FILTER_WINDOW_SIZE]);

        // real_to_complex
        //     .process_with_scratch(&mut real_buff, &mut comp_buff, &mut [])
        //     .unwrap();

        assert!(comp_buff.capacity() == 50);
        comp_buff[0..50].copy_from_slice(&rand_vec[0..50]);

        Self {
            params: Arc::new(AutomataParams::default()),

            _ir_consumer: None,
            current_ir: comp_buff.clone(),

            fft: real_to_complex,
            ifft: complex_to_real,

            stft: util::StftHelper::new(2, WINDOW_SIZE, FFT_WINDOW_SIZE - WINDOW_SIZE),

            comp_buff,
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
        // TODO we might not want to do this in the editor funtion
        nih_log!("we are trying to make editor");
        let e = editor::create(self.params.clone(), self.params.editor_state.clone());
        // self.ir_consumer = Some(cons);
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

        // TODO figure out how to handle panic here
        // match self.ir_consumer.as_mut() {
        //     Some(c) => match c.read_chunk(DEFAULT_IR_SPECTRUM_SIZE) {
        //         Ok(ir) => {
        //             let slices = ir.as_slices();
        //             self.current_ir[0..slices.0.len()].copy_from_slice(slices.0);
        //             self.current_ir[slices.0.len()..slices.0.len() + slices.1.len()]
        //                 .copy_from_slice(slices.1);
        //             ir.commit_all()
        //         }
        //         Err(e) => {
        //             nih_log!("{}", e);
        //         }
        //     },
        //     None => {
        //         nih_log!("no ir buff");
        //         return ProcessStatus::Normal;
        //     }
        // }

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

        // // TODO do process status stuff for reverb tail
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
