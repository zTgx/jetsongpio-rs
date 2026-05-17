# jetsongpio-rs

A Rust library for controlling GPIO pins on NVIDIA Jetson platforms.

Jetson TX1, TX2, AGX Xavier, Xavier NX, Nano, AGX Orin, Orin NX, Orin Nano, and
Thor Reference development boards contain a 40 pin GPIO header. These GPIOs can be
controlled for digital input and output using this library. The API is modeled after
the [Jetson.GPIO](https://github.com/NVIDIA/jetson-gpio) Python library.

This library uses the Linux GPIO character device API (`/dev/gpiochipX`) and
does not depend on the deprecated sysfs interface.

Pin definitions are automatically synchronized from the upstream `jetson-gpio`
repository via a git submodule and code generation at build time. See the
[Data Synchronization](#data-synchronization) section for details.

## Requirements

- NVIDIA Jetson platform (aarch64, Linux)
- Linux kernel with GPIO character device support (`/dev/gpiochipX`)
- Appropriate permissions for `/dev/gpiochipX` (root, or udev rules)

## Setting User Permissions

In order to use the library without root, the correct user permissions must be
set first.

Create a new gpio user group and add your user to it:

```shell
sudo groupadd -f -r gpio
sudo usermod -a -G gpio your_user_name
```

Install custom udev rules:

```shell
sudo cp vendor/jetson-gpio/lib/python/Jetson/GPIO/99-gpio.rules /etc/udev/rules.d/
```

For the new rule to take place, either reboot or reload the udev rules:

```shell
sudo udevadm control --reload-rules && sudo udevadm trigger
```

## Usage

Add the dependency to `Cargo.toml`:

```toml
[dependencies]
jetsongpio = "0.1"
```

### 1. Pin Numbering

The library provides four ways of numbering the I/O pins:

```rust
use jetsongpio::{GPIO, Mode};

let mut gpio = GPIO::new();

// Board pin number (physical pin on the 40-pin header)
gpio.setmode(Mode::BOARD)?;

// Broadcom SoC GPIO numbers
gpio.setmode(Mode::BCM)?;

// CVM/CVB connector signal names
gpio.setmode(Mode::CVM)?;

// Tegra SoC pin names
gpio.setmode(Mode::TegraSoc)?;
```

Call `setmode()` before any other GPIO operation. The mode cannot be changed
once set. To check which mode has been set:

```rust
let mode = gpio.getmode(); // Returns Option<Mode>
```

### 2. Warnings

It is possible that the GPIO you are trying to use is already being used
external to the current application. The library will warn you if the GPIO
being used is configured to anything but the default direction (input). It will
also warn you if you try cleaning up before setting up the mode and channels.

To disable warnings:

```rust
gpio.setwarnings(false);
```

### 3. Set up a Channel

The GPIO channel must be set up before use as input or output.

To configure a channel as input:

```rust
use jetsongpio::{GPIO, Direction, Mode};

let mut gpio = GPIO::new();
gpio.setmode(Mode::BOARD)?;
gpio.setup(vec![18], Direction::IN, None, None)?;
```

To configure a channel as output with an initial value:

```rust
use jetsongpio::Level;

gpio.setup(vec![18], Direction::OUT, Some(Level::LOW), None)?;
```

Multiple channels can be set up at once:

```rust
gpio.setup(vec![7, 11], Direction::OUT, Some(Level::LOW), None)?;
```

### 4. Input

To read the value of a channel:

```rust
let value = gpio.input(18)?;
// Returns Level::LOW or Level::HIGH
```

### 5. Output

To set the value of pin(s) configured as output:

```rust
// Single pin
gpio.output(vec![18], vec![Level::HIGH])?;

// Multiple pins
gpio.output(vec![7, 11], vec![Level::HIGH, Level::LOW])?;
```

### 6. Cleanup

At the end of the program, it is good to clean up the channels so that all
pins are set in their default state.

```rust
// Cleanup all channels
gpio.cleanup(None)?;

// Cleanup specific channels
gpio.cleanup(Some(vec![7, 11]))?;
```

### 7. Check Function of GPIO Channels

```rust
let direction = gpio.gpio_function(18)?;
// Returns Direction::IN, Direction::OUT, or Direction::HardPwm
```

### 8. Interrupts

Aside from busy-polling, the library provides additional ways of monitoring an
input event:

#### `wait_for_edge()`

This function blocks the calling thread until the provided edge is detected:

```rust
use jetsongpio::{GPIO, Direction, Edge, Mode};
use std::time::Duration;

let mut gpio = GPIO::new();
gpio.setmode(Mode::BOARD)?;
gpio.setup(vec![18], Direction::IN, None, None)?;

// Blocking wait with timeout
let detected = gpio.wait_for_edge(18, Edge::Falling, Some(Duration::from_secs(5)))?;
```

The edge parameter can be `Edge::Rising`, `Edge::Falling`, or `Edge::Both`.

#### `event_detected()`

This function can be used to periodically check if an event occurred since the
last call:

```rust
gpio.add_event_detect(18, Edge::Rising, None, None)?;

// ... in your main loop ...
if gpio.event_detected(18)? {
    println!("Event detected on pin 18!");
}
```

#### Callback function

A callback function can be run when an edge is detected, concurrent to the main
program:

```rust
gpio.add_event_detect(
    18,
    Edge::Falling,
    Some(Box::new(|| println!("Button pressed!"))),
    Some(Duration::from_millis(200)), // debounce
)?;
```

To remove event detection:

```rust
gpio.remove_event_detect(18)?;
```

### 9. Hardware PWM

The library supports hardware PWM output via the Linux sysfs PWM interface.
Only pins with attached hardware PWM controllers are supported. Jetson Nano
supports 2 PWM channels, Jetson AGX Xavier supports 3 PWM channels. Jetson TX1
and TX2 do not support any PWM channels.

The system pinmux must be configured to connect the hardware PWM controller(s)
to the relevant pins. If the pinmux is not configured, PWM signals will not
reach the pins! The library does not dynamically modify the pinmux configuration.
Read the L4T documentation for details on how to configure the pinmux.

```rust
use jetsongpio::{GPIO, Mode, PWM};

let mut gpio = GPIO::new();
gpio.setmode(Mode::BOARD)?;

// Create PWM on pin 18 at 50 Hz
let mut pwm = PWM::new(&mut gpio, 18, 50.0)?;

// Start with 25% duty cycle
pwm.start(25.0)?;

// Change duty cycle (0.0 - 100.0)
pwm.set_duty_cycle(50.0)?;

// Change frequency
pwm.set_frequency(100.0)?;

// Stop PWM output
pwm.stop()?;
```

PWM-capable pins vary by model:

| Model | BOARD Pin |
|---|---|
| Jetson AGX Xavier / Clara AGX Xavier / Jetson AGX Orin | 18 |
| Jetson Nano / Jetson Xavier NX / Jetson Orin NX / Jetson Orin Nano | 33 |
| Jetson TX2 NX | 32 |

## CLI Tool

The library includes a command-line tool for quick GPIO operations. Install with:

```bash
cargo install jetsongpio
```

### Commands

```
jetsongpio pinmux-lookup <PIN>     Look up pinmux register address (BOARD pin)
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
jetsongpio pinmux-lookup 7
jetsongpio high 18
jetsongpio read 12
jetsongpio setup 13 --direction out --initial low
jetsongpio cleanup
```

## Examples

The `examples/` directory contains complete example programs. All examples use
BOARD pin numbering mode.

```bash
# Toggle an LED on pin 18
cargo run --example simple_out

# Read a button on pin 18
cargo run --example simple_input

# Button press with blocking wait (LED on pin 12)
cargo run --example button_event

# Button interrupt via edge callback (LEDs on pin 12 and 13)
cargo run --example button_interrupt

# Hardware PWM breathing LED
cargo run --example simple_pwm

# GPIO output toggle on pin 29
cargo run --example gpio
```

## Environment Variables

The library supports two environment variables for model detection:

- **`JETSON_TESTING_MODEL_NAME`** — Takes precedence over device tree detection.
  Useful for testing on non-Jetson hosts. Value must be a valid model constant
  (e.g. `JETSON_ORIN_NX`).

- **`JETSON_MODEL_NAME`** — Used as a fallback when `/proc/device-tree/compatible`
  is not available (e.g. Docker containers). Same format.

Valid model names: `JETSON_TX1`, `JETSON_TX2`, `JETSON_TX2_NX`,
`CLARA_AGX_XAVIER`, `JETSON_XAVIER`, `JETSON_NANO`, `JETSON_NX`,
`JETSON_ORIN`, `JETSON_ORIN_NX`, `JETSON_ORIN_NANO`,
`JETSON_THOR_REFERENCE`.

## Data Synchronization

Pin definitions, compatibility strings, and board metadata are sourced from the
upstream [NVIDIA/jetson-gpio](https://github.com/NVIDIA/jetson-gpio) Python
library. The `vendor/jetson-gpio` directory is a git submodule pointing to that
repository.

During `cargo build`, `build.rs` parses
`vendor/jetson-gpio/lib/python/Jetson/GPIO/gpio_pin_data.py` and generates
Rust code into `OUT_DIR`. This includes:

- Model constants (`JETSON_ORIN_NX`, `JETSON_NANO`, etc.)
- `get_jetson_models()` -- list of all supported models
- `get_<model>_pin_defs()` -- pin definitions for each model
- `get_compats_<model>()` -- device tree compatibility strings
- `get_jetson_data()` -- pin defs and board metadata for the detected model

To update pin data from upstream:

```bash
git submodule update --init --remote vendor/jetson-gpio
cargo build
```

Pin data is regenerated automatically whenever the Python source file changes
(tracked via `cargo:rerun-if-changed`).

## License

MIT
