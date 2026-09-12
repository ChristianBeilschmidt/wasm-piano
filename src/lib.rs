use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AudioContext, CanvasRenderingContext2d, HtmlCanvasElement};
use yew::prelude::*;

// 1. Core Audio & Pitch Calculations
fn get_frequency(note: &str) -> f32 {
    match note {
        "C2" => 65.41,
        "C#2" => 69.30,
        "D2" => 73.42,
        "D#2" => 77.78,
        "E2" => 82.41,
        "F2" => 87.31,
        "F#2" => 92.50,
        "G2" => 98.00,
        "G#2" => 103.83,
        "A2" => 110.00,
        "A#2" => 116.54,
        "B2" => 123.47,
        "C3" => 130.81,
        "C#3" => 138.59,
        "D3" => 146.83,
        "D#3" => 155.56,
        "E3" => 164.81,
        "F3" => 174.61,
        "F#3" => 185.00,
        "G3" => 196.00,
        "G#3" => 207.65,
        "A3" => 220.00,
        "A#3" => 233.08,
        "B3" => 246.94,
        "C4" => 261.63,
        _ => 440.0,
    }
}

fn note_to_staff_step(note: &str) -> i32 {
    let base_note = if note.len() == 3 {
        format!("{}{}", &note[0..1], &note[2..3])
    } else {
        note.to_string()
    };
    match base_note.as_str() {
        "C2" => -8,
        "D2" => -7,
        "E2" => -6,
        "F2" => -5,
        "G2" => -4,
        "A2" => -3,
        "B2" => -2,
        "C3" => -1,
        "D3" => 0,
        "E3" => 1,
        "F3" => 2,
        "G3" => 3,
        "A3" => 4,
        "B3" => 5,
        "C4" => 6,
        _ => 0,
    }
}

fn check_accidental(note: &str) -> bool {
    note.contains('#')
}

// 2. Pure Canvas Rendering Pipeline
fn draw_staff(context: &CanvasRenderingContext2d, last_note: Option<&str>) {
    context.clear_rect(0.0, 0.0, 800.0, 300.0);
    let line_spacing = 20.0;
    let center_y = 150.0; // D3 Middle Line

    context.set_stroke_style_str("#333333");
    context.set_line_width(2.0);

    for i in -2..=2 {
        let y = center_y - (i as f64 * line_spacing);
        context.begin_path();
        context.move_to(50.0, y);
        context.line_to(750.0, y);
        context.stroke();
    }

    context.set_fill_style_str("#111111");
    context.set_font("65px Arial");
    let _ = context.fill_text("𝄢", 70.0, center_y - 10.0);

    if let Some(note) = last_note {
        let step = note_to_staff_step(note);
        let note_y = center_y - (step as f64 * (line_spacing / 2.0));
        let note_x = 400.0;

        context.set_stroke_style_str("#222222");
        if step <= -6 {
            for s in (-8..=-6).step_by(2) {
                if step <= s {
                    let ledger_y = center_y - (s as f64 * (line_spacing / 2.0));
                    context.begin_path();
                    context.move_to(note_x - 20.0, ledger_y);
                    context.line_to(note_x + 20.0, ledger_y);
                    context.stroke();
                }
            }
        } else if step >= 6 {
            for s in (6..=6).step_by(2) {
                if step >= s {
                    let ledger_y = center_y - (s as f64 * (line_spacing / 2.0));
                    context.begin_path();
                    context.move_to(note_x - 20.0, ledger_y);
                    context.line_to(note_x + 20.0, ledger_y);
                    context.stroke();
                }
            }
        }

        if check_accidental(note) {
            context.set_font("30px Arial");
            let _ = context.fill_text("#", note_x - 35.0, note_y + 10.0);
        }

        context.begin_path();
        let _ = context.ellipse(
            note_x,
            note_y,
            13.0,
            9.0,
            0.0,
            0.0,
            2.0 * std::f64::consts::PI,
        );
        context.fill();

        context.set_font("bold 20px Arial");
        let _ = context.fill_text(note, note_x - 15.0, 270.0);
    }
}

fn play_tone(audio_ctx: &AudioContext, frequency: f32) {
    let osc = audio_ctx.create_oscillator().unwrap();
    let gain = audio_ctx.create_gain().unwrap();

    osc.set_type(web_sys::OscillatorType::Sine);
    osc.frequency().set_value(frequency);

    gain.gain().set_value(0.3);
    let current_time = audio_ctx.current_time();
    let _ = gain
        .gain()
        .exponential_ramp_to_value_at_time(0.0001, current_time + 0.6);

    let _ = osc.connect_with_audio_node(&gain);
    let _ = gain.connect_with_audio_node(&audio_ctx.destination());

    let _ = osc.start();
    let _ = osc.stop_with_when(current_time + 0.6);
}

// 3. Declarative Yew Application Component
#[function_component(App)]
fn app() -> Html {
    let canvas_ref = use_node_ref();
    let last_note = use_state(|| Option::<String>::None);

    // Persist a single AudioContext safely across renders without leaking threads
    let audio_ctx = use_memo((), |_| AudioContext::new().unwrap());

    // Side Effect: Runs automatically whenever `last_note` mutations occur
    {
        let canvas_ref = canvas_ref.clone();
        let last_note = last_note.clone();
        use_effect_with(last_note, move |current_note| {
            if let Some(canvas) = canvas_ref.cast::<HtmlCanvasElement>() {
                let context = canvas
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<CanvasRenderingContext2d>()
                    .unwrap();
                draw_staff(&context, current_note.as_deref());
            }
        });
    }

    // Interactive Key Click Event Handler
    let on_key_down = {
        let last_note = last_note.clone();
        let audio_ctx = audio_ctx.clone();
        Callback::from(move |note: String| {
            if audio_ctx.state() == web_sys::AudioContextState::Suspended {
                let _ = audio_ctx.resume();
            }
            play_tone(&audio_ctx, get_frequency(&note));
            last_note.set(Some(note));
        })
    };

    // Vector mapping layout architecture for the rendering loop
    let piano_keys = vec![
        ("C2", "white", ""),
        ("C#2", "black", "left: 5.5%;"),
        ("D2", "white", ""),
        ("D#2", "black", "left: 12.5%;"),
        ("E2", "white", ""),
        ("F2", "white", ""),
        ("F#2", "black", "left: 26.5%;"),
        ("G2", "white", ""),
        ("G#2", "black", "left: 33.5%;"),
        ("A2", "white", ""),
        ("A#2", "black", "left: 40.5%;"),
        ("B2", "white", ""),
        ("C3", "white", ""),
        ("C#3", "black", "left: 54.5%;"),
        ("D3", "white", ""),
        ("D#3", "black", "left: 61.5%;"),
        ("E3", "white", ""),
        ("F3", "white", ""),
        ("F#3", "black", "left: 75.5%;"),
        ("G3", "white", ""),
        ("G#3", "black", "left: 82.5%;"),
        ("A3", "white", ""),
        ("A#3", "black", "left: 89.5%;"),
        ("B3", "white", ""),
        ("C4", "white", ""),
    ];

    html! {
            <>
                <div id="top-half">
                    <canvas ref={canvas_ref} id="staff-canvas" width="800" height="300"></canvas>
                </div>
                <div id="bottom-half">
                    <div class="piano">
                        {
        piano_keys.into_iter().map(|(note, key_class, style_offset)| {
            let on_click = on_key_down.clone();
            let note_string = note.to_string();

            // Only apply the style attribute if an offset string exists (prevents empty style="")
            let inline_style = if style_offset.is_empty() { None } else { Some(style_offset) };

            html! {
                <div
                    key={note_string.clone()} // Fix 1: Unique key identifier for Yew's tracking engine
                    class={format!("key {}", key_class)}
                    style={inline_style}       // Fix 2: Clean style handling
                    onclick={move |_| on_click.emit(note_string.clone())}
                >
                </div>                     // Fix 3: Explicit opening and closing tags
            }
        }).collect::<Html>()
    }
                    </div>
                </div>
            </>
        }
}

#[wasm_bindgen(start)]
pub fn run() {
    yew::Renderer::<App>::new().render();
}
