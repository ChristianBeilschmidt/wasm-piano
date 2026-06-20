use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{AudioContext, CanvasRenderingContext2d, HtmlCanvasElement, HtmlElement};

// Mapping frequencies for standard scientific pitch notation (Bass Range)
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

// Map the musical pitch to staff drawing steps relative to the center line (D3 = Step 0)
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

fn draw_staff(context: &CanvasRenderingContext2d, last_note: Option<&str>) {
    // Clear canvas
    context.clear_rect(0.0, 0.0, 800.0, 300.0);

    let line_spacing = 20.0;
    let center_y = 150.0; // Corresponds to D3 (middle line of bass clef)

    context.set_stroke_style_str("#333333");
    context.set_line_width(2.0);

    // Draw the 5 main lines of the Bass Clef staff (G2, B2, D3, F3, A3)
    for i in -2..=2 {
        let y = center_y - (i as i32 * line_spacing as i32) as f64;
        context.begin_path();
        context.move_to(50.0, y);
        context.line_to(750.0, y);
        context.stroke();
    }

    // Render Unicode Bass Clef glyph (𝄢) cleanly on the 4th line (F3)
    context.set_fill_style_str("#111111");
    context.set_font("65px Arial");
    let _ = context.fill_text("𝄢", 70.0, center_y - 10.0);

    // If a note has been played, paint it on the canvas
    if let Some(note) = last_note {
        let step = note_to_staff_step(note);
        let note_y = center_y - (step as f64 * (line_spacing / 2.0));
        let note_x = 400.0;

        // Render Ledger Lines for notes outside the core 5 staff lines (e.g. Low C2 or Middle C4)
        context.set_stroke_style_str("#222222");
        if step <= -6 {
            // Below the staff lines (E2 and lower)
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
            // Above staff lines (Middle C4)
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

        // Draw Accidental symbol (Sharp #) if needed
        if check_accidental(note) {
            context.set_font("30px Arial");
            let _ = context.fill_text("#", note_x - 35.0, note_y + 10.0);
        }

        // Draw standard oval note head
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

        // Draw note name string text below for visual aid
        context.set_font("bold 20px Arial");
        let _ = context.fill_text(note, note_x - 15.0, 270.0);
    }
}

fn play_tone(audio_ctx: &AudioContext, frequency: f32) {
    let osc = audio_ctx.create_oscillator().unwrap();
    let gain = audio_ctx.create_gain().unwrap();

    osc.set_type(web_sys::OscillatorType::Sine);
    osc.frequency().set_value(frequency);

    // Short musical envelope setting to prevent audio popping
    gain.gain().set_value(0.3);
    let current_time = audio_ctx.current_time();
    gain.gain()
        .exponential_ramp_to_value_at_time(0.0001, current_time + 0.6)
        .unwrap();

    let _ = osc.connect_with_audio_node(&gain);
    let _ = gain.connect_with_audio_node(&audio_ctx.destination());

    let _ = osc.start();
    let _ = osc.stop_with_when(current_time + 0.6);
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    let window = web_sys::window().expect("No global window found");
    let document = window.document().expect("No document found");

    // Setup Canvas and context bindings
    let canvas = document
        .get_element_by_id("staff-canvas")
        .expect("Canvas element missing")
        .dyn_into::<HtmlCanvasElement>()?;
    let context = canvas
        .get_context("2d")?
        .expect("Could not obtain 2D context")
        .dyn_into::<CanvasRenderingContext2d>()?;

    // Initial pristine layout render
    draw_staff(&context, None);

    // Instantiate Audio Context
    let audio_ctx = AudioContext::new()?;

    // Fetch all musical keys defined within our DOM layout
    let keys = document.get_elements_by_class_name("key");

    for i in 0..keys.length() {
        let key = keys.item(i).unwrap().dyn_into::<HtmlElement>()?;
        let note = key
            .get_attribute("data-note")
            .unwrap_or_else(|| "C3".to_string());

        let context_clone = context.clone();
        let audio_ctx_clone = audio_ctx.clone();
        let note_clone = note.clone();

        let closure = Closure::wrap(Box::new(move || {
            // Wake AudioContext up if suspended by browser security policies
            if audio_ctx_clone.state() == web_sys::AudioContextState::Suspended {
                let _ = audio_ctx_clone.resume();
            }

            let freq = get_frequency(&note_clone);
            play_tone(&audio_ctx_clone, freq);
            draw_staff(&context_clone, Some(&note_clone));
        }) as Box<dyn FnMut()>);

        key.set_onclick(Some(closure.as_ref().unchecked_ref()));
        closure.forget();
    }

    Ok(())
}
