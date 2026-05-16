#!/usr/bin/env cargo run
//!
//! Example: Button Event Detection (Blocking Mode)
//!
//! This example demonstrates blocking wait for button press events using
//! GPIO edge detection, reducing CPU usage compared to continuous polling.
//!
//! # Hardware Setup
//!
//! Connect:
//! - Button to pin 18 and GND
//! - Pull-up resistor connecting the button to 3V3
//! - LED connected to pin 12
//!
//! # Usage
//!
//! ```bash
//! cargo run --example button_event
//! ```
//!
//! # Exit
//!
//! Press CTRL+C to exit

use jetsongpio::{Direction, GPIO, Level, Mode};
use jetsongpio::gpio_event::Edge;
use std::thread;
use std::time::Duration;

const LED_PIN: u32 = 12;  // Board pin 12
const BUTTON_PIN: u32 = 18; // Board pin 18

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create GPIO instance
    let mut gpio = GPIO::new();

    // Pin Setup:
    gpio.setmode(Mode::BOARD)?;
    gpio.setup(vec![LED_PIN], Direction::OUT, Some(Level::LOW), None)?;
    gpio.setup(vec![BUTTON_PIN], Direction::IN, None, None)?;

    // Initial state for LED:
    gpio.output(vec![LED_PIN], vec![Level::LOW])?;

    println!("Starting demo now! Press CTRL+C to exit");
    println!("Waiting for button event on pin {}", BUTTON_PIN);

    loop {
        // Wait for button press (falling edge)
        let detected = gpio.wait_for_edge(BUTTON_PIN, Edge::Falling, None)?;

        if detected {
            // Event received when button pressed
            println!("Button Pressed!");
            gpio.output(vec![LED_PIN], vec![Level::HIGH])?;
            thread::sleep(Duration::from_secs(1));
            gpio.output(vec![LED_PIN], vec![Level::LOW])?;
        }
    }
}