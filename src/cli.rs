use crate::gpio::{Direction, GPIO, Level};
use crate::gpio_pin_data::{GpioPin, Mode, get_jetson_data, get_model};
use clap::{Parser, Subcommand, ValueEnum};
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
    /// Quick set pin to HIGH (automatically setup as OUT)
    High {
        /// GPIO pin number
        pin: u32,
    },
    /// Quick set pin to LOW (automatically setup as OUT)
    Low {
        /// GPIO pin number
        pin: u32,
    },
    /// Setup a GPIO pin with direction and optional initial value
    Setup {
        /// GPIO pin number
        pin: u32,
        /// Direction: in or out
        #[arg(long, value_enum)]
        direction: DirectionArg,
        /// Initial value for output (high or low)
        #[arg(long, value_enum)]
        initial: Option<LevelArg>,
    },
    /// Set a GPIO pin value (must be setup as OUT first)
    Set {
        /// GPIO pin number
        pin: u32,
        /// Value: high or low
        value: LevelArg,
    },
    /// Read a GPIO pin value
    Read {
        /// GPIO pin number
        pin: u32,
    },
    /// Cleanup GPIO pin(s)
    Cleanup {
        /// GPIO pin number (optional, if not specified cleanup all)
        pin: Option<u32>,
    },
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum DirectionArg {
    In,
    Out,
}

impl From<DirectionArg> for Direction {
    fn from(arg: DirectionArg) -> Self {
        match arg {
            DirectionArg::In => Direction::IN,
            DirectionArg::Out => Direction::OUT,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum LevelArg {
    High,
    Low,
}

impl From<LevelArg> for Level {
    fn from(arg: LevelArg) -> Self {
        match arg {
            LevelArg::High => Level::HIGH,
            LevelArg::Low => Level::LOW,
        }
    }
}

#[derive(Debug)]
pub enum CliError {
    PinNotFound(u32, String),
    ModelDetectionFailed(String),
    GpioError(String),
}

impl From<anyhow::Error> for CliError {
    fn from(e: anyhow::Error) -> Self {
        CliError::GpioError(e.to_string())
    }
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
            CliError::GpioError(e) => {
                write!(f, "GPIO error: {}", e)
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

fn run_high(pin: u32) -> Result<(), CliError> {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;
    gpio.setup(vec![pin], Direction::OUT, Some(Level::HIGH), None)?;
    println!("GPIO Pin {} set to HIGH", pin);
    Ok(())
}

fn run_low(pin: u32) -> Result<(), CliError> {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;
    gpio.setup(vec![pin], Direction::OUT, Some(Level::LOW), None)?;
    println!("GPIO Pin {} set to LOW", pin);
    Ok(())
}

fn run_setup(pin: u32, direction: DirectionArg, initial: Option<LevelArg>) -> Result<(), CliError> {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;
    let initial_level = initial.map(|l| l.into());
    gpio.setup(vec![pin], direction.into(), initial_level, None)?;
    println!(
        "GPIO Pin {} setup as {:?}{}",
        pin,
        direction,
        initial
            .map(|l| format!(" with initial {:?}", l))
            .unwrap_or_default()
    );
    Ok(())
}

fn run_set(pin: u32, value: LevelArg) -> Result<(), CliError> {
    // CLI invocations are stateless — there is no persistent setup() from a
    // previous run, so calling output() alone would fail with "channel not
    // set up as OUTPUT". Setup + output in one shot, identical to high/low.
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;
    let level: Level = value.into();
    gpio.setup(vec![pin], Direction::OUT, Some(level), None)?;
    println!("GPIO Pin {} set to {:?}", pin, value);
    Ok(())
}

fn run_read(pin: u32) -> Result<(), CliError> {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;
    let level = gpio.input(pin)?;
    println!("GPIO Pin {} = {:?}", pin, level);
    Ok(())
}

fn run_cleanup(pin: Option<u32>) -> Result<(), CliError> {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;
    match pin {
        Some(p) => {
            gpio.cleanup(Some(vec![p]))?;
            println!("GPIO Pin {} cleaned up", p);
        }
        None => {
            gpio.cleanup(None)?;
            println!("All GPIO pins cleaned up");
        }
    }
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
        Commands::High { pin } => {
            if let Err(e) = run_high(pin) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Low { pin } => {
            if let Err(e) = run_low(pin) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Setup {
            pin,
            direction,
            initial,
        } => {
            if let Err(e) = run_setup(pin, direction, initial) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Set { pin, value } => {
            if let Err(e) = run_set(pin, value) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Read { pin } => {
            if let Err(e) = run_read(pin) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
        Commands::Cleanup { pin } => {
            if let Err(e) = run_cleanup(pin) {
                eprintln!("Error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
