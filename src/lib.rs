//! # jetsongpio
//!
//! A Rust library that enables the use of Jetson's GPIOs.
//!
//! This is the Rust implementation of the Python library for controlling GPIO pins on NVIDIA Jetson devices.
//!

mod gpio;
mod gpio_pin_data;
pub use gpio::*;
pub use gpio_pin_data::*;