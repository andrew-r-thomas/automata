pub mod consts;
pub mod editor;
pub mod gol;
pub mod gol_utils;

use std::sync::{Arc, Mutex};

use consts::*;

use gol::GOL;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, FftError, RealFftPlanner, RealToComplex};
use rtrb::{Consumer, RingBuffer};

struct Automata {
    params: Arc<AutomataParams>,

    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,

    stft: util::StftHelper,

    comp_buff: Vec<Complex<f32>>,
    game_comp_buff: Vec<Complex<f32>>,

    cons: Option<Consumer<Complex<f32>>>,
}

enum Tasks {
    CalcStep,
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

        let comp_buff = ifft.make_input_vec();
        let game_comp_buff = fft.make_output_vec();

        Self {
            params: Arc::new(AutomataParams::default()),

            fft,
            ifft,

            stft: util::StftHelper::new(2, WINDOW_SIZE, FFT_WINDOW_SIZE - WINDOW_SIZE),

            comp_buff,
            game_comp_buff,

            cons: None,
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

    type BackgroundTask = Tasks;

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let (prod, cons) = RingBuffer::<Complex<f32>>::new(self.game_comp_buff.len() * 3);
        let gol = GOL::new(prod, FILTER_WINDOW_SIZE, FFT_WINDOW_SIZE, SEED);
        let protec = Arc::new(Mutex::new(gol));

        self.cons = Some(cons);

        Box::new(move |task: Tasks| match task {
            Tasks::CalcStep => match protec.try_lock() {
                Ok(mut gol_lock) => gol_lock.advance(),
                Err(_) => nih_log!("error taking lock"),
            },
        })
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
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
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if self.params.running.value() {
            context.execute_background(Tasks::CalcStep);
        }

        match self
            .cons
            .as_mut()
            .expect("initialized in task executor func")
            .read_chunk(self.game_comp_buff.len())
        {
            Ok(c) => {
                let (s1, s2) = c.as_slices();
                let len1 = s1.len();
                let len2 = s2.len();

                self.game_comp_buff[0..len1].copy_from_slice(s1);
                self.game_comp_buff[0..len2].copy_from_slice(s2);

                c.commit_all();
            }
            Err(_) => nih_log!("error reading chunk"),
        }

        self.stft
            .process_overlap_add(buffer, 1, |_channel, real_buff| {
                match self
                    .fft
                    .process_with_scratch(real_buff, &mut self.comp_buff, &mut [])
                {
                    Ok(_) => {}
                    Err(_e) => {
                        nih_log!("audio fft error");
                        panic!()
                    }
                };

                for (fft_bin, kernel_bin) in self.comp_buff.iter_mut().zip(&self.game_comp_buff) {
                    *fft_bin *= *kernel_bin * GAIN_COMP;
                }

                match self
                    .ifft
                    .process_with_scratch(&mut self.comp_buff, real_buff, &mut [])
                {
                    Ok(_) => {}
                    Err(e) => match e {
                        FftError::InputBuffer(_, _) => {
                            nih_log!("ifft error: input buffer");
                        }
                        FftError::OutputBuffer(_, _) => {
                            nih_log!("ifft error: output buffer");
                        }
                        FftError::ScratchBuffer(_, _) => {
                            nih_log!("ifft error: scratch buffer");
                        }
                        FftError::InputValues(_, _) => {
                            nih_log!("ifft error: input values");
                        }
                    },
                };
            });

        ProcessStatus::Normal
    }
}

impl Vst3Plugin for Automata {
    const VST3_CLASS_ID: [u8; 16] = *b"diy!studios_auto";

    // TODO
    // And also don't forget to change these categories
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Dynamics];
}

nih_export_vst3!(Automata);
