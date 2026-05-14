//! # jetsongpio
//!
//! A Rust library that enables the use of Jetson's GPIOs.
//!
//! This is the Rust implementation of the Python library for controlling GPIO pins on NVIDIA Jetson devices.
//!
//! Note: This crate is currently a placeholder to reserve the crate name.

/// Says hello to the world
pub fn hello() -> String {
    "Hello from jetsongpio!".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hello() {
        assert_eq!(hello(), "Hello from jetsongpio!");
    }
}
