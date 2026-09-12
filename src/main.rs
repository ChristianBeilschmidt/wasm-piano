use leptos::mount::mount_to_body;
use leptos::prelude::*;
use std::cell::RefCell;
use std::collections::HashMap;

// Global thread-local state for managing active Web Audio Context nodes
thread_local! {
    static AUDIO_STATE: RefCell<AudioState> = RefCell::new(AudioState::new());
}

struct AudioState {
    ctx: Option<web_sys::AudioContext>,
    oscillators: HashMap<u8, (web_sys::OscillatorNode, web_sys::GainNode)>,
}

impl AudioState {
    fn new() -> Self {
        Self {
            ctx: None,
            oscillators: HashMap::new(),
        }
    }
}

// -----------------------------------------------------------------------------
// Web Audio API Synthesis
// -----------------------------------------------------------------------------

fn play_note_audio(midi: u8) {
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

        // Convert MIDI note number to frequency (Hz)
        let freq = 440.0 * 2.0_f32.powf((midi as f32 - 69.0) / 12.0);

        let osc = ctx.create_oscillator().unwrap();
        let gain = ctx.create_gain().unwrap();

        osc.set_type(web_sys::OscillatorType::Triangle);
        osc.frequency().set_value(freq);

        let now = ctx.current_time();
        gain.gain().set_value_at_time(0.0, now).unwrap();
        gain.gain()
            .linear_ramp_to_value_at_time(0.4, now + 0.02)
            .unwrap();

        osc.connect_with_audio_node(&gain).unwrap();
        gain.connect_with_audio_node(&ctx.destination()).unwrap();
        osc.start().unwrap();

        state.oscillators.insert(midi, (osc, gain));
    });
}

fn stop_note_audio(midi: u8) {
    AUDIO_STATE.with(|state| {
        let mut state = state.borrow_mut();
        if let Some((osc, gain)) = state.oscillators.remove(&midi) {
            if let Some(ctx) = &state.ctx {
                let now = ctx.current_time();
                let _ = gain.gain().cancel_scheduled_values(now);
                let _ = gain.gain().set_value_at_time(gain.gain().value(), now);
                let _ = gain.gain().linear_ramp_to_value_at_time(0.0, now + 0.08);
                let _ = osc.stop_with_when(now + 0.08);
            }
        }
    });
}

// -----------------------------------------------------------------------------
// Music Theory & Diatonic Mapping Helpers
// -----------------------------------------------------------------------------

struct KeyDef {
    midi: u8,
    is_black: bool,
    white_idx: usize,
}

/// Generates piano key data from C2 (MIDI 36) to C4 / Middle C (MIDI 60)
fn generate_keys() -> Vec<KeyDef> {
    let mut keys = Vec::new();
    let mut current_white = 0;

    for midi in 36..=60 {
        let pitch_class = midi % 12;
        let is_black = matches!(pitch_class, 1 | 3 | 6 | 8 | 10);

        keys.push(KeyDef {
            midi,
            is_black,
            white_idx: if is_black {
                current_white - 1
            } else {
                current_white
            },
        });

        if !is_black {
            current_white += 1;
        }
    }
    keys
}

/// Maps a MIDI note to diatonic staff steps relative to C2 (0) and detects accidentals
fn midi_to_step(midi: u8) -> (i32, bool) {
    let note_idx = midi as i32 - 36; // C2 is index 0
    let octave = note_idx / 12;
    let pitch_class = note_idx % 12;

    let (step_offset, is_sharp) = match pitch_class {
        0 => (0, false),  // C
        1 => (0, true),   // C#
        2 => (1, false),  // D
        3 => (1, true),   // D#
        4 => (2, false),  // E
        5 => (3, false),  // F
        6 => (3, true),   // F#
        7 => (4, false),  // G
        8 => (4, true),   // G#
        9 => (5, false),  // A
        10 => (5, true),  // A#
        11 => (6, false), // B
        _ => (0, false),
    };

    let step = octave * 7 + step_offset;
    (step, is_sharp)
}

// -----------------------------------------------------------------------------
// Leptos 0.8 UI Components
// -----------------------------------------------------------------------------

#[component]
fn Staff(active_note: ReadSignal<Option<u8>>) -> impl IntoView {
    view! {
        <div class="top-pane">
            <span class="pane-label">"Bass Clef Visualization"</span>
            <svg width="600" height="200" viewBox="0 0 600 200">
                // Bass Clef Staff Lines: G2 (140), B2 (120), D3 (100), F3 (80), A3 (60)
                <line x1="50" y1="60" x2="550" y2="60" stroke="#222" stroke-width="2"/>
                <line x1="50" y1="80" x2="550" y2="80" stroke="#222" stroke-width="2"/>
                <line x1="50" y1="100" x2="550" y2="100" stroke="#222" stroke-width="2"/>
                <line x1="50" y1="120" x2="550" y2="120" stroke="#222" stroke-width="2"/>
                <line x1="50" y1="140" x2="550" y2="140" stroke="#222" stroke-width="2"/>

                // Bass Clef Symbol (Unicode 𝄢)
                <text x="60" y="118" font-size="68" font-family="serif" fill="#222">"𝄢"</text>

                // Dynamically Render Played Note
                {move || {
                    active_note.get().map(|midi| {
                        let (step, is_sharp) = midi_to_step(midi);

                        // D3 (MIDI 50, Step 8) rests on the middle staff line at Y=100.
                        // Each step corresponds to 10px.
                        let y_pos = 100 - (step - 8) * 10;

                        // Calculate required ledger lines for notes outside the 5-line staff
                        let mut ledger_lines = Vec::new();
                        if y_pos <= 40 {
                            let mut l_y = 40;
                            while l_y >= y_pos {
                                ledger_lines.push(l_y);
                                l_y -= 20;
                            }
                        }
                        if y_pos >= 160 {
                            let mut l_y = 160;
                            while l_y <= y_pos {
                                ledger_lines.push(l_y);
                                l_y += 20;
                            }
                        }

                        view! {
                            <g class="note">
                                {ledger_lines.into_iter().map(|l_y| view! {
                                    <line x1="280" y1=l_y x2="320" y2=l_y stroke="#222" stroke-width="2"/>
                                }).collect_view()}

                                // Rotating the ellipse gives an authentic musical note head appearance
                                <ellipse
                                    cx="300"
                                    cy=y_pos
                                    rx="11"
                                    ry="8"
                                    fill="#2563eb"
                                    transform=format!("rotate(-15 300 {})", y_pos)
                                />

                                {if is_sharp {
                                    view! {
                                        <text x="268" y=y_pos + 7 font-size="24" font-weight="bold" fill="#2563eb">
                                            "♯"
                                        </text>
                                    }.into_any()
                                } else {
                                    view! { <></> }.into_any()
                                }}
                            </g>
                        }
                    })
                }}
            </svg>
        </div>
    }
}

#[component]
fn Keyboard(set_active_note: WriteSignal<Option<u8>>) -> impl IntoView {
    let keys = generate_keys();

    view! {
        <div class="bottom-pane">
            <span class="pane-label">"Interactive Piano Keyboard (C2 – C4)"</span>
            <div class="keyboard">
                {keys.into_iter().map(|k| {
                    let midi = k.midi;
                    let is_black = k.is_black;
                    let class = if is_black { "key black-key" } else { "key white-key" };
                    let style = if is_black {
                        format!("left: {}px;", k.white_idx * 40 + 40 - 12)
                    } else {
                        String::new()
                    };

                    let on_down = move |_| {
                        set_active_note.set(Some(midi));
                        play_note_audio(midi);
                    };

                    let on_up = move |_| {
                        set_active_note.set(None);
                        stop_note_audio(midi);
                    };

                    view! {
                        <div
                            class=class
                            style=style
                            on:pointerdown=on_down
                            on:pointerup=on_up
                            on:pointerleave=on_up
                        ></div>
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
pub fn App() -> impl IntoView {
    let (active_note, set_active_note) = signal(None::<u8>);

    view! {
        <div class="app-container">
            <Staff active_note=active_note />
            <Keyboard set_active_note=set_active_note />
        </div>
    }
}

fn main() {
    mount_to_body(App);
}
