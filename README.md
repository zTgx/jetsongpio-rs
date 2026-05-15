# jetsongpio

A Rust library that enables the use of Jetson's GPIOs.

This is the Rust implementation of the Python library for controlling GPIO pins on NVIDIA Jetson devices.

## Features

- GPIO character device API (not sysfs)
- Board, BCM, Tegra SOC, and CVM pin numbering modes
- Input and output modes
- Pin cleanup

## Example

```rust
use jetsongpio::{GPIO, Direction, Level, Mode};
use std::thread;
use std::time::Duration;

fn main() {
    let mut gpio = GPIO::new();
    gpio.setmode(Mode::BOARD).unwrap();

    // Setup pin as output with initial LOW value
    gpio.setup(vec![29], Direction::OUT, Some(Level::LOW)).unwrap();

    loop {
        // Set HIGH
        gpio.output(vec![29], vec![Level::HIGH]).unwrap();
        thread::sleep(Duration::from_secs(1));

        // Set LOW
        gpio.output(vec![29], vec![Level::LOW]).unwrap();
        thread::sleep(Duration::from_secs(1));
    }

    // Cleanup
    gpio.cleanup(None).unwrap();
}
```

## Requirements

- NVIDIA Jetson platform (Jetson Nano, Jetson TX1/TX2, Jetson Xavier, Jetson Orin, etc.)
- Linux kernel with GPIO character device support
- Access to `/dev/gpiochip0` (requires proper permissions or root user)

## License

MIT