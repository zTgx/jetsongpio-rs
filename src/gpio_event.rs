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
//! use jetsongpio::{GPIO, Direction, Edge, Mode};
//! use jetsongpio::gpio_event::{blocking_wait_for_edge, request_event, open_event};
//! use std::time::Duration;
//!
//! let gpio = GPIO::new();
//! gpio.setmode(Mode::BOARD).unwrap();
//! gpio.setup(vec![18], Direction::IN, None, None).unwrap();
//!
//! // Wait for button press (falling edge), block forever
//! if let Some(ch) = gpio.wait_for_edge(18, Edge::Falling, None).unwrap() {
//!     println!("Button pressed on channel {ch}!");
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

/// Callback signature for edge-detection events.
///
/// Receives the channel number (in the current pin numbering mode) that
/// triggered. Matches Python's `lambda: callback(channel)` (gpio.py:431).
pub type EdgeCallback = Box<dyn Fn(u32) + Send + Sync>;

/// Internal GPIO event object
struct GpioEventObject {
    /// File descriptor for the GPIO line
    value_fd: i32,
    /// Channel number passed to callbacks (matches Python semantics)
    channel: u32,
    /// Debounce time (None means no debounce)
    bouncetime: Option<Duration>,
    /// List of callback functions
    callbacks: Vec<EdgeCallback>,
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
    fn new(fd: i32, channel: u32, bouncetime: Option<Duration>) -> Self {
        Self {
            value_fd: fd,
            channel,
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
            callback(self.channel);
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
    /// * `polltime` - How long the handler thread waits per poll iteration
    ///   before checking the shutdown flag. Smaller values shut down faster
    ///   but use more CPU.
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
        polltime: Duration,
    ) -> Result<()> {
        if self.event_added(chip_name, channel) {
            return Err(anyhow!("Event is already added for channel {}", channel));
        }

        let gpio_obj = Arc::new(Mutex::new(GpioEventObject::new(fd, channel, bouncetime)));

        // Start event handler thread (each thread creates its own Poll instance)
        let running = Arc::clone(&gpio_obj.lock().unwrap().thread_running);
        *running.lock().unwrap() = true;
        let gpio_obj_clone = Arc::clone(&gpio_obj);

        let handle = thread::spawn(move || {
            edge_handler(running, gpio_obj_clone, fd, channel, polltime);
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

    /// Returns true if the channel currently has edge detection registered.
    /// Mirrors Python `gpio_event.gpio_event_added`.
    pub fn is_event_added(&self, chip_name: &str, channel: u32) -> bool {
        self.event_added(chip_name, channel)
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

        if let Some(gpio_obj) = self.event_list.remove(&key) {
            // Signal the thread to stop and take the JoinHandle.
            let handle = {
                let mut obj = gpio_obj.lock().unwrap();
                *obj.thread_running.lock().unwrap() = false;
                obj.thread_handle.take()
            };

            // Join the thread with a timeout.
            if let Some(handle) = handle {
                let start = Instant::now();
                // Spin-join with timeout so we don't block forever.
                loop {
                    if handle.is_finished() {
                        let _ = handle.join();
                        break;
                    }
                    if start.elapsed() >= timeout {
                        // Thread did not exit in time; it will detach on drop.
                        eprintln!(
                            "Warning: event handler thread for channel {} did not exit within timeout",
                            channel
                        );
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
            }
            // gpio_obj Arc dropped here; if the thread holds the last Arc,
            // GpioEventObject::Drop will close value_fd after the thread exits.
        }

        Ok(())
    }

    /// Add a callback function for an event
    ///
    /// # Arguments
    ///
    /// * `chip_name` - Name of the GPIO chip
    /// * `channel` - GPIO channel/pin number
    /// * `callback` - Callback function to execute on event. Receives the
    ///   channel number as its sole argument (matches Python semantics).
    pub fn add_callback(
        &mut self,
        chip_name: &str,
        channel: u32,
        callback: EdgeCallback,
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
    polltime: Duration,
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
        match poll.poll(&mut events, Some(polltime)) {
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
/// * `timeout` - Maximum time to wait for event. `None` blocks forever
///   (mirrors Python `select.select(.., None)`).
///
/// # Returns
///
/// * `Result<bool>` - True if event was detected, false on timeout
pub fn blocking_wait_for_edge(
    chip_fd: i32,
    request: &mut GpioEventRequest,
    bouncetime: Option<Duration>,
    timeout: Option<Duration>,
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
        // `None` here means block forever — mirrors Python's
        // `select.select([], [], [], None)` when timeout is None.
        let remaining = match timeout {
            Some(t) => {
                let r = t.saturating_sub(start.elapsed());
                if r.is_zero() {
                    unsafe {
                        libc::close(event_fd);
                    }
                    return Ok(false);
                }
                Some(r)
            }
            None => None,
        };

        // Use mio poll for efficient blocking wait
        match poll.poll(&mut events, remaining) {
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
    /// * `timeout` - Maximum time to wait for event. `None` blocks forever
    ///   (matches Python `wait_for_edge(channel, edge, timeout=None)`).
    ///
    /// # Returns
    ///
    /// * `Result<Option<u32>>` - `Some(channel)` on detection, `None` on
    ///   timeout. Mirrors Python's `wait_for_edge` return value.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use jetsongpio::{GPIO, Direction, Edge, Mode};
    /// use std::time::Duration;
    ///
    /// let gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![18], Direction::IN, None, None).unwrap();
    ///
    /// // Wait for button press (falling edge), block forever
    /// if let Some(ch) = gpio.wait_for_edge(18, Edge::Falling, None).unwrap() {
    ///     println!("Button pressed on channel {ch}!");
    /// }
    /// ```
    pub fn wait_for_edge(
        &self,
        channel: u32,
        edge: Edge,
        timeout: Option<Duration>,
    ) -> Result<Option<u32>> {
        let (chip_fd_raw, ch_info_line_offset) = {
            let mut inner = self.inner();
            let mode = inner
                .gpio_mode
                .ok_or_else(|| anyhow!("GPIO mode not set"))?;
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

            let chip_name = &ch_info.gpio_chip;
            let chip_fd = if !inner.chip_fd_map.contains_key(chip_name) {
                let fd = chip_open_by_label(chip_name)?;
                inner.chip_fd_map.insert(chip_name.clone(), fd);
                inner.chip_fd_map.get(chip_name).unwrap().try_clone()?
            } else {
                inner.chip_fd_map.get(chip_name).unwrap().try_clone()?
            };

            // Close any existing line handle so the kernel allows re-request as event line.
            if let Some(setup_ch_info) = inner.channel_data.get_mut(&channel) {
                if let Some(line_handle) = setup_ch_info.line_handle.take() {
                    let _ = crate::gpio_cdev::close_line(Some(line_handle));
                }
            }

            (chip_fd.as_raw_fd(), ch_info.line_offset)
        };

        let mut request = request_event(ch_info_line_offset, u32::from(edge), "jetsongpio-rs")?;
        let detected = blocking_wait_for_edge(chip_fd_raw, &mut request, None, timeout)?;

        Ok(if detected { Some(channel) } else { None })
    }

    /// Add event detection on a GPIO channel with optional callback (non-blocking).
    ///
    /// Mirrors Python `GPIO.add_event_detect(channel, edge, callback, bouncetime, polltime)`.
    ///
    /// # Arguments
    ///
    /// * `channel` - GPIO channel/pin number
    /// * `edge` - Edge type to detect (Rising, Falling, or Both)
    /// * `callback` - Callback to execute on event. Receives the channel
    ///   number (matches Python `lambda: callback(channel)`). `None` registers
    ///   detection without a callback; use [`Self::add_event_callback`] later.
    /// * `bouncetime` - Optional debounce interval
    /// * `polltime` - Per-iteration poll timeout in the handler thread. `None`
    ///   uses 200 ms (matches Python's `polltime=0.2`).
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use jetsongpio::{GPIO, Direction, Edge, Mode};
    /// use std::time::Duration;
    ///
    /// let gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![18], Direction::IN, None, None).unwrap();
    ///
    /// gpio.add_event_detect(
    ///     18,
    ///     Edge::Falling,
    ///     Some(Box::new(|ch| println!("Button pressed on {ch}!"))),
    ///     Some(Duration::from_millis(200)),
    ///     None,
    /// ).unwrap();
    ///
    /// // Main program continues...
    /// loop {
    ///     std::thread::sleep(Duration::from_secs(1));
    /// }
    /// ```
    pub fn add_event_detect(
        &self,
        channel: u32,
        edge: Edge,
        callback: Option<EdgeCallback>,
        bouncetime: Option<Duration>,
        polltime: Option<Duration>,
    ) -> Result<()> {
        let (chip_fd_raw, ch_info_line_offset, chip_name) = {
            let mut inner = self.inner();
            let mode = inner
                .gpio_mode
                .ok_or_else(|| anyhow!("GPIO mode not set"))?;
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

            if inner.event_manager.is_none() {
                inner.event_manager = Some(EventManager::new());
            }

            let chip_name = ch_info.gpio_chip.clone();
            let chip_fd = if !inner.chip_fd_map.contains_key(&chip_name) {
                let fd = chip_open_by_label(&chip_name)?;
                inner.chip_fd_map.insert(chip_name.clone(), fd);
                inner.chip_fd_map.get(&chip_name).unwrap().try_clone()?
            } else {
                inner.chip_fd_map.get(&chip_name).unwrap().try_clone()?
            };

            // Close existing line handle before requesting an event line.
            if let Some(setup_ch_info) = inner.channel_data.get_mut(&channel) {
                if let Some(line_handle) = setup_ch_info.line_handle.take() {
                    let _ = crate::gpio_cdev::close_line(Some(line_handle));
                }
            }

            (chip_fd.as_raw_fd(), ch_info.line_offset, chip_name)
        };

        // Open event line (ioctl — lock is released so other threads aren't
        // blocked during the kernel call).
        let mut request = request_event(ch_info_line_offset, u32::from(edge), "jetsongpio-rs")?;
        let event_fd = open_event(chip_fd_raw, &mut request)?;

        let polltime = polltime.unwrap_or(Duration::from_millis(200));

        {
            let mut inner = self.inner();
            inner
                .event_manager
                .as_mut()
                .unwrap()
                .add_edge_detect(&chip_name, channel, event_fd, bouncetime, polltime)?;

            if let Some(cb) = callback {
                inner
                    .event_manager
                    .as_mut()
                    .unwrap()
                    .add_callback(&chip_name, channel, cb)?;
            }
        }

        // Give the handler thread a moment to come up and drain pre-existing edges.
        thread::sleep(Duration::from_secs(1));

        Ok(())
    }

    /// Append an additional callback to an already-detecting channel.
    ///
    /// Mirrors Python `GPIO.add_event_callback`. Errors if detection was not
    /// previously registered via [`Self::add_event_detect`].
    pub fn add_event_callback(&self, channel: u32, callback: EdgeCallback) -> Result<()> {
        let mut inner = self.inner();
        let mode = inner
            .gpio_mode
            .ok_or_else(|| anyhow!("GPIO mode not set"))?;
        let channel_data = self
            .channel_data_by_mode
            .get(&mode)
            .ok_or_else(|| anyhow!("Invalid GPIO mode"))?;

        let ch_info = channel_data
            .get(&channel)
            .ok_or_else(|| anyhow!("Invalid channel: {}", channel))?;

        let chip_name = ch_info.gpio_chip.clone();
        let manager = inner
            .event_manager
            .as_mut()
            .ok_or_else(|| anyhow!("Add event detection using add_event_detect first"))?;

        if !manager.is_event_added(&chip_name, channel) {
            return Err(anyhow!(
                "Add event detection using add_event_detect first before adding a callback"
            ));
        }

        manager.add_callback(&chip_name, channel, callback)?;

        // Same wait-for-thread-startup as add_event_detect.
        drop(inner);
        thread::sleep(Duration::from_secs(1));

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
    /// * `timeout` - How long to wait for the handler thread to exit. `None`
    ///   uses 500 ms (matches Python's `timeout=0.5`).
    pub fn remove_event_detect(&self, channel: u32, timeout: Option<Duration>) -> Result<()> {
        let mut inner = self.inner();
        let mode = inner
            .gpio_mode
            .ok_or_else(|| anyhow!("GPIO mode not set"))?;
        let channel_data = self
            .channel_data_by_mode
            .get(&mode)
            .ok_or_else(|| anyhow!("Invalid GPIO mode"))?;

        let ch_info = channel_data
            .get(&channel)
            .ok_or_else(|| anyhow!("Invalid channel: {}", channel))?;

        let chip_name = &ch_info.gpio_chip;
        let timeout = timeout.unwrap_or(Duration::from_millis(500));

        if let Some(ref mut manager) = inner.event_manager {
            let _ = manager.remove_edge_detect(chip_name, channel, timeout);
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
    pub fn event_detected(&self, channel: u32) -> bool {
        let mut inner = self.inner();

        let mode = match inner.gpio_mode {
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

        if let Some(ref mut manager) = inner.event_manager {
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
