#!/usr/bin/env cargo run
//!
//! Example: Button Interrupt with Callback (Non-Blocking Mode)
//!
//! This example demonstrates non-blocking event detection with callbacks.
//! When a button press is detected, the callback function is executed.
//!
//! # Hardware Setup
//!
//! Connect:
//! - Button to pin 18 and GND
//! - Pull-up resistor connecting the button to 3V3
//! - LED 1 connected to pin 12
//! - LED 2 connected to pin 13
//!
//! # Usage
//!
//! ```bash
//! cargo run --example button_interrupt
//! ```
//!
//! # Exit
//!
//! Press CTRL+C to exit

use jetsongpio::{Direction, GPIO, Level, Mode};
use jetsongpio::gpio_event::Edge;
use std::thread;
use std::time::Duration;

const LED1_PIN: u32 = 12;  // Board pin 12
const LED2_PIN: u32 = 13;  // Board pin 13
const BUTTON_PIN: u32 = 18; // Board pin 18

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create GPIO instance
    let mut gpio = GPIO::new();

    // Pin Setup:
    gpio.setmode(Mode::BOARD)?;
    gpio.setup(vec![LED1_PIN, LED2_PIN], Direction::OUT, Some(Level::LOW), None)?;
    gpio.setup(vec![BUTTON_PIN], Direction::IN, None, None)?;

    // Initial state for LEDs:
    gpio.output(vec![LED1_PIN, LED2_PIN], vec![Level::LOW, Level::LOW])?;

    // Add event detection for button press (falling edge) with callback
    gpio.add_event_detect(
        BUTTON_PIN,
        Edge::Falling,
        Box::new(|| {
            println!("Button pressed! Blink LED2");
            for _ in 0..5 {
                println!("LED2 HIGH");
                thread::sleep(Duration::from_millis(500));
                println!("LED2 LOW");
                thread::sleep(Duration::from_millis(500));
            }
        }),
        Some(Duration::from_millis(200)), // 200ms debounce
    )?;

    println!("Starting demo now! Press CTRL+C to exit");

    // Main loop - blink LED1 slowly while button interrupts work independently
    loop {
        gpio.output(vec![LED1_PIN], vec![Level::HIGH])?;
        thread::sleep(Duration::from_secs(2));
        gpio.output(vec![LED1_PIN], vec![Level::LOW])?;
        thread::sleep(Duration::from_secs(2));
    }
}