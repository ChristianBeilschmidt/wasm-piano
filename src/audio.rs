use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static AUDIO_STATE: RefCell<AudioState> = RefCell::new(AudioState::new());
}

struct NoteVoice {
    master_gain: web_sys::GainNode,
    oscillators: Vec<web_sys::OscillatorNode>,
}

struct AudioState {
    ctx: Option<web_sys::AudioContext>,
    active_voices: HashMap<u8, NoteVoice>,
}

impl AudioState {
    fn new() -> Self {
        Self {
            ctx: None,
            active_voices: HashMap::new(),
        }
    }
}

pub fn play_note_audio(midi: u8) {
    AUDIO_STATE.with(|state| {
        let mut state = state.borrow_mut();

        let ctx = if let Some(ctx) = &state.ctx {
            let _ = ctx.resume();
            ctx.clone()
        } else {
            let ctx = web_sys::AudioContext::new().unwrap();
            state.ctx = Some(ctx.clone());
            ctx
        };

        let now = ctx.current_time();
        let freq = 440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0);

        // 1. Master Gain Node with per-note ADSR decay
        let master_gain = ctx.create_gain().unwrap();
        master_gain.gain().set_value_at_time(0.0, now).unwrap();

        // Percussive Attack (5ms fast rise)
        master_gain
            .gain()
            .linear_ramp_to_value_at_time(0.7, now + 0.005)
            .unwrap();
        // Natural exponential decay (simulates string vibration dying out over 3.5s)
        master_gain
            .gain()
            .exponential_ramp_to_value_at_time(0.0001, now + 3.5)
            .unwrap();

        // 2. Dynamic Low-Pass Filter (simulates hammer damping)
        let filter = ctx.create_biquad_filter().unwrap();
        filter.set_type(web_sys::BiquadFilterType::Lowpass);
        // Start bright (high cutoff) on impact, then close rapidly
        filter
            .frequency()
            .set_value_at_time(freq * 6.0, now)
            .unwrap();
        filter
            .frequency()
            .exponential_ramp_to_value_at_time(freq * 1.5, now + 1.2)
            .unwrap();

        filter.connect_with_audio_node(&master_gain).unwrap();
        master_gain
            .connect_with_audio_node(&ctx.destination())
            .unwrap();

        // 3. Additive Harmonics: Fundamental + Overtones (Ratio, Relative Gain)
        let harmonics = [
            (1.0, 1.00), // Fundamental (f)
            (2.0, 0.45), // Octave overtone (2f)
            (3.0, 0.20), // 12th overtone (3f)
            (4.0, 0.10), // 15th overtone (4f)
        ];

        let mut oscillators = Vec::new();

        for (multiplier, gain_level) in harmonics {
            let osc = ctx.create_oscillator().unwrap();
            let harm_gain = ctx.create_gain().unwrap();

            osc.set_type(web_sys::OscillatorType::Sine);
            osc.frequency().set_value(freq * multiplier);
            harm_gain.gain().set_value_at_time(gain_level, now).unwrap();

            osc.connect_with_audio_node(&harm_gain).unwrap();
            harm_gain.connect_with_audio_node(&filter).unwrap();
            osc.start().unwrap();

            oscillators.push(osc);
        }

        // Store active voice
        state.active_voices.insert(
            midi,
            NoteVoice {
                master_gain,
                oscillators,
            },
        );
    });
}

pub fn stop_note_audio(midi: u8) {
    AUDIO_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some(voice) = state.active_voices.remove(&midi) {
            if let Some(ctx) = &state.ctx {
                let now = ctx.current_time();
                // Smooth damper release (80ms fade out when key is released)
                let _ = voice.master_gain.gain().cancel_scheduled_values(now);
                let _ = voice
                    .master_gain
                    .gain()
                    .set_value_at_time(voice.master_gain.gain().value(), now);
                let _ = voice
                    .master_gain
                    .gain()
                    .linear_ramp_to_value_at_time(0.0001, now + 0.08);

                for osc in voice.oscillators {
                    let _ = osc.stop_with_when(now + 0.09);
                }
            }
        }
    });
}
