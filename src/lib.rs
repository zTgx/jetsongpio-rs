//! # jetsongpio
//!
//! A Rust library for controlling GPIO pins on NVIDIA Jetson platforms.
//!
//! This is a Rust implementation of the [Jetson.GPIO](https://github.com/NVIDIA/jetson-gpio)
//! Python library, using the Linux GPIO character device API (`/dev/gpiochipX`).
//!
//! # Quick Start
//!
//! ```no_run
//! use jetsongpio::{GPIO, Direction, Level, Mode};
//!
//! let gpio = GPIO::new();
//! gpio.setmode(Mode::BOARD)?;
//! gpio.setup(vec![18], Direction::OUT, Some(Level::LOW), None)?;
//! gpio.output(vec![18], vec![Level::HIGH])?;
//! gpio.cleanup(None)?;
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # Pin Numbering
//!
//! Two modes are supported:
//! - [`Mode::BOARD`] — physical pin number on the 40-pin header
//! - [`Mode::BCM`] — BCM numbering
//!
//! Python's `CVM` / `TEGRA_SOC` modes are intentionally not implemented:
//! they key the channel map by pin-name string, which doesn't fit this port's
//! integer-channel API.

#![allow(dead_code)]

// ── Modules ──────────────────────────────────────────────────────────────

pub(crate) mod gpio;
pub(crate) mod gpio_cdev;
pub(crate) mod gpio_event;
pub(crate) mod gpio_pin_data;

#[cfg(feature = "cli")]
#[doc(hidden)]
pub mod cli;

// ── Public re-exports ────────────────────────────────────────────────────

pub use gpio::{Direction, GPIO, Level, PWM};
pub use gpio_event::{Edge, EdgeCallback, EventManager};
pub use gpio_pin_data::{ChannelInfo, JetsonInfo, Mode, get_model};
