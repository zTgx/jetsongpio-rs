#!/usr/bin/env cargo run
//!
//! Example: Simple GPIO Output
//!
//! This example demonstrates writing output to a GPIO pin.
//! It toggles the output between HIGH and LOW every second.
//!
//! # Hardware Setup
//!
//! Connect an LED (with appropriate resistor) to pin 18 (BOARD mode) and GND.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example simple_out
//! ```
//!
//! # Exit
//!
//! Press CTRL+C to exit

use jetsongpio::{Direction, GPIO, Level, Mode};
use std::thread;
use std::time::Duration;

const OUTPUT_PIN: u32 = 18; // BOARD pin 18

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create GPIO instance
    let mut gpio = GPIO::new();

    // Pin Setup:
    gpio.setmode(Mode::BOARD)?; // BOARD pin-numbering scheme
    // set pin as an output pin with optional initial state of HIGH
    gpio.setup(vec![OUTPUT_PIN], Direction::OUT, Some(Level::HIGH), None)?;

    println!("Starting demo now! Press CTRL+C to exit");
    let mut curr_value = Level::HIGH;

    loop {
        thread::sleep(Duration::from_secs(1));
        // Toggle the output every second
        println!("Outputting {:?} to pin {}", curr_value, OUTPUT_PIN);
        gpio.output(vec![OUTPUT_PIN], vec![curr_value])?;
        curr_value = match curr_value {
            Level::HIGH => Level::LOW,
            Level::LOW => Level::HIGH,
        };
    }
}
