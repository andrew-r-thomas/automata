pub mod consts;
pub mod editor;

use std::collections::HashSet;
use std::sync::{mpsc, Arc};
use std::thread;

use consts::*;
use editor::GUIEvent::{self, PlayPause, Reset};

use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use realfft::num_complex::Complex;
use realfft::{ComplexToReal, RealFftPlanner, RealToComplex};
use rtrb::*;

struct Automata {
    params: Arc<AutomataParams>,

    fft: Arc<dyn RealToComplex<f32>>,
    ifft: Arc<dyn ComplexToReal<f32>>,

    stft: util::StftHelper,

    comp_buff: Vec<Complex<f32>>,

    ir_cons: Option<Consumer<Complex<f32>>>,
    current_ir: Vec<Complex<f32>>,
}

#[derive(Params)]
struct AutomataParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Automata {
    fn default() -> Self {
        let mut planner = RealFftPlanner::new();
        let fft = planner.plan_fft_forward(FFT_WINDOW_SIZE);
        let ifft = planner.plan_fft_inverse(FFT_WINDOW_SIZE);

        let comp_buff = ifft.make_input_vec();

        Self {
            params: Arc::new(AutomataParams::default()),

            current_ir: comp_buff.clone(),
            ir_cons: None,

            fft,
            ifft,

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

    type BackgroundTask = GUIEvent;
    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let (mut ir_prod, ir_cons) = rtrb::RingBuffer::<Complex<f32>>::new(FILTER_WINDOW_SIZE * 3);
        self.ir_cons = Some(ir_cons);

        let (message_sender, message_reciever) = mpsc::channel::<GUIEvent>();

        thread::spawn(move || {
            // initialize stuff
            let mut current_board: HashSet<(i32, i32)> =
                HashSet::with_capacity(FILTER_WINDOW_SIZE * FILTER_WINDOW_SIZE);
            let mut rng = SmallRng::seed_from_u64(SEED);
            let mut running = false;
            let mut planner = RealFftPlanner::new();
            let real_to_complex = planner.plan_fft_forward(FFT_WINDOW_SIZE);

            let mut comp_buff = real_to_complex.make_output_vec();
            let mut real_buff = real_to_complex.make_input_vec();

            // make some closures
            let mut build_random = |board: &mut HashSet<(i32, i32)>| {
                for i in 0..FILTER_WINDOW_SIZE {
                    for j in 0..FILTER_WINDOW_SIZE {
                        if rng.gen() {
                            board.insert((i as i32, j as i32));
                        }
                    }
                }
            };
            let find_neighbors = |pos: &(i32, i32)| -> Vec<(i32, i32)> {
                let mut neighbors: Vec<(i32, i32)> = Vec::new();
                for x in -1..2 {
                    for y in -1..2 {
                        if x != 0 || y != 0 {
                            neighbors.push((pos.0 + x, pos.1 + y));
                        }
                    }
                }
                neighbors
            };
            let build_ir = |board: &HashSet<(i32, i32)>,
                            comp_buff: &mut Vec<Complex<f32>>,
                            real_buff: &mut Vec<f32>| {
                let mut ir: Vec<f32> = vec![0.0; FILTER_WINDOW_SIZE];
                for i in 0..FILTER_WINDOW_SIZE {
                    ir[i] = {
                        let mut out = 0.0;
                        for j in 0..FILTER_WINDOW_SIZE {
                            let b_ij = match board.contains(&(i as i32, j as i32)) {
                                true => 1.0,
                                false => -1.0,
                            };
                            let b_ji = match board.contains(&(j as i32, i as i32)) {
                                true => 1.0,
                                false => -1.0,
                            };

                            out += b_ij + b_ji;
                        }

                        out /= FILTER_WINDOW_SIZE as f32;
                        out
                    }
                }

                real_buff[0..FILTER_WINDOW_SIZE].copy_from_slice(&ir[0..FILTER_WINDOW_SIZE]);

                // TODO might want to think about moving the fft to the audio thread
                // and just sending the real ir over
                real_to_complex
                    .process_with_scratch(real_buff, comp_buff, &mut [])
                    .unwrap();
            };

            // initialize a random board
            build_random(&mut current_board);

            let mut dying: Vec<(i32, i32)> = Vec::new();
            let mut born: Vec<(i32, i32)> = Vec::new();

            loop {
                match message_reciever.recv() {
                    Ok(x) => match x {
                        PlayPause => running = !running,
                        Reset => {
                            running = false;
                            current_board.clear();
                            build_random(&mut current_board);
                        }
                    },
                    Err(_) => todo!(),
                };

                if running {
                    for cell in current_board.iter() {
                        let neighbors = find_neighbors(&cell);
                        let mut living_neighbors: u8 = 0;
                        for neighbor in neighbors.iter() {
                            if current_board.contains(neighbor) {
                                living_neighbors += 1;
                            } else if !born.contains(neighbor) {
                                let neighbors_neighbors = find_neighbors(neighbor);
                                let mut neighbors_living_neighbors: u8 = 0;
                                for neighbor_neighbor in neighbors_neighbors.iter() {
                                    if current_board.contains(neighbor_neighbor) {
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
                        current_board.remove(cell);
                    }
                    for cell in born.iter() {
                        current_board.insert(*cell);
                    }
                    dying.clear();
                    born.clear();

                    // build impluse response and send it to audio thread
                    build_ir(&mut current_board, &mut comp_buff, &mut real_buff);
                    match ir_prod.write_chunk(FILTER_WINDOW_SIZE) {
                        Ok(mut chunk) => {
                            let slices = chunk.as_mut_slices();

                            let first_len = slices.0.len();
                            let second_len = slices.0.len();

                            slices.0[0..first_len].copy_from_slice(&comp_buff[0..first_len]);
                            slices.1[0..second_len]
                                .copy_from_slice(&comp_buff[first_len..first_len + second_len]);

                            chunk.commit_all();
                        }
                        Err(_) => {
                            todo!();
                        }
                    }

                    comp_buff.clear();
                }
            }
        });

        Box::new(move |task: GUIEvent| match message_sender.send(task) {
            Ok(_) => {}
            Err(_) => todo!(),
        })
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // TODO we might not want to do this in the editor funtion
        nih_log!("we are trying to make editor");
        let e = editor::create(
            self.params.clone(),
            self.params.editor_state.clone(),
            async_executor,
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

        // TODO figure out how to handle panic here
        match self
            .ir_cons
            .as_mut()
            .unwrap()
            .read_chunk(FILTER_WINDOW_SIZE)
        {
            Ok(ir) => {
                let slices = ir.as_slices();
                self.current_ir[0..slices.0.len()].copy_from_slice(slices.0);
                self.current_ir[slices.0.len()..slices.0.len() + slices.1.len()]
                    .copy_from_slice(slices.1);
                ir.commit_all()
            }
            Err(_e) => {
                // panic!()
            }
        };

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
