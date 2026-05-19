#!/usr/bin/env cargo run
//!
//! Example: Simple GPIO Input
//!
//! This example demonstrates reading input from a GPIO pin.
//! It prints the value only when it changes from LOW to HIGH or HIGH to LOW.
//!
//! # Hardware Setup
//!
//! Connect a button or switch to pin 18 (BOARD mode) and GND.
//! You may also want to add a pull-up resistor.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example simple_input
//! ```
//!
//! # Exit
//!
//! Press CTRL+C to exit

use jetsongpio::{Direction, GPIO, Level, Mode};
use std::thread;
use std::time::Duration;

const INPUT_PIN: u32 = 18; // BOARD pin 18

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut prev_value: Option<Level> = None;

    // Create GPIO instance
    let gpio = GPIO::new();

    // Pin Setup:
    gpio.setmode(Mode::BOARD)?; // BOARD pin-numbering scheme
    gpio.setup(vec![INPUT_PIN], Direction::IN, None, None)?; // set pin as an input pin
    println!("Starting demo now! Press CTRL+C to exit");

    loop {
        let value = gpio.input(INPUT_PIN)?;
        if prev_value.is_none() || value != prev_value.unwrap() {
            let value_str = match value {
                Level::HIGH => "HIGH",
                Level::LOW => "LOW",
            };
            println!("Value read from pin {} : {}", INPUT_PIN, value_str);
            prev_value = Some(value);
        }
        thread::sleep(Duration::from_secs(1));
    }
}
