use crate::gpio_cdev::*;
use crate::gpio_pin_data::{ChannelInfo, JetsonInfo, Mode, get_data};
use anyhow::Error;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, Write};
use std::os::fd::AsRawFd;
use std::path::Path;
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

// GPIO character device constants
const GPIOHANDLE_REQUEST_INPUT: u32 = 0x1;
const GPIOHANDLE_REQUEST_OUTPUT: u32 = 0x2;

/// Specifies the GPIO pin value in output mode.
///
/// * `LOW` - 0
/// * `HIGH` - 1
///
/// # Example
///
/// When writing to a GPIO pin, you must specify the value. For example, to set
/// GPIO pin 7 to HIGH and GPIO pin 11 to LOW:
///
/// ```rust
/// use jetsongpio::{GPIO, Level, Direction, Mode};
///
/// let gpio = GPIO::new();
/// gpio.setmode(Mode::BOARD).unwrap();
///
/// gpio.setup(vec![7, 11], Direction::OUT, None, None).unwrap();
/// gpio.output(vec![7, 11], vec![Level::HIGH, Level::LOW]).unwrap();
/// ```
#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Level {
    LOW = 0,
    HIGH = 1,
}

/// Specifies the GPIO pin direction.
///
/// * `IN` - Input
/// * `OUT` - Output
/// * `HardPwm` - Hardware PWM output
/// * `UNKNOWN` - Unknown direction for GPIOs that are not yet setup
///
/// # Example
///
/// When setting up a GPIO pin, you must specify the direction. For example, to
/// set up GPIO pin 7 as an output:
///
/// ```rust
/// use jetsongpio::{GPIO, Direction};
///
/// let gpio = GPIO::new();
///
/// gpio.setup(vec![7], Direction::OUT, None, None).unwrap();
/// ```
#[derive(PartialEq, Clone, Copy)]
pub enum Direction {
    UNKNOWN = -1,
    OUT = 0,
    IN = 1,
    HardPwm = 43,
}

impl Direction {
    pub fn is_valid(&self) -> bool {
        matches!(self, Direction::OUT | Direction::IN | Direction::HardPwm)
    }

    fn from_cdev(value: i32) -> Self {
        if value == GPIOHANDLE_REQUEST_INPUT as i32 {
            Direction::IN
        } else if value == GPIOHANDLE_REQUEST_OUTPUT as i32 {
            Direction::OUT
        } else {
            Direction::UNKNOWN
        }
    }

    fn to_cdev(&self) -> u32 {
        match self {
            Direction::IN => GPIOHANDLE_REQUEST_INPUT,
            Direction::OUT => GPIOHANDLE_REQUEST_OUTPUT,
            _ => GPIOHANDLE_REQUEST_INPUT,
        }
    }
}

fn check_write_access() -> Result<(), Error> {
    let gpiochip_path = "/dev/gpiochip0";
    if !Path::new(gpiochip_path).exists() {
        return Err(Error::msg(
            "GPIO character device not found. This library requires a Jetson platform.",
        ));
    }

    if !Path::new(gpiochip_path).metadata().is_ok_and(|_m| {
        std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(gpiochip_path)
            .is_ok()
    }) {
        return Err(Error::msg(
            "The current user does not have permissions set to access the library functionalites. Please configure permissions or use the root user to run this. It is also possible that /dev/gpiochip0 does not exist. Please check if that file is present.",
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// GpioInner — mutable state protected by Mutex for thread safety
// ---------------------------------------------------------------------------

/// Mutable inner state of `GPIO`, protected by a `Mutex` for thread safety.
pub(crate) struct GpioInner {
    pub(crate) channel_data: HashMap<u32, ChannelInfo>,
    pub(crate) gpio_warnings: bool,
    pub(crate) gpio_mode: Option<Mode>,
    pub(crate) channel_configuration: HashMap<u32, Direction>,
    pub(crate) chip_fd_map: HashMap<String, File>,
    pub(crate) event_manager: Option<crate::gpio_event::EventManager>,
}

impl GpioInner {
    fn validate_mode_set(&self) -> Result<(), Error> {
        match self.gpio_mode {
            Some(_) => Ok(()),
            None => Err(Error::msg(
                "Please set pin numbering mode using GPIO.setmode(Mode::BOARD) or GPIO.setmode(Mode::BCM)",
            )),
        }
    }

    fn channel_to_info_lookup(
        &self,
        channel: u32,
        need_gpio: bool,
        need_pwm: bool,
    ) -> Result<&ChannelInfo, Error> {
        if !self.channel_data.contains_key(&channel) {
            return Err(Error::msg(format!(
                "The channel sent is invalid: {}",
                channel
            )));
        }

        let ch_info = self.channel_data.get(&channel).unwrap();

        if need_gpio && ch_info.gpio_chip.is_empty() {
            return Err(Error::msg(format!("Channel {} is not a GPIO", channel)));
        }

        if need_pwm && ch_info.pwm_chip_dir.is_none() {
            return Err(Error::msg(format!("Channel {} is not a PWM", channel)));
        }

        Ok(ch_info)
    }

    fn channel_to_info(
        &self,
        channel: u32,
        need_gpio: bool,
        need_pwm: bool,
    ) -> Result<&ChannelInfo, Error> {
        self.validate_mode_set()?;
        self.channel_to_info_lookup(channel, need_gpio, need_pwm)
    }

    fn channels_to_infos(
        &self,
        channels: Vec<u32>,
        need_gpio: bool,
        need_pwm: bool,
    ) -> Result<Vec<&ChannelInfo>, Error> {
        self.validate_mode_set()?;
        let mut ret: Vec<&ChannelInfo> = Vec::new();
        for channel in channels {
            ret.push(self.channel_to_info_lookup(channel, need_gpio, need_pwm)?);
        }
        Ok(ret)
    }

    fn app_channel_configuration(&self, ch_info: &ChannelInfo) -> Option<Direction> {
        self.channel_configuration.get(&ch_info.channel).copied()
    }

    fn do_one_channel(
        &mut self,
        ch_info: ChannelInfo,
        direction: u32,
        initial: Option<u8>,
        consumer: &str,
    ) -> Result<(), Error> {
        let chip_name = ch_info.gpio_chip.clone();
        let chip_fd = if !self.chip_fd_map.contains_key(&chip_name) {
            let fd = chip_open_by_label(&chip_name)?;
            self.chip_fd_map.insert(chip_name.clone(), fd);
            self.chip_fd_map.get(&chip_name).unwrap().try_clone()?
        } else {
            self.chip_fd_map.get(&chip_name).unwrap().try_clone()?
        };

        let chip_fd_raw = chip_fd.as_raw_fd();

        let mut request = request_handle(ch_info.line_offset, direction, initial, consumer)?;
        let line_handle = open_line(&mut request, &chip_fd)?;

        let mut ch_info = ch_info;
        ch_info.chip_fd = Some(chip_fd_raw);
        ch_info.line_handle = Some(line_handle);

        if self.gpio_warnings {
            if let Err(e) = check_pinmux(ch_info.reg_addr, direction, ch_info.channel) {
                eprintln!("Pinmux check warning: {}", e);
            }
        }

        self.channel_configuration
            .insert(ch_info.channel, Direction::from_cdev(direction as i32));
        self.channel_data.insert(ch_info.channel, ch_info);
        Ok(())
    }

    fn cleanup_one(&mut self, ch_info: ChannelInfo) {
        let app_cfg = self.channel_configuration.get(&ch_info.channel).copied();
        match app_cfg {
            Some(Direction::HardPwm) => {
                pwm_disable(&ch_info).ok();
                pwm_unexport(&ch_info).ok();
            }
            _ => {
                if let Some(ref mut manager) = self.event_manager {
                    manager.event_cleanup(&ch_info.gpio_chip, ch_info.channel);
                }
                if let Some(line_handle) = ch_info.line_handle {
                    let _ = close_line(Some(line_handle));
                }
            }
        }
        self.channel_configuration.remove(&ch_info.channel);

        if let Some(chip_fd) = self.chip_fd_map.remove(&ch_info.gpio_chip) {
            let _ = close_chip(Some(chip_fd));
        }
    }

    fn cleanup_all(&mut self) -> Result<(), Error> {
        let ch_infos_to_cleanup: Vec<ChannelInfo> = self.channel_data.values().cloned().collect();
        for ch_info in ch_infos_to_cleanup {
            self.cleanup_one(ch_info);
        }

        for (_chip_name, chip_fd) in self.chip_fd_map.drain() {
            let _ = close_chip(Some(chip_fd));
        }

        self.gpio_mode = None;

        Ok(())
    }

    fn setup_single_out(
        &mut self,
        ch_info: ChannelInfo,
        initial: Option<Level>,
        consumer: &str,
    ) -> Result<(), Error> {
        let initial_value = initial.map(|l| l as u8);
        self.do_one_channel(ch_info, Direction::OUT.to_cdev(), initial_value, consumer)
    }

    fn setup_single_in(&mut self, ch_info: ChannelInfo, consumer: &str) -> Result<(), Error> {
        self.do_one_channel(ch_info, Direction::IN.to_cdev(), None, consumer)
    }
}

// ---------------------------------------------------------------------------
// GPIO — thread-safe public struct
// ---------------------------------------------------------------------------

/// A thread-safe struct that holds state information about the GPIO pins.
///
/// Public fields:
/// * `model` - The model of the Jetson board
/// * `jetson_info` - A `JetsonInfo` struct that holds information about the Jetson board
///
/// # Example
///
/// ```rust
/// use jetsongpio::GPIO;
///
/// let gpio = GPIO::new();
/// ```
pub struct GPIO {
    /// Model name of the Jetson board (immutable after construction).
    pub model: String,
    /// Board identification info (immutable after construction).
    pub jetson_info: JetsonInfo,
    /// Static pin mapping tables keyed by Mode (immutable after construction;
    /// only read via `channel_data_by_mode.get()` — no mutation).
    pub channel_data_by_mode: HashMap<Mode, HashMap<u32, ChannelInfo>>,
    /// Mutable state protected by a Mutex for thread safety.
    inner: Mutex<GpioInner>,
}

// Safety: All mutable state is inside `Mutex<GpioInner>` which provides safe
// interior mutability. The immutable fields (`model`, `jetson_info`,
// `channel_data_by_mode`) are never modified after construction.
// `channel_data_by_mode` contains `ChannelInfo` which has an `f_duty_cycle:
// Option<File>` field, but those files are always `None` in the static tables
// (see `Clone` impl in `gpio_pin_data.rs`). The `File` inside `chip_fd_map`
// and `f_duty_cycle` (when present) are only accessed while holding the Mutex,
// so concurrent access is safe.
unsafe impl Sync for GPIO {}

impl GPIO {
    /// Creates a new `GPIO` object.
    ///
    /// Calling this function will automatically populate the `model` and `jetson_info` fields.
    pub fn new() -> Self {
        let (model, jetson_info, channel_data_by_mode) = get_data();

        GPIO {
            model,
            jetson_info,
            channel_data_by_mode,
            inner: Mutex::new(GpioInner {
                channel_data: HashMap::new(),
                gpio_warnings: true,
                gpio_mode: None,
                channel_configuration: HashMap::new(),
                chip_fd_map: HashMap::new(),
                event_manager: None,
            }),
        }
    }

    /// Lock the inner state and return a MutexGuard.
    pub(crate) fn inner(&self) -> std::sync::MutexGuard<'_, GpioInner> {
        self.inner.lock().unwrap()
    }

    /// Enable or disable warnings during setup and cleanup.
    ///
    /// # Arguments
    ///
    /// * `warnings` - `true` to enable warnings, `false` to disable warnings
    pub fn setwarnings(&self, warnings: bool) {
        self.inner().gpio_warnings = warnings;
    }

    /// Sets the pin numbering mode.
    ///
    /// Possible mode values are
    /// * `Mode::BOARD`
    /// * `Mode::BCM`
    ///
    /// # Arguments
    ///
    /// * `mode` - The pin numbering mode to use
    pub fn setmode(&self, mode: Mode) -> Result<(), Error> {
        let mut inner = self.inner();

        if let Some(ref current_mode) = inner.gpio_mode {
            if *current_mode != mode {
                return Err(Error::msg("A different mode has already been set!"));
            }
        }

        if !mode.is_valid() {
            return Err(Error::msg("An invalid mode was passed to setmode!"));
        }

        inner.channel_data = self.channel_data_by_mode.get(&mode).unwrap().clone();
        inner.gpio_mode = Some(mode);

        Ok(())
    }

    /// Returns the currently set pin numbering mode as an `Option<Mode>`.
    pub fn getmode(&self) -> Option<Mode> {
        self.inner().gpio_mode
    }

    /// Setup a channel or list of channels with a direction and (optional) pull/up down control and (optional) initial value.
    ///
    /// # Arguments
    ///
    /// * `channels` - A list of channels to setup.
    /// * `direction` - `Direction::IN` or `Direction::OUT`
    /// * `initial` - An optional initial level for an output channel.
    /// * `consumer` - An optional consumer label for the GPIO line (default: "jetsongpio-rs").
    ///
    /// # Example
    ///
    /// ```rust
    /// use jetsongpio::{GPIO, Direction, Mode};
    ///
    /// let gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![7], Direction::OUT, None, None).unwrap();
    /// ```
    pub fn setup(
        &self,
        channels: Vec<u32>,
        direction: Direction,
        initial: Option<Level>,
        consumer: Option<&str>,
    ) -> Result<(), Error> {
        check_write_access()?;

        let mut inner = self.inner();
        let ch_infos = inner.channels_to_infos(channels, true, false)?;

        if !direction.is_valid() {
            return Err(Error::msg("An invalid direction was passed to setup()"));
        }

        let consumer = consumer.unwrap_or("jetsongpio-rs");

        let ch_infos_owned: Vec<ChannelInfo> = ch_infos.iter().map(|&ch| ch.clone()).collect();

        for ch_info in ch_infos_owned.iter() {
            if inner.channel_configuration.contains_key(&ch_info.channel) {
                inner.cleanup_one(ch_info.clone());
            }
        }

        match direction {
            Direction::OUT => {
                for ch_info in ch_infos_owned {
                    inner.setup_single_out(ch_info, initial, consumer)?;
                }
            }
            Direction::IN => {
                if initial.is_some() {
                    return Err(Error::msg("initial parameter is not valid for inputs"));
                }
                for ch_info in ch_infos_owned {
                    inner.setup_single_in(ch_info, consumer)?;
                }
            }
            _ => {
                return Err(Error::msg("Unsupported direction for setup()"));
            }
        }

        Ok(())
    }

    /// Cleans up channels at the end of the program.
    ///
    /// # Arguments
    ///
    /// * `channels` - An optional list of channels to cleanup. If no channel is provided, all channels are cleaned.
    pub fn cleanup(&self, channels: Option<Vec<u32>>) -> Result<(), Error> {
        let mut inner = self.inner();

        if inner.gpio_mode.is_none() {
            if inner.gpio_warnings {
                println!(
                    "No channels have been set up yet - nothing to clean up! Try cleaning up at the end of your program instead!"
                );
            }
            return Ok(());
        }

        if channels.is_none() {
            inner.cleanup_all()?;
            return Ok(());
        }

        let ch_infos = inner.channels_to_infos(channels.unwrap(), false, false)?;
        let channels_to_cleanup: Vec<u32> = ch_infos
            .iter()
            .filter_map(|ch_info| {
                if inner.channel_configuration.contains_key(&ch_info.channel) {
                    Some(ch_info.channel)
                } else {
                    None
                }
            })
            .collect();

        for channel in channels_to_cleanup {
            if let Some(ch_info) = inner.channel_data.get(&channel).cloned() {
                inner.cleanup_one(ch_info);
            }
        }

        Ok(())
    }

    /// Returns the current value of the specified channel.
    ///
    /// Return either `Level::HIGH` or `Level::LOW`.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to read from.
    pub fn input(&self, channel: u32) -> Result<Level, Error> {
        // Acquire lock only to read state, release before ioctl.
        let line_handle = {
            let inner = self.inner();
            let ch_info = inner.channel_to_info(channel, true, false)?;

            let app_cfg = inner.app_channel_configuration(ch_info);
            if app_cfg.is_none() || ![Direction::IN, Direction::OUT].contains(&app_cfg.unwrap()) {
                return Err(Error::msg("You must setup() the GPIO channel first"));
            }

            ch_info
                .line_handle
                .ok_or_else(|| Error::msg("GPIO line handle not found"))?
        }; // lock released here

        let value = get_value(line_handle)?;

        match value {
            0 => Ok(Level::LOW),
            _ => Ok(Level::HIGH),
        }
    }

    /// Writes a value to channels.
    ///
    /// # Arguments
    ///
    /// * `channels` - A list of channels to write to.
    /// * `values` - A list of values to write to the channels. Must be either HIGH or LOW.
    ///
    /// # Example
    /// ```rust
    /// use jetsongpio::{GPIO, Direction, Level, Mode};
    ///
    /// let gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![7], Direction::OUT, None, None).unwrap();
    /// gpio.output(vec![7], vec![Level::HIGH]).unwrap();
    /// ```
    pub fn output(&self, channels: Vec<u32>, values: Vec<Level>) -> Result<(), Error> {
        // Acquire lock only to read state, release before ioctl.
        let ops: Vec<(i32, u8)> = {
            let inner = self.inner();
            let ch_infos = inner.channels_to_infos(channels, true, false)?;

            if values.len() != ch_infos.len() {
                return Err(Error::msg("Number of values != number of channels"));
            }

            for ch_info in &ch_infos {
                let app_cfg = inner.app_channel_configuration(ch_info);
                if app_cfg.is_none() || app_cfg.unwrap() != Direction::OUT {
                    return Err(Error::msg(
                        "The GPIO channel has not been set up as an OUTPUT",
                    ));
                }
            }

            ch_infos
                .iter()
                .zip(values.iter())
                .map(|(ch_info, value)| {
                    let line_handle = ch_info
                        .line_handle
                        .ok_or_else(|| Error::msg("GPIO line handle not found"))?;
                    Ok((line_handle, *value as u8))
                })
                .collect::<Result<Vec<_>, Error>>()?
        }; // lock released here

        for (line_handle, value) in ops {
            set_value(line_handle, value)?;
        }

        Ok(())
    }

    /// Returns the currently set function of the specified channel.
    ///
    /// Returns either `Direction::IN`, `Direction::OUT`, or `Direction::UNKNOWN`.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to check.
    pub fn gpio_function(&self, channel: u32) -> Result<Direction, Error> {
        let inner = self.inner();
        let ch_info = inner.channel_to_info(channel, false, false)?;
        let func = inner.app_channel_configuration(ch_info);
        Ok(func.unwrap_or(Direction::UNKNOWN))
    }
}

// ---------------------------------------------------------------------------
// PWM sysfs helpers (private, mirror Python gpio.py:136-210)
// ---------------------------------------------------------------------------

fn pwm_path(ch_info: &ChannelInfo) -> String {
    format!(
        "{}/pwm{}",
        ch_info.pwm_chip_dir.as_deref().unwrap_or(""),
        ch_info.pwm_id.unwrap_or(0)
    )
}

fn pwm_export_path(ch_info: &ChannelInfo) -> String {
    format!("{}/export", ch_info.pwm_chip_dir.as_deref().unwrap_or(""))
}

fn pwm_unexport_path(ch_info: &ChannelInfo) -> String {
    format!("{}/unexport", ch_info.pwm_chip_dir.as_deref().unwrap_or(""))
}

fn pwm_period_path(ch_info: &ChannelInfo) -> String {
    format!("{}/period", pwm_path(ch_info))
}

fn pwm_duty_cycle_path(ch_info: &ChannelInfo) -> String {
    format!("{}/duty_cycle", pwm_path(ch_info))
}

fn pwm_enable_path(ch_info: &ChannelInfo) -> String {
    format!("{}/enable", pwm_path(ch_info))
}

fn pwm_export(ch_info: &mut ChannelInfo) -> Result<(), Error> {
    let pwm_dir = pwm_path(ch_info);
    if !Path::new(&pwm_dir).exists() {
        let export_path = pwm_export_path(ch_info);
        let mut f = std::fs::OpenOptions::new().write(true).open(&export_path)?;
        write!(f, "{}", ch_info.pwm_id.unwrap_or(0))?;
    }

    // Wait for enable file to become readable (mirrors Python time.sleep loop)
    let enable_path = pwm_enable_path(ch_info);
    loop {
        if std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&enable_path)
            .is_ok()
        {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let duty_path = pwm_duty_cycle_path(ch_info);
    let f_duty = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&duty_path)?;
    ch_info.f_duty_cycle = Some(f_duty);

    Ok(())
}

fn pwm_unexport(ch_info: &ChannelInfo) -> Result<(), Error> {
    let unexport_path = pwm_unexport_path(ch_info);
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .open(&unexport_path)?;
    write!(f, "{}", ch_info.pwm_id.unwrap_or(0))?;
    Ok(())
}

fn pwm_set_period(ch_info: &ChannelInfo, period_ns: u64) -> Result<(), Error> {
    let path = pwm_period_path(ch_info);
    let mut f = std::fs::OpenOptions::new().write(true).open(&path)?;
    write!(f, "{}", period_ns)?;
    Ok(())
}

fn pwm_set_duty_cycle(f_duty: &mut File, duty_cycle_ns: u64) -> Result<(), Error> {
    if duty_cycle_ns == 0 {
        f_duty.rewind()?;
        let mut cur = String::new();
        f_duty.read_to_string(&mut cur)?;
        if cur.trim() == "0" {
            return Ok(());
        }
    }
    f_duty.rewind()?;
    write!(f_duty, "{}", duty_cycle_ns)?;
    f_duty.flush()?;
    Ok(())
}

fn pwm_enable(ch_info: &ChannelInfo) -> Result<(), Error> {
    let path = pwm_enable_path(ch_info);
    let mut f = std::fs::OpenOptions::new().write(true).open(&path)?;
    write!(f, "1")?;
    Ok(())
}

fn pwm_disable(ch_info: &ChannelInfo) -> Result<(), Error> {
    let path = pwm_enable_path(ch_info);
    let mut f = std::fs::OpenOptions::new().write(true).open(&path)?;
    write!(f, "0")?;
    Ok(())
}

// ---------------------------------------------------------------------------
// PWM struct (mirrors Python gpio.py:542-624)
// ---------------------------------------------------------------------------

/// Hardware PWM controller for a GPIO channel.
///
/// # Example
///
/// ```rust
/// use jetsongpio::{GPIO, Mode, PWM};
///
/// let gpio = GPIO::new();
/// gpio.setmode(Mode::BCM).unwrap();
/// let mut pwm = PWM::new(&gpio, 18, 50.0).unwrap();
/// pwm.start(25.0).unwrap();
/// // ... change duty cycle ...
/// pwm.stop().unwrap();
/// gpio.cleanup(Some(vec![18])).unwrap();
/// ```
pub struct PWM {
    ch_info: ChannelInfo,
    frequency_hz: f64,
    duty_cycle_percent: f64,
    period_ns: u64,
    duty_cycle_ns: u64,
    started: bool,
}

impl PWM {
    /// Create a new PWM instance for the given channel and frequency.
    ///
    /// # Arguments
    ///
    /// * `gpio` - The GPIO instance (must have mode set)
    /// * `channel` - The channel number in the current pin numbering mode
    /// * `frequency_hz` - The PWM frequency in Hz
    pub fn new(gpio: &GPIO, channel: u32, frequency_hz: f64) -> Result<Self, Error> {
        // Hold the lock for the entire setup to prevent TOCTOU races.
        let mut inner = gpio.inner();

        inner.validate_mode_set()?;

        let ch_info = inner.channel_to_info_lookup(channel, false, true)?;
        let mut ch_info = ch_info.clone();

        // Check existing configuration
        let app_cfg = inner.app_channel_configuration(&ch_info);
        if app_cfg == Some(Direction::HardPwm) {
            return Err(Error::msg("Can't create duplicate PWM objects"));
        }
        // If channel is set up as GPIO, clean it up first
        if app_cfg == Some(Direction::OUT) || app_cfg == Some(Direction::IN) {
            inner.cleanup_one(ch_info.clone());
        }

        // Export the PWM (sysfs I/O — we hold the lock for simplicity; the
        // export operation is fast enough that this won't cause contention
        // issues in practice).
        pwm_export(&mut ch_info)?;

        if let Some(ref mut f_duty) = ch_info.f_duty_cycle {
            pwm_set_duty_cycle(f_duty, 0)?;
        }

        let mut pwm = PWM {
            ch_info,
            frequency_hz: -frequency_hz,
            duty_cycle_percent: 0.0,
            period_ns: 0,
            duty_cycle_ns: 0,
            started: false,
        };
        pwm.reconfigure(frequency_hz, 0.0, false)?;

        inner
            .channel_configuration
            .insert(channel, Direction::HardPwm);

        Ok(pwm)
    }

    /// Start PWM output with the given duty cycle percentage (0.0 - 100.0).
    pub fn start(&mut self, duty_cycle_percent: f64) -> Result<(), Error> {
        self.reconfigure(self.frequency_hz, duty_cycle_percent, true)
    }

    /// Stop PWM output.
    pub fn stop(&mut self) -> Result<(), Error> {
        if !self.started {
            return Ok(());
        }
        pwm_disable(&self.ch_info)?;
        self.started = false;
        Ok(())
    }

    /// Change the duty cycle percentage (0.0 - 100.0).
    pub fn set_duty_cycle(&mut self, duty_cycle_percent: f64) -> Result<(), Error> {
        self.reconfigure(self.frequency_hz, duty_cycle_percent, false)
    }

    /// Change the frequency in Hz.
    pub fn set_frequency(&mut self, frequency_hz: f64) -> Result<(), Error> {
        self.reconfigure(frequency_hz, self.duty_cycle_percent, false)
    }

    /// Returns true if `cleanup()` already tore this PWM down via the parent
    /// `GPIO`. Used by `Drop` so it does not attempt to unexport twice.
    fn already_cleaned_up(&self) -> bool {
        !Path::new(&pwm_path(&self.ch_info)).exists()
    }

    fn reconfigure(
        &mut self,
        frequency_hz: f64,
        duty_cycle_percent: f64,
        start: bool,
    ) -> Result<(), Error> {
        if !(0.0..=100.0).contains(&duty_cycle_percent) {
            return Err(Error::msg(
                "duty_cycle_percent must be between 0.0 and 100.0",
            ));
        }

        let freq_change = start || (frequency_hz != self.frequency_hz);
        let stop = self.started && freq_change;

        if stop {
            self.started = false;
            pwm_disable(&self.ch_info)?;
        }

        if freq_change {
            self.frequency_hz = frequency_hz;
            self.period_ns = (1_000_000_000.0 / frequency_hz) as u64;
            if let Some(ref mut f_duty) = self.ch_info.f_duty_cycle {
                pwm_set_duty_cycle(f_duty, 0)?;
            }
            pwm_set_period(&self.ch_info, self.period_ns)?;
        }

        self.duty_cycle_percent = duty_cycle_percent;
        self.duty_cycle_ns = (self.period_ns as f64 * (duty_cycle_percent / 100.0)) as u64;

        if let Some(ref mut f_duty) = self.ch_info.f_duty_cycle {
            pwm_set_duty_cycle(f_duty, self.duty_cycle_ns)?;
        }

        if stop || start {
            pwm_enable(&self.ch_info)?;
            self.started = true;
        }

        Ok(())
    }
}

impl Drop for PWM {
    /// Mirrors Python `PWM.__del__` — disable the PWM and unexport the sysfs
    /// entry. Note: unlike Python (which uses module-level singletons), this
    /// Rust port cannot reach back into the parent `GPIO` to clear
    /// `channel_configuration` from `Drop`. Callers should still invoke
    /// `GPIO::cleanup` to fully reset that bookkeeping.
    fn drop(&mut self) {
        if self.already_cleaned_up() {
            return;
        }
        let _ = pwm_disable(&self.ch_info);
        let _ = pwm_unexport(&self.ch_info);
    }
}
