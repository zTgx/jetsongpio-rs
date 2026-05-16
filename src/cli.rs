use crate::gpio_pin_data::{get_jetson_data, get_model, GpioPin};
use clap::{Parser, Subcommand};
use std::fmt;

/// CLI tool for NVIDIA Jetson GPIO operations
#[derive(Parser)]
#[command(name = "jetsongpio")]
#[command(about = "A Rust CLI tool for controlling GPIO pins on NVIDIA Jetson devices", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Lookup pinmux register address for a GPIO pin
    PinmuxLookup {
        /// Board Mode GPIO pin number (e.g., 7, 11, 40, etc.)
        gpio_pin: u32,
    },
}

#[derive(Debug)]
pub enum CliError {
    PinNotFound(u32, String),
    ModelDetectionFailed(String),
}

impl fmt::Display for CliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CliError::PinNotFound(pin, model) => {
                write!(f, "GPIO pin {} not found in {} pin definitions", pin, model)
            }
            CliError::ModelDetectionFailed(e) => {
                write!(f, "Failed to detect Jetson model: {}", e)
            }
        }
    }
}

impl std::error::Error for CliError {}

fn lookup_mux_register(gpio_pin: u32, pin_defs: &[GpioPin]) -> Option<u32> {
    pin_defs
        .iter()
        .find(|pin| pin.board_pin == gpio_pin)
        .and_then(|pin| pin.padctl_addr)
}

fn run_pinmux_lookup(gpio_pin: u32) -> Result<(), CliError> {
    let model = get_model().map_err(CliError::ModelDetectionFailed)?;
    let (pin_defs, _jetson_info) = get_jetson_data(&model);

    let pin_register_address = lookup_mux_register(gpio_pin, &pin_defs)
        .ok_or_else(|| CliError::PinNotFound(gpio_pin, model.clone()))?;

    println!(
        "GPIO Pin {}: Mux Register Address = 0x{:X}",
        gpio_pin, pin_register_address
    );
    Ok(())
}

pub fn run() {
    let cli = Cli::parse();

    match cli.command {
        Commands::PinmuxLookup { gpio_pin } => {
            if let Err(e) = run_pinmux_lookup(gpio_pin) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}