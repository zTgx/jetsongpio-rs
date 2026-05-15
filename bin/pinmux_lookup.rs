//! Pinmux Lookup Utility
//!
//! This is a CLI tool to lookup pinmux register addresses for GPIO pins.
//!
//! Usage:
//! ```bash
//! cargo run --bin pinmux_lookup <gpio_pin_number>
//! ```
//!
//! Specify the Board Mode GPIO pin number (e.g., 7, 11, 40, etc.)

use std::env;
use jetsongpio::gpio_pin_data::{get_jetson_data, get_model, GpioPin};

fn lookup_mux_register(gpio_pin: u32, pin_defs: &[GpioPin]) -> Option<u32> {
    pin_defs
        .iter()
        .find(|pin| pin.board_pin == gpio_pin)
        .and_then(|pin| pin.padctl_addr)
}

fn print_usage() {
    eprintln!("Usage: cargo run --bin pinmux_lookup <gpio_pin_number>");
    eprintln!();
    eprintln!("Lookup pinmux register address for GPIO pins.");
    eprintln!("Specify the Board Mode GPIO pin number (e.g., 7, 11, 40, etc.)");
    eprintln!();
    eprintln!("Example:");
    eprintln!("  cargo run --bin pinmux_lookup 7");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        print_usage();
        std::process::exit(1);
    }

    let gpio_pin = match args[1].parse::<u32>() {
        Ok(pin) => pin,
        Err(_) => {
            eprintln!(
                "Error: GPIO pin number must be an integer, got '{}'",
                args[1]
            );
            std::process::exit(1);
        }
    };

    // Get the current Jetson model
    let model = match get_model() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("Error: Failed to detect Jetson model: {}", e);
            std::process::exit(1);
        }
    };

    let (pin_defs, _jetson_info) = get_jetson_data(&model);

    // Get pin register address
    let pin_register_address = match lookup_mux_register(gpio_pin, &pin_defs) {
        Some(addr) => addr,
        None => {
            eprintln!(
                "Error: GPIO pin {} not found in {} pin definitions",
                gpio_pin, model
            );
            std::process::exit(1);
        }
    };

    println!("GPIO Pin {}: Mux Register Address = 0x{:X}", gpio_pin, pin_register_address);
}