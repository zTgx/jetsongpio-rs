#!/usr/bin/env cargo run
//!
//! Example: Hardware PWM (breathing LED)
//!
//! Demonstrates hardware PWM control on Jetson devices. The duty cycle sweeps
//! from 0% to 100% and back, creating a breathing LED effect.
//!
//! # PWM Pin Mapping (BOARD mode)
//!
//! - JETSON_XAVIER / CLARA_AGX_XAVIER / JETSON_ORIN: 18
//! - JETSON_NANO / JETSON_NX / JETSON_ORIN_NX / JETSON_ORIN_NANO: 33
//! - JETSON_TX2_NX: 32
//!
//! # Usage
//!
//! ```bash
//! cargo run --example simple_pwm
//! ```

use jetsongpio::{GPIO, Mode, PWM, get_model};

/// Map model name to its PWM-capable BOARD pin number.
fn pwm_board_pin(model: &str) -> Option<u32> {
    match model {
        "JETSON_XAVIER" | "CLARA_AGX_XAVIER" | "JETSON_ORIN" => Some(18),
        "JETSON_NANO" | "JETSON_NX" | "JETSON_ORIN_NX" | "JETSON_ORIN_NANO" => Some(33),
        "JETSON_TX2_NX" => Some(32),
        _ => None,
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = get_model()?;
    println!("Detected board: {}", model);

    let board_pin = match pwm_board_pin(&model) {
        Some(p) => p,
        None => {
            eprintln!("No PWM pin defined for board: {}", model);
            return Ok(());
        }
    };

    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;

    println!("Creating PWM on BOARD pin {} at 50 Hz...", board_pin);
    let mut pwm = PWM::new(&mut gpio, board_pin, 50.0)?;

    let mut val: f64 = 25.0;
    let incr: f64 = 5.0;
    pwm.start(val)?;

    println!("PWM running. Press Ctrl+C to exit.");
    loop {
        std::thread::sleep(std::time::Duration::from_millis(250));
        if val >= 100.0 {
            val -= incr;
        } else if val <= 0.0 {
            val += incr;
        } else {
            val += incr;
        }
        val = val.clamp(0.0, 100.0);
        pwm.set_duty_cycle(val)?;
        println!("duty_cycle: {:.1}%", val);
    }
}
