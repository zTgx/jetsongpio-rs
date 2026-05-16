//! GPIO Event Detection Module
//!
//! This module provides edge detection capabilities for GPIO pins using the Linux GPIO character
//! device API. It supports both blocking and non-blocking event detection with callback support.
//!
//! # Features
//!
//! - Edge detection (RISING, FALLING, BOTH)
//! - Non-blocking event detection with callbacks (using mio/epoll for efficiency)
//! - Blocking wait for edge events
//! - Debounce support
//! - Multi-channel event management
//!
//! # Example
//!
//! ```rust,no_run
//! use jetsongpio::{GPIO, Direction, Mode};
//! use jetsongpio::gpio_event::{Edge, blocking_wait_for_edge, request_event, open_event};
//! use std::time::Duration;
//!
//! let mut gpio = GPIO::new();
//! gpio.setmode(Mode::BOARD).unwrap();
//! gpio.setup(vec![18], Direction::IN, None).unwrap();
//!
//! // Wait for button press (falling edge)
//! let detected = gpio.wait_for_edge(18, Edge::Falling, None).unwrap();
//! if detected {
//!     println!("Button pressed!");
//! }
//! ```

use crate::gpio_cdev::{
    GPIO_GET_LINEEVENT_IOCTL, GPIOEVENT_REQUEST_BOTH_EDGES, GPIOEVENT_REQUEST_FALLING_EDGE,
    GPIOEVENT_REQUEST_RISING_EDGE, GpioEventData, GpioEventRequest, chip_open_by_label,
    request_event,
};
use anyhow::{Error, Result, anyhow};
use mio::unix::SourceFd;
use mio::{Events, Interest, Poll, Token};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::ffi::c_void;
use std::os::fd::AsRawFd;
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Edge detection types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    /// No edge detection
    None = 0,
    /// Detect rising edge (0 -> 1)
    Rising = 1,
    /// Detect falling edge (1 -> 0)
    Falling = 2,
    /// Detect both rising and falling edges
    Both = 3,
}

/// Error type for invalid GPIO event flag
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidEventFlagError(pub u32);

impl std::fmt::Display for InvalidEventFlagError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "invalid GPIO event flag: {}", self.0)
    }
}

impl std::error::Error for InvalidEventFlagError {}

impl From<Edge> for u32 {
    fn from(edge: Edge) -> Self {
        match edge {
            Edge::None => 0,
            Edge::Rising => GPIOEVENT_REQUEST_RISING_EDGE,
            Edge::Falling => GPIOEVENT_REQUEST_FALLING_EDGE,
            Edge::Both => GPIOEVENT_REQUEST_BOTH_EDGES,
        }
    }
}

impl TryFrom<u32> for Edge {
    type Error = InvalidEventFlagError;

    fn try_from(flag: u32) -> Result<Self, Self::Error> {
        match flag {
            GPIOEVENT_REQUEST_RISING_EDGE => Ok(Edge::Rising),
            GPIOEVENT_REQUEST_FALLING_EDGE => Ok(Edge::Falling),
            GPIOEVENT_REQUEST_BOTH_EDGES => Ok(Edge::Both),
            0 => Ok(Edge::None),
            invalid => Err(InvalidEventFlagError(invalid)),
        }
    }
}

/// Internal GPIO event object
struct GpioEventObject {
    /// File descriptor for the GPIO line
    value_fd: i32,
    /// Debounce time (None means no debounce)
    bouncetime: Option<Duration>,
    /// List of callback functions
    callbacks: Vec<Box<dyn Fn() + Send>>,
    /// Timestamp of last trigger (for debounce)
    last_call: Option<Instant>,
    /// Flag indicating if an event has occurred
    event_occurred: bool,
    /// Flag indicating if the thread is running
    thread_running: Arc<Mutex<bool>>,
    /// Event handler thread handle
    thread_handle: Option<JoinHandle<()>>,
}

impl GpioEventObject {
    fn new(fd: i32, bouncetime: Option<Duration>) -> Self {
        Self {
            value_fd: fd,
            bouncetime,
            callbacks: Vec::new(),
            last_call: None,
            event_occurred: false,
            thread_running: Arc::new(Mutex::new(false)),
            thread_handle: None,
        }
    }

    /// Check if event should trigger based on debounce settings
    fn should_trigger(&mut self) -> bool {
        let now = Instant::now();

        match self.bouncetime {
            Some(bouncetime) => {
                let should = match self.last_call {
                    Some(last) => now.duration_since(last) >= bouncetime,
                    None => true,
                };
                if should {
                    self.last_call = Some(now);
                }
                should
            }
            None => {
                self.last_call = Some(now);
                true
            }
        }
    }

    /// Trigger callbacks
    fn trigger_callbacks(&self) {
        for callback in &self.callbacks {
            callback();
        }
    }
}

impl Drop for GpioEventObject {
    fn drop(&mut self) {
        // Stop the thread if running
        *self.thread_running.lock().unwrap() = false;

        // Close the file descriptor
        if self.value_fd >= 0 {
            unsafe {
                libc::close(self.value_fd);
            }
        }
    }
}

/// Event manager for GPIO edge detection
pub struct EventManager {
    /// Map of (chip_name, channel) to GPIO event objects
    event_list: HashMap<(String, u32), Arc<Mutex<GpioEventObject>>>,
}

impl EventManager {
    /// Create a new event manager
    pub fn new() -> Self {
        Self {
            event_list: HashMap::new(),
        }
    }

    /// Check if an event is already added for a channel
    fn event_added(&self, chip_name: &str, channel: u32) -> bool {
        self.event_list
            .contains_key(&(chip_name.to_string(), channel))
    }

    /// Add an edge detection event (non-blocking mode with mio/epoll)
    ///
    /// # Arguments
    ///
    /// * `chip_name` - Name of the GPIO chip
    /// * `channel` - GPIO channel/pin number
    /// * `fd` - File descriptor of the GPIO event line
    /// * `bouncetime` - Optional debounce time
    ///
    /// # Returns
    ///
    /// * `Result<()>` - Success or error
    pub fn add_edge_detect(
        &mut self,
        chip_name: &str,
        channel: u32,
        fd: i32,
        bouncetime: Option<Duration>,
    ) -> Result<()> {
        if self.event_added(chip_name, channel) {
            return Err(anyhow!("Event is already added for channel {}", channel));
        }

        let gpio_obj = Arc::new(Mutex::new(GpioEventObject::new(fd, bouncetime)));

        // Start event handler thread (each thread creates its own Poll instance)
        let running = Arc::clone(&gpio_obj.lock().unwrap().thread_running);
        let gpio_obj_clone = Arc::clone(&gpio_obj);

        let handle = thread::spawn(move || {
            edge_handler(running, gpio_obj_clone, fd, channel);
        });

        // Update thread info
        {
            let mut obj = gpio_obj.lock().unwrap();
            obj.thread_handle = Some(handle);
        }

        self.event_list
            .insert((chip_name.to_string(), channel), gpio_obj);

        Ok(())
    }

    /// Remove an edge detection event
    ///
    /// # Arguments
    ///
    /// * `chip_name` - Name of the GPIO chip
    /// * `channel` - GPIO channel/pin number
    /// * `timeout` - Timeout for waiting for thread to stop
    pub fn remove_edge_detect(
        &mut self,
        chip_name: &str,
        channel: u32,
        timeout: Duration,
    ) -> Result<()> {
        let key = (chip_name.to_string(), channel);

        if let Some(gpio_obj) = self.event_list.get(&key) {
            // Stop the thread
            {
                let obj = gpio_obj.lock().unwrap();
                *obj.thread_running.lock().unwrap() = false;
            }

            // Wait for thread to exit
            thread::sleep(timeout);

            // Remove from event list
            self.event_list.remove(&key);
        }

        Ok(())
    }

    /// Add a callback function for an event
    ///
    /// # Arguments
    ///
    /// * `chip_name` - Name of the GPIO chip
    /// * `channel` - GPIO channel/pin number
    /// * `callback` - Callback function to execute on event
    pub fn add_callback(
        &mut self,
        chip_name: &str,
        channel: u32,
        callback: Box<dyn Fn() + Send>,
    ) -> Result<()> {
        let key = (chip_name.to_string(), channel);

        if let Some(gpio_obj) = self.event_list.get(&key) {
            let mut obj = gpio_obj.lock().unwrap();
            obj.callbacks.push(callback);
            Ok(())
        } else {
            Err(anyhow!("Event not found for channel {}", channel))
        }
    }

    /// Check if an edge event has occurred
    ///
    /// This clears the event flag after reading.
    ///
    /// # Arguments
    ///
    /// * `chip_name` - Name of the GPIO chip
    /// * `channel` - GPIO channel/pin number
    ///
    /// # Returns
    ///
    /// * `bool` - True if an event occurred, false otherwise
    pub fn edge_event_detected(&mut self, chip_name: &str, channel: u32) -> bool {
        let key = (chip_name.to_string(), channel);

        if let Some(gpio_obj) = self.event_list.get(&key) {
            let mut obj = gpio_obj.lock().unwrap();
            let occurred = obj.event_occurred;
            obj.event_occurred = false;
            occurred
        } else {
            false
        }
    }

    /// Clean up all events for a specific channel
    pub fn event_cleanup(&mut self, chip_name: &str, channel: u32) {
        let _ = self.remove_edge_detect(chip_name, channel, Duration::from_millis(300));
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Edge handler thread function using mio/epoll for efficient event detection
fn edge_handler(
    running: Arc<Mutex<bool>>,
    gpio_obj: Arc<Mutex<GpioEventObject>>,
    fd: i32,
    channel: u32,
) {
    // Clean initial buffer - read any pending events
    let mut initial_buf = vec![0u8; std::mem::size_of::<GpioEventData>()];
    unsafe {
        libc::read(
            fd,
            initial_buf.as_mut_ptr() as *mut c_void,
            initial_buf.len(),
        );
    }

    // Create Poll instance in this thread
    let mut poll = match Poll::new() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Failed to create Poll instance: {}", e);
            return;
        }
    };

    // Register the file descriptor with mio/epoll
    let mut source = SourceFd(&fd);
    if let Err(e) =
        poll.registry()
            .register(&mut source, Token(channel as usize), Interest::READABLE)
    {
        eprintln!("Failed to register fd with Poll: {}", e);
        return;
    }

    // Create events buffer for mio
    let mut events = Events::with_capacity(1);

    while *running.lock().unwrap() {
        // Check if we should still be running
        if !*running.lock().unwrap() {
            break;
        }

        // Use mio poll (epoll under the hood) - efficient waiting
        match poll.poll(&mut events, Some(Duration::from_millis(100))) {
            Ok(_) => {
                if events.is_empty() {
                    // Timeout without event, check running state
                    continue;
                }

                for _event in &events {
                    // Read event data
                    let mut buf = vec![0u8; std::mem::size_of::<GpioEventData>()];
                    let result =
                        unsafe { libc::read(fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };

                    if result < 0 {
                        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                            continue;
                        }
                        break;
                    }

                    if result > 0 {
                        let event_data: GpioEventData =
                            unsafe { std::ptr::read(buf.as_ptr() as *const GpioEventData) };

                        // Validate event type
                        if event_data.id != GPIOEVENT_REQUEST_RISING_EDGE
                            && event_data.id != GPIOEVENT_REQUEST_FALLING_EDGE
                        {
                            continue;
                        }

                        // Trigger callbacks with debounce
                        let mut obj = gpio_obj.lock().unwrap();
                        if obj.should_trigger() {
                            obj.event_occurred = true;
                            obj.trigger_callbacks();
                        }
                    }
                }
            }
            Err(_) => {
                // Poll error, likely due to thread shutdown
                break;
            }
        }
    }
}

/// Wait for an edge event in blocking mode using mio/epoll
///
/// This function blocks until an edge event is detected or timeout occurs.
/// Note: Unlike the non-blocking mode with callbacks, this function does NOT
/// apply debounce, matching the Python implementation behavior.
///
/// # Arguments
///
/// * `chip_fd` - File descriptor of the GPIO chip
/// * `request` - GpioEventRequest configuration (fd will be set)
/// * `bouncetime` - Not used in blocking mode (kept for API compatibility)
/// * `timeout` - Maximum time to wait for event
///
/// # Returns
///
/// * `Result<bool>` - True if event was detected, false on timeout
pub fn blocking_wait_for_edge(
    chip_fd: i32,
    request: &mut GpioEventRequest,
    bouncetime: Option<Duration>,
    timeout: Duration,
) -> Result<bool> {
    // Configure the request
    request.handleflags = crate::gpio_cdev::GPIOHANDLE_REQUEST_INPUT;

    // Get event line using ioctl
    unsafe {
        let result = libc::ioctl(
            chip_fd,
            GPIO_GET_LINEEVENT_IOCTL as libc::c_ulong,
            request as *mut GpioEventRequest,
        );

        if result < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            return Err(Error::msg(format!(
                "Opening input line event handle: errno {}",
                errno
            )));
        }
    }

    let event_fd = request.fd;
    if event_fd < 0 {
        return Err(Error::msg("Failed to get valid line event handle"));
    }

    // Set file descriptor to non-blocking
    unsafe {
        let flags = libc::fcntl(event_fd, libc::F_GETFL, 0);
        libc::fcntl(event_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    // Create mio Poll for efficient blocking wait
    let mut poll = Poll::new()?;
    let mut source = SourceFd(&event_fd);
    poll.registry()
        .register(&mut source, Token(0), Interest::READABLE)?;

    let mut events = Events::with_capacity(1);
    let start = Instant::now();

    // bouncetime is not used in blocking mode (matching Python behavior)
    let _ = bouncetime;

    loop {
        let remaining = timeout.saturating_sub(start.elapsed());
        if remaining.is_zero() {
            unsafe {
                libc::close(event_fd);
            }
            return Ok(false);
        }

        // Use mio poll for efficient blocking wait
        match poll.poll(&mut events, Some(remaining)) {
            Ok(_) => {
                if events.is_empty() {
                    // Timeout
                    unsafe {
                        libc::close(event_fd);
                    }
                    return Ok(false);
                }

                for _event in &events {
                    // Read event data
                    let mut buf = vec![0u8; std::mem::size_of::<GpioEventData>()];
                    let result =
                        unsafe { libc::read(event_fd, buf.as_mut_ptr() as *mut c_void, buf.len()) };

                    if result < 0 {
                        let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                        if errno == libc::EAGAIN || errno == libc::EWOULDBLOCK {
                            continue;
                        }
                        unsafe {
                            libc::close(event_fd);
                        }
                        return Err(Error::msg(format!("Reading GPIO event: errno {}", errno)));
                    }

                    if result > 0 {
                        let event_data: GpioEventData =
                            unsafe { std::ptr::read(buf.as_ptr() as *const GpioEventData) };

                        // Validate event type
                        if event_data.id != GPIOEVENT_REQUEST_RISING_EDGE
                            && event_data.id != GPIOEVENT_REQUEST_FALLING_EDGE
                        {
                            unsafe {
                                libc::close(event_fd);
                            }
                            return Err(Error::msg("Unknown event type"));
                        }

                        // Event detected - return immediately (no debounce in blocking mode)
                        unsafe {
                            libc::close(event_fd);
                        }
                        return Ok(true);
                    }
                }
            }
            Err(_) => {
                unsafe {
                    libc::close(event_fd);
                }
                return Err(Error::msg("Poll error during blocking wait"));
            }
        }
    }
}

/// Open a GPIO event line using ioctl
///
/// # Arguments
///
/// * `chip_fd` - File descriptor of the GPIO chip
/// * `request` - GpioEventRequest configuration (fd will be set)
///
/// # Returns
///
/// * `Result<i32>` - File descriptor of the event line
pub fn open_event(chip_fd: i32, request: &mut GpioEventRequest) -> Result<i32> {
    unsafe {
        let result = libc::ioctl(
            chip_fd,
            GPIO_GET_LINEEVENT_IOCTL as libc::c_ulong,
            request as *mut GpioEventRequest,
        );

        if result < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            return Err(Error::msg(format!(
                "Opening input line event handle: errno {}",
                errno
            )));
        }
    }

    let fd = request.fd;
    if fd < 0 {
        return Err(Error::msg("Failed to get valid line event handle"));
    }

    // Set file descriptor to non-blocking
    unsafe {
        let flags = libc::fcntl(fd, libc::F_GETFL, 0);
        libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }

    Ok(fd)
}

// Event detection methods for GPIO
impl crate::GPIO {
    /// Wait for an edge event on a GPIO channel (blocking)
    ///
    /// This function blocks until an edge event is detected on the specified channel.
    /// Similar to Python's `GPIO.wait_for_edge()`.
    ///
    /// # Arguments
    ///
    /// * `channel` - GPIO channel/pin number
    /// * `edge` - Edge type to detect (Rising, Falling, or Both)
    /// * `timeout` - Maximum time to wait for event (None = infinite wait)
    ///
    /// # Returns
    ///
    /// * `Result<bool>` - True if event was detected, false on timeout
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use jetsongpio::{GPIO, Direction, Mode};
    /// use jetsongpio::gpio_event::Edge;
    /// use std::time::Duration;
    ///
    /// let mut gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![18], Direction::IN, None).unwrap();
    ///
    /// // Wait for button press (falling edge)
    /// let detected = gpio.wait_for_edge(18, Edge::Falling, None).unwrap();
    /// if detected {
    ///     println!("Button pressed!");
    /// }
    /// ```
    pub fn wait_for_edge(
        &mut self,
        channel: u32,
        edge: Edge,
        timeout: Option<Duration>,
    ) -> Result<bool> {
        // Get channel info to find chip and line offset
        let mode = self.gpio_mode.ok_or_else(|| anyhow!("GPIO mode not set"))?;
        let channel_data = self
            .channel_data_by_mode
            .get(&mode)
            .ok_or_else(|| anyhow!("Invalid GPIO mode"))?;

        let ch_info = channel_data
            .get(&channel)
            .ok_or_else(|| anyhow!("Invalid channel: {}", channel))?;

        if ch_info.gpio_chip.is_empty() {
            return Err(anyhow!("Channel {} is not a GPIO", channel));
        }

        // Open the chip
        let chip_name = &ch_info.gpio_chip;
        let chip_fd = if !self.chip_fd_map.contains_key(chip_name) {
            let fd = chip_open_by_label(chip_name)?;
            self.chip_fd_map.insert(chip_name.clone(), fd);
            self.chip_fd_map.get(chip_name).unwrap().try_clone()?
        } else {
            self.chip_fd_map.get(chip_name).unwrap().try_clone()?
        };

        let chip_fd_raw = chip_fd.as_raw_fd();

        // Create event request
        let mut request = request_event(ch_info.line_offset, u32::from(edge), "jetsongpio-rs")?;

        // Use default timeout of 10 seconds if not specified
        let timeout = timeout.unwrap_or(Duration::from_secs(10));

        // Wait for edge event
        let detected = blocking_wait_for_edge(chip_fd_raw, &mut request, None, timeout)?;

        Ok(detected)
    }

    /// Add event detection on a GPIO channel with callback (non-blocking)
    ///
    /// This function sets up edge detection with a callback function that runs
    /// when the edge is detected. Similar to Python's `GPIO.add_event_detect()`.
    ///
    /// # Arguments
    ///
    /// * `channel` - GPIO channel/pin number
    /// * `edge` - Edge type to detect (Rising, Falling, or Both)
    /// * `callback` - Callback function to execute on event
    /// * `bouncetime` - Optional debounce time
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use jetsongpio::{GPIO, Direction, Mode};
    /// use jetsongpio::gpio_event::Edge;
    /// use std::time::Duration;
    ///
    /// let mut gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![18], Direction::IN, None).unwrap();
    ///
    /// gpio.add_event_detect(
    ///     18,
    ///     Edge::Falling,
    ///     Box::new(|| println!("Button pressed!")),
    ///     Some(Duration::from_millis(200))
    /// ).unwrap();
    ///
    /// // Main program continues...
    /// loop {
    ///     std::thread::sleep(Duration::from_secs(1));
    /// }
    /// ```
    pub fn add_event_detect(
        &mut self,
        channel: u32,
        edge: Edge,
        callback: Box<dyn Fn() + Send>,
        bouncetime: Option<Duration>,
    ) -> Result<()> {
        // Get channel info
        let mode = self.gpio_mode.ok_or_else(|| anyhow!("GPIO mode not set"))?;
        let channel_data = self
            .channel_data_by_mode
            .get(&mode)
            .ok_or_else(|| anyhow!("Invalid GPIO mode"))?;

        let ch_info = channel_data
            .get(&channel)
            .ok_or_else(|| anyhow!("Invalid channel: {}", channel))?;

        if ch_info.gpio_chip.is_empty() {
            return Err(anyhow!("Channel {} is not a GPIO", channel));
        }

        // Initialize event manager if not exists
        if self.event_manager.is_none() {
            self.event_manager = Some(EventManager::new());
        }

        // Open the chip
        let chip_name = &ch_info.gpio_chip;
        let chip_fd = if !self.chip_fd_map.contains_key(chip_name) {
            let fd = chip_open_by_label(chip_name)?;
            self.chip_fd_map.insert(chip_name.clone(), fd);
            self.chip_fd_map.get(chip_name).unwrap().try_clone()?
        } else {
            self.chip_fd_map.get(chip_name).unwrap().try_clone()?
        };

        let chip_fd_raw = chip_fd.as_raw_fd();

        // Create and open event request
        let mut request = request_event(ch_info.line_offset, u32::from(edge), "jetsongpio-rs")?;
        let event_fd = open_event(chip_fd_raw, &mut request)?;

        // Add edge detection
        self.event_manager
            .as_mut()
            .unwrap()
            .add_edge_detect(chip_name, channel, event_fd, bouncetime)?;

        // Add callback
        self.event_manager
            .as_mut()
            .unwrap()
            .add_callback(chip_name, channel, callback)?;

        Ok(())
    }

    /// Remove event detection on a GPIO channel
    ///
    /// This function removes edge detection that was previously set up with
    /// `add_event_detect()`. Similar to Python's `GPIO.remove_event_detect()`.
    ///
    /// # Arguments
    ///
    /// * `channel` - GPIO channel/pin number
    pub fn remove_event_detect(&mut self, channel: u32) -> Result<()> {
        // Get channel info to find chip name
        let mode = self.gpio_mode.ok_or_else(|| anyhow!("GPIO mode not set"))?;
        let channel_data = self
            .channel_data_by_mode
            .get(&mode)
            .ok_or_else(|| anyhow!("Invalid GPIO mode"))?;

        let ch_info = channel_data
            .get(&channel)
            .ok_or_else(|| anyhow!("Invalid channel: {}", channel))?;

        let chip_name = &ch_info.gpio_chip;

        // Remove event detection
        if let Some(ref mut manager) = self.event_manager {
            manager.event_cleanup(chip_name, channel);
        }

        Ok(())
    }

    /// Check if an event has been detected on a GPIO channel
    ///
    /// This function checks and clears the event flag. Similar to Python's
    /// `GPIO.event_detected()`.
    ///
    /// # Arguments
    ///
    /// * `channel` - GPIO channel/pin number
    ///
    /// # Returns
    ///
    /// * `bool` - True if an event was detected, false otherwise
    pub fn event_detected(&mut self, channel: u32) -> bool {
        // Get channel info to find chip name
        let mode = match self.gpio_mode {
            Some(m) => m,
            None => return false,
        };

        let channel_data = match self.channel_data_by_mode.get(&mode) {
            Some(data) => data,
            None => return false,
        };

        let ch_info = match channel_data.get(&channel) {
            Some(info) => info,
            None => return false,
        };

        let chip_name = &ch_info.gpio_chip;

        // Check if event was detected
        if let Some(ref mut manager) = self.event_manager {
            manager.edge_event_detected(chip_name, channel)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_edge_conversion() {
        assert_eq!(u32::from(Edge::Rising), GPIOEVENT_REQUEST_RISING_EDGE);
        assert_eq!(u32::from(Edge::Falling), GPIOEVENT_REQUEST_FALLING_EDGE);
        assert_eq!(u32::from(Edge::Both), GPIOEVENT_REQUEST_BOTH_EDGES);
        assert_eq!(u32::from(Edge::None), 0);

        assert_eq!(
            Edge::try_from(GPIOEVENT_REQUEST_RISING_EDGE).unwrap(),
            Edge::Rising
        );
        assert_eq!(
            Edge::try_from(GPIOEVENT_REQUEST_FALLING_EDGE).unwrap(),
            Edge::Falling
        );
        assert_eq!(
            Edge::try_from(GPIOEVENT_REQUEST_BOTH_EDGES).unwrap(),
            Edge::Both
        );
        assert_eq!(Edge::try_from(0).unwrap(), Edge::None);

        // Test invalid flag
        assert!(Edge::try_from(999).is_err());
    }

    #[test]
    fn test_event_manager() {
        let manager = EventManager::new();
        assert!(!manager.event_added("test", 1));
    }
}
