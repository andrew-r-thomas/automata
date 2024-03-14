// TODO so its not the threads that are the issue
// its something to do with how the ffts are done,
// maybe the sizes?

pub mod consts;
pub mod editor;

use crate::consts::*;

use std::env::set_var;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use rtrb::*;

struct Automata {
    params: Arc<AutomataParams>,
    ir_consumer: Option<Consumer<Complex<f32>>>,
    current_ir: Vec<Complex<f32>>,
    fft: Option<Arc<dyn RealToComplex<f32>>>,
    ifft: Option<Arc<dyn ComplexToReal<f32>>>,
    fft_input: Vec<f32>,
    fft_output: Vec<Complex<f32>>,
    ifft_input: Vec<Complex<f32>>,
    ifft_output: Vec<f32>,
    output_buff: Vec<Vec<f32>>,
}

#[derive(Params)]
struct AutomataParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Automata {
    fn default() -> Self {
        set_var("NIH_LOG", "~/dev/diy!/automata/log.md");
        nih_log!("we are making default plugin");

        Self {
            params: Arc::new(AutomataParams::default()),
            ir_consumer: None,
            current_ir: Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE),
            fft: None,
            ifft: None,
            fft_input: Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE * 2),
            fft_output: Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE),
            ifft_input: Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE),
            ifft_output: Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE * 2),
            output_buff: vec![
                Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE + 512),
                Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE + 512),
            ],
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
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_log!("initializing");

        // Resize buffers and perform other potentially expensive initialization operations here.
        // The `reset()` function is always called right after this function. You can remove this
        // function if you do not need it.
        self.current_ir = Vec::with_capacity(DEFAULT_IR_SPECTRUM_SIZE);
        self.current_ir.fill(Complex { re: 0.0, im: 0.0 });

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(DEFAULT_FFT_SIZE);
        let ifft = planner.plan_fft_inverse(DEFAULT_FFT_SIZE);

        let mut fft_input = fft.make_input_vec();
        fft_input[0..DEFAULT_WINDOW_SIZE].copy_from_slice(&SMOOVE);
        let _ = fft.process(&mut fft_input, &mut self.current_ir);

        fft_input.fill(0.0);
        let mut fft_output = fft.make_output_vec();
        fft_output.fill(Complex { re: 0.0, im: 0.0 });
        let mut ifft_input = ifft.make_input_vec();
        ifft_input.fill(Complex { re: 0.0, im: 0.0 });
        let mut ifft_output = ifft.make_output_vec();
        ifft_output.fill(0.0);

        self.fft = Some(fft);
        self.ifft = Some(ifft);
        self.fft_input = fft_input;
        self.fft_output = fft_output;
        self.ifft_input = ifft_input;
        self.ifft_output = ifft_output;

        self.output_buff = vec![
            Vec::with_capacity(
                buffer_config.max_buffer_size as usize + DEFAULT_IR_SPECTRUM_SIZE
            );
            2
        ];

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

        let channels = buffer.channels();
        let mut cursor = 0;
        for block in buffer.iter_blocks(DEFAULT_WINDOW_SIZE) {
            let block_len = block.1.samples();
            let mut blocks = block.1.into_iter();

            for channel in 0..channels {
                let channel_block = blocks.next().unwrap();
                // NOTE the len of our blocks can be different from default window size

                self.fft_input[0..block_len].copy_from_slice(channel_block);
                match self
                    .fft
                    .as_ref()
                    .unwrap()
                    .process(&mut self.fft_input, &mut self.ifft_input)
                {
                    Ok(_) => {}
                    Err(_) => todo!(),
                }

                // TODO simd this
                for i in 0..self.current_ir.len().min(self.ifft_input.len()) {
                    self.ifft_input[i] *= self.current_ir[i];
                }

                match self
                    .ifft
                    .as_ref()
                    .unwrap()
                    .process(&mut self.ifft_input, &mut self.ifft_output)
                {
                    Ok(_) => {}
                    Err(_) => todo!(),
                }

                // TODO this is all kinds of slow and bad
                //     for i in cursor..cursor + self.ifft_output.len() {
                //         self.output_buff[channel][i] += self.ifft_output[i];
                //     }
                //     channel_block
                //         .copy_from_slice(&self.output_buff[channel][cursor..cursor + block_len])
            }

            // cursor += block_len;
        }

        // for i in 0..2 {
        //     self.output_buff[i].rotate_right(DEFAULT_IR_SPECTRUM_SIZE);
        //     self.output_buff[i][DEFAULT_IR_SPECTRUM_SIZE..].fill(0.0);
        // }

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
