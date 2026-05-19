#!/usr/bin/env cargo run
//!
//! Example: Multi-threaded GPIO Usage
//!
//! This example demonstrates sharing a single `GPIO` instance across multiple
//! threads using `Arc<GPIO>`. One thread blinks an LED while another thread
//! reads a button input. The `GPIO` type is `Send + Sync`, so it can be safely
//! shared.
//!
//! # Hardware Setup
//!
//! - LED (with resistor) on BOARD pin 12
//! - Button on BOARD pin 18 (with pull-up resistor to 3V3)
//!
//! # Usage
//!
//! ```bash
//! cargo run --example multithread
//! ```
//!
//! # Exit
//!
//! Press CTRL+C to exit

use jetsongpio::{Direction, GPIO, Level, Mode};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

const LED_PIN: u32 = 12;
const BUTTON_PIN: u32 = 18;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let running = Arc::new(AtomicBool::new(true));

    // Register CTRL+C handler to signal graceful shutdown
    {
        let r = Arc::clone(&running);
        ctrlc::set_handler(move || {
            println!("\nReceived CTRL+C, shutting down...");
            r.store(false, Ordering::SeqCst);
        })?;
    }

    let gpio = Arc::new(GPIO::new());
    gpio.setmode(Mode::BOARD)?;

    // Setup LED as output
    gpio.setup(vec![LED_PIN], Direction::OUT, Some(Level::LOW), None)?;
    // Setup button as input
    gpio.setup(vec![BUTTON_PIN], Direction::IN, None, None)?;

    println!("Starting multi-threaded demo! Press CTRL+C to exit");

    // Spawn a thread to blink the LED
    let gpio_led = Arc::clone(&gpio);
    let led_running = Arc::clone(&running);
    let led_handle = thread::spawn(move || {
        let mut high = true;
        while led_running.load(Ordering::SeqCst) {
            let level = if high { Level::HIGH } else { Level::LOW };
            if let Err(e) = gpio_led.output(vec![LED_PIN], vec![level]) {
                eprintln!("LED thread error: {}", e);
                break;
            }
            high = !high;
            // Use short sleeps so we respond promptly to shutdown
            for _ in 0..50 {
                if !led_running.load(Ordering::SeqCst) {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        }
        println!("LED thread exiting.");
    });

    // Spawn a thread to read the button
    let gpio_btn = Arc::clone(&gpio);
    let btn_running = Arc::clone(&running);
    let btn_handle = thread::spawn(move || {
        let mut last: Option<Level> = None;
        while btn_running.load(Ordering::SeqCst) {
            match gpio_btn.input(BUTTON_PIN) {
                Ok(level) => {
                    if last != Some(level) {
                        println!("Button: {:?}", level);
                        last = Some(level);
                    }
                }
                Err(e) => {
                    eprintln!("Button thread error: {}", e);
                    break;
                }
            }
            thread::sleep(Duration::from_millis(50));
        }
        println!("Button thread exiting.");
    });

    // Wait for both threads to finish
    led_handle.join().unwrap();
    btn_handle.join().unwrap();

    gpio.cleanup(None)?;
    println!("Cleanup done. Goodbye!");
    Ok(())
}
