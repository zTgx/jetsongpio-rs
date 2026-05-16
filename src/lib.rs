//! # jetsongpio
//!
//! A Rust library that enables the use of Jetson's GPIOs.
//!
//! This is the Rust implementation of the Python library for controlling GPIO pins on NVIDIA Jetson devices.
//!
pub mod gpio;

// #[cfg(all(target_arch = "aarch64", target_os = "linux"))]
pub mod gpio_cdev;

pub mod gpio_pin_data;

pub mod gpio_event;

pub use gpio::{Direction, GPIO, Level};
pub use gpio_pin_data::*;
pub use gpio_event::{Edge, EventManager, GPIOEvent, GPIOEventExt, InvalidEventFlagError, blocking_wait_for_edge, open_event};

#[cfg(feature = "cli")]
pub mod cli;
