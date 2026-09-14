use std::cell::RefCell;
use std::collections::HashMap;

thread_local! {
    static AUDIO_STATE: RefCell<AudioState> = RefCell::new(AudioState::new());
}

struct NoteVoice {
    master_gain: web_sys::GainNode,
    filter: web_sys::BiquadFilterNode,
    oscillators: Vec<web_sys::OscillatorNode>,
    harmonic_gains: Vec<web_sys::GainNode>,
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

    fn ensure_context(&mut self) -> web_sys::AudioContext {
        if let Some(ctx) = &self.ctx {
            let _ = ctx.resume();
            return ctx.clone();
        }

        let ctx = web_sys::AudioContext::new().unwrap();
        let _ = ctx.resume();
        self.ctx = Some(ctx.clone());
        ctx
    }

    fn release_voice_for_note(&mut self, midi: u8) {
        let Some(voice) = self.active_voices.remove(&midi) else {
            return;
        };

        let NoteVoice {
            master_gain,
            filter,
            mut oscillators,
            mut harmonic_gains,
        } = voice;

        if let Some(ctx) = &self.ctx {
            let now = ctx.current_time();

            // Firefox is much more sensitive to abrupt scheduled-value cancellation.
            // Instead of resetting the envelope mid-flight, fade continuously to silence.
            let _ = master_gain
                .gain()
                .set_value_at_time(master_gain.gain().value(), now);
            let _ = master_gain
                .gain()
                .linear_ramp_to_value_at_time(0.0001, now + 0.18);

            while let Some(osc) = oscillators.pop() {
                if let Some(harm_gain) = harmonic_gains.pop() {
                    let _ = harm_gain
                        .gain()
                        .set_value_at_time(harm_gain.gain().value(), now);
                    let _ = harm_gain
                        .gain()
                        .linear_ramp_to_value_at_time(0.0001, now + 0.18);
                }

                let _ = osc.stop_with_when(now + 0.22);
            }

            let _ = filter;
            let _ = master_gain;
        }
    }
}

fn midi_to_frequency(midi: u8) -> f32 {
    440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0)
}

pub fn play_note_audio(midi: u8) {
    AUDIO_STATE.with(|state| {
        let mut state = state.borrow_mut();

        let ctx = state.ensure_context();

        if state.active_voices.contains_key(&midi) {
            let _ = state.release_voice_for_note(midi);
        }

        let now = ctx.current_time();
        let freq = midi_to_frequency(midi);

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
        let mut harmonic_gains = Vec::new();

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
            harmonic_gains.push(harm_gain);
        }

        // Store active voice
        state.active_voices.insert(
            midi,
            NoteVoice {
                master_gain,
                filter,
                oscillators,
                harmonic_gains,
            },
        );
    });
}

pub fn stop_note_audio(midi: u8) {
    AUDIO_STATE.with(|state| {
        let mut state = state.borrow_mut();
        let _ = state.release_voice_for_note(midi);
    });
}

#[cfg(test)]
mod tests {
    use super::midi_to_frequency;

    #[test]
    fn midi_pitch_frequency_checks_are_valid() {
        let c4 = 261.625_565_f32;
        let c5 = 523.251_13_f32;
        let a4 = 440.0_f32;
        let a5 = 880.0_f32;

        let freq_for_c4 = midi_to_frequency(60);
        let freq_for_c5 = midi_to_frequency(72);
        let freq_for_a4 = midi_to_frequency(69);
        let freq_for_a5 = midi_to_frequency(81);

        assert!((freq_for_c4 - c4).abs() < 0.01, "C4 should be ~261.63Hz");
        assert!((freq_for_c5 - c5).abs() < 0.01, "C5 should be ~523.25Hz");
        assert!((freq_for_a4 - a4).abs() < 0.01, "A4 should be ~440.00Hz");
        assert!((freq_for_a5 - a5).abs() < 0.01, "A5 should be ~880.00Hz");
    }
}
