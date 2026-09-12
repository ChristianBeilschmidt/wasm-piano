use leptos::mount::mount_to_body;
use leptos::prelude::*;

use crate::audio::{play_note_audio, stop_note_audio};

mod audio;

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

/// Maps a MIDI note to diatonic staff steps relative to C2 (0) and detects accidentals.
/// The bass staff is anchored on G2 at the bottom line and A3 at the top line, so each
/// diatonic step changes vertical position by 10px.
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

#[component]
fn Staff(active_note: ReadSignal<Option<u8>>) -> impl IntoView {
    let is_active = move || active_note.get().is_some();

    let note_data = move || {
        active_note.get().map(|midi| {
            let (step, is_sharp) = midi_to_step(midi);
            let y = 100 - (step - 8) * 10;
            (y, is_sharp)
        })
    };

    let note_y = move || note_data().map(|(y, _)| y).unwrap_or(100);
    let note_sharp = move || note_data().map(|(_, sharp)| sharp).unwrap_or(false);

    // Dynamic visibility for ledger lines
    let ledger_40_vis = move || {
        if is_active() && note_y() <= 40 {
            "visible"
        } else {
            "hidden"
        }
    };
    let ledger_160_vis = move || {
        if is_active() && note_y() >= 160 {
            "visible"
        } else {
            "hidden"
        }
    };
    let ledger_180_vis = move || {
        if is_active() && note_y() >= 180 {
            "visible"
        } else {
            "hidden"
        }
    };

    let note_vis = move || if is_active() { "visible" } else { "hidden" };
    let sharp_vis = move || {
        if is_active() && note_sharp() {
            "visible"
        } else {
            "hidden"
        }
    };

    // FIX 2: Use standard CSS transform properties routed through the CSS compositor
    let note_style = move || {
        format!(
            "transform: translate(300px, {}px) rotate(-15deg); transform-box: fill-box; transform-origin: center;",
            note_y()
        )
    };

    view! {
        <div class="top-pane">
            <span class="pane-label">"Bass Clef Visualization"</span>
            <svg
                width="600"
                height="200"
                viewBox="0 0 600 200"
                // FIX 1: Containment prevents Firefox WebRender from slicing SVG into dirty-rect tiles
                style="isolation: isolate; contain: paint;"
            >
                // FIX 3: Isolated group for static background elements
                <g style="isolation: isolate;">
                    <line x1="50" y1="60" x2="550" y2="60" stroke="#222" stroke-width="2"/>
                    <line x1="50" y1="80" x2="550" y2="80" stroke="#222" stroke-width="2"/>
                    <line x1="50" y1="100" x2="550" y2="100" stroke="#222" stroke-width="2"/>
                    <line x1="50" y1="120" x2="550" y2="120" stroke="#222" stroke-width="2"/>
                    <line x1="50" y1="140" x2="550" y2="140" stroke="#222" stroke-width="2"/>
                    <text x="60" y="118" font-size="68" font-family="serif" fill="#222" pointer-events="none">"𝄢"</text>
                </g>

                // Static ledger lines with signal-driven visibility
                <g style="isolation: isolate;">
                    <line x1="280" y1="40" x2="320" y2="40" stroke="#222" stroke-width="2" visibility=ledger_40_vis/>
                    <line x1="280" y1="160" x2="320" y2="160" stroke="#222" stroke-width="2" visibility=ledger_160_vis/>
                    <line x1="280" y1="180" x2="320" y2="180" stroke="#222" stroke-width="2" visibility=ledger_180_vis/>
                </g>

                // Dynamic notehead rendered at origin (0,0) and positioned via CSS transform
                <g style=note_style visibility=note_vis>
                    <ellipse cx="0" cy="0" rx="11" ry="8" fill="#2563eb" />
                </g>

                // Accidental indicator
                <text
                    x="268"
                    y=move || note_y() + 7
                    font-size="24"
                    font-weight="bold"
                    fill="#2563eb"
                    visibility=sharp_vis
                >
                    "♯"
                </text>
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
