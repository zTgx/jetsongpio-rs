# jetsongpio-rs

A Rust library for controlling GPIO pins on NVIDIA Jetson platforms. This is a Rust implementation of the [Jetson.GPIO](https://github.com/NVIDIA/jetson-gpio) Python library, using the Linux GPIO character device API (`/dev/gpiochipX`).

## Supported Platforms

| Model | Status |
|---|---|
| Jetson Orin NX | Supported |
| Jetson Orin Nano | Supported |
| Jetson AGX Orin | Supported |
| Jetson Xavier NX | Supported |
| Jetson AGX Xavier | Supported |
| Jetson TX2 NX | Supported |
| Jetson TX2 | Supported |
| Jetson TX1 | Supported |
| Jetson Nano | Supported |
| Clara AGX Xavier | Supported |
| Jetson Thor Reference | Supported |

Pin definitions are automatically synchronized from the upstream `jetson-gpio` repository via a git submodule and code generation at build time. See the [Data Synchronization](#data-synchronization) section for details.

## Features

- GPIO character device API (not sysfs)
- Pin numbering modes: BOARD, BCM, Tegra SOC, CVM
- Input and output modes with configurable initial state
- Hardware PWM output with configurable frequency and duty cycle
- GPIO event detection (rising, falling, both edges) with epoll-based polling
- Pinmux register address lookup
- Automatic Jetson model detection via device tree
- CLI tool for quick GPIO operations

## Requirements

- NVIDIA Jetson platform (aarch64, Linux)
- Linux kernel with GPIO character device support (`/dev/gpiochipX`)
- Appropriate permissions for `/dev/gpiochipX` (root, or udev rules)

## Usage

Add the dependency to `Cargo.toml`:

```toml
[dependencies]
jetsongpio = "0.1"
```

### Basic Setup

```rust
use jetsongpio::{GPIO, Direction, Level, Mode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD)?;

    // Setup pin 18 as output, initial LOW
    gpio.setup(vec![18], Direction::OUT, Some(Level::LOW), None)?;

    // Set HIGH
    gpio.output(vec![18], vec![Level::HIGH])?;

    // Cleanup
    gpio.cleanup(None)?;
    Ok(())
}
```

### Pin Numbering Modes

Four pin numbering schemes are supported:

```rust
use jetsongpio::Mode;

Mode::BOARD      // Board pin number (physical pin on the 40-pin header)
Mode::BCM        // BCM mode numbering
Mode::CVM        // CVM connector name
Mode::TegraSoc   // Tegra SoC pin name
```

Call `setmode()` before any other GPIO operation. The mode cannot be changed once set.

### Digital Output

```rust
use jetsongpio::{GPIO, Direction, Level, Mode};

let mut gpio = GPIO::new();
gpio.setmode(Mode::BOARD)?;

// Setup multiple pins as output
gpio.setup(vec![7, 11], Direction::OUT, Some(Level::LOW), None)?;

// Write values
gpio.output(vec![7, 11], vec![Level::HIGH, Level::LOW])?;
```

### Digital Input

```rust
use jetsongpio::{GPIO, Direction, Mode};

let mut gpio = GPIO::new();
gpio.setmode(Mode::BCM)?;

// Setup pin as input
gpio.setup(vec![18], Direction::IN, None, None)?;

// Read value
let value = gpio.input(18)?;
println!("Pin 18: {:?}", value);
```

### Event Detection

The library supports edge detection using the Linux GPIO event interface backed by epoll:

```rust
use jetsongpio::{GPIO, Direction, Mode};
use jetsongpio::gpio_event::Edge;

let mut gpio = GPIO::new();
gpio.setmode(Mode::BCM)?;
gpio.setup(vec![18], Direction::IN, None, None)?;

// Blocking wait for falling edge with 5-second timeout
let detected = gpio.wait_for_edge(18, Edge::Falling, Some(std::time::Duration::from_secs(5)))?;
if detected {
    println!("Edge detected!");
}
```

### Querying Pin State

```rust
let direction = gpio.gpio_function(18)?;
println!("Pin 18 direction: {:?}", direction);
```

### Hardware PWM

The library supports hardware PWM output via the Linux sysfs PWM interface:

```rust
use jetsongpio::{GPIO, Mode, PWM};

let mut gpio = GPIO::new();
gpio.setmode(Mode::BCM)?;

// Create PWM on BCM pin 18 at 50 Hz
let mut pwm = PWM::new(&mut gpio, 18, 50.0)?;

// Start with 25% duty cycle
pwm.start(25.0)?;

// Change duty cycle
pwm.set_duty_cycle(50.0)?;

// Change frequency
pwm.set_frequency(100.0)?;

// Stop PWM output
pwm.stop()?;
```

PWM-capable pins vary by model (BCM numbering):
- Jetson AGX Xavier / Clara AGX Xavier / Jetson AGX Orin: pin 18
- Jetson Nano / Jetson Xavier NX / Jetson Orin NX / Jetson Orin Nano: pin 33
- Jetson TX2 NX: pin 32

### Cleanup

Always call `cleanup()` before exiting to release GPIO lines:

```rust
// Cleanup specific pins
gpio.cleanup(Some(vec![7, 11]))?;

// Cleanup all configured pins
gpio.cleanup(None)?;
```

## CLI Tool

The library includes a command-line tool for quick GPIO operations. Build and install with:

```bash
cargo build --release
sudo ./target/release/jetsongpio <command>
```

### Commands

```
jetsongpio pinmux-lookup <PIN>     Look up pinmux register address (BOARD pin number)
jetsongpio high <PIN>              Set pin HIGH (auto-setup as output)
jetsongpio low <PIN>               Set pin LOW (auto-setup as output)
jetsongpio setup <PIN>             Setup pin direction and optional initial value
  --direction <in|out>             Pin direction (required)
  --initial <high|low>             Initial value for output mode (optional)
jetsongpio set <PIN> <high|low>    Set pin value (must be setup as output first)
jetsongpio read <PIN>              Read pin value
jetsongpio cleanup [PIN]           Cleanup pin(s), all pins if PIN omitted
```

All pin numbers in the CLI use BOARD mode.

### Examples

```bash
# Look up pinmux register address for board pin 7
jetsongpio pinmux-lookup 7

# Set board pin 18 to HIGH
jetsongpio high 18

# Read board pin 12
jetsongpio read 12

# Setup board pin 13 as output with initial LOW
jetsongpio setup 13 --direction out --initial low

# Cleanup all pins
jetsongpio cleanup
```

## Examples

The `examples/` directory contains complete example programs:

```bash
# Toggle an LED on BCM pin 18
cargo run --example simple_out

# Read a button on BCM pin 18
cargo run --example simple_input

# Button interrupt via edge detection
cargo run --example button_interrupt

# Button event detection (blocking mode)
cargo run --example button_event

# Hardware PWM breathing LED
cargo run --example simple_pwm

# GPIO output toggle on BOARD pin 29
cargo run --example gpio
```

## Data Synchronization

Pin definitions, compatibility strings, and board metadata are sourced from the upstream [NVIDIA/jetson-gpio](https://github.com/NVIDIA/jetson-gpio) Python library. The `vendor/jetson-gpio` directory is a git submodule pointing to that repository.

During `cargo build`, `build.rs` parses `vendor/jetson-gpio/lib/python/Jetson/GPIO/gpio_pin_data.py` and generates Rust code into `OUT_DIR`. This includes:

- Model constants (`JETSON_ORIN_NX`, `JETSON_NANO`, etc.)
- `get_jetson_models()` -- list of all supported models
- `get_<model>_pin_defs()` -- pin definitions for each model
- `get_compats_<model>()` -- device tree compatibility strings
- `get_jetson_data()` -- pin defs and board metadata for the detected model

To update pin data from upstream:

```bash
cd vendor/jetson-gpio
git pull origin master
cd ../..
cargo build
```

Pin data is regenerated automatically whenever the Python source file changes (tracked via `cargo:rerun-if-changed`).

## License

MIT
