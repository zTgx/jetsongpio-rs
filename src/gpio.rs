use anyhow::Error;
use std::{
    collections::HashMap,
    fs,
    io::{Read, Seek, Write},
    path::Path,
    thread,
    time::Duration,
};

use crate::gpio_pin_data::{get_data, ChannelInfo, JetsonInfo, Mode};

static SYSFS_ROOT: &str = "/sys/class/gpio";

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
/// use jetson_gpio::{GPIO, Level, Direction, Mode};
///
/// let mut gpio = GPIO::new();
/// gpio.setmode(Mode::BOARD).unwrap();
///
/// gpio.setup(vec![7, 11], Direction::OUT, None).unwrap();
/// gpio.output(vec![7, 11], vec![Level::HIGH, Level::LOW]).unwrap();
/// ```
#[derive(PartialEq, Clone, Copy)]
pub enum Level {
    LOW = 0,
    HIGH = 1,
}

/// Specifies the GPIO pin direction.
///
/// * `IN` - Input
/// * `OUT` - Output
/// * `HARD_PWM` - Hardware PWM output
/// * `UNKNOWN` - Unknown direction for GPIOs that are not yet setup
///
/// # Example
///
/// When setting up a GPIO pin, you must specify the direction. For example, to
/// set up GPIO pin 7 as an output:
///
/// ```rust
/// use jetson_gpio::{GPIO, Direction};
///
/// let mut gpio = GPIO::new();
///
/// gpio.setup(vec![7], Direction::OUT, None).unwrap();
/// ```
#[derive(PartialEq, Clone, Copy)]
pub enum Direction {
    UNKNOWN = -1,
    OUT = 0,
    IN = 1,
    HARD_PWM = 43,
}

impl Direction {
    pub fn is_valid(&self) -> bool {
        matches!(self, Direction::OUT | Direction::IN | Direction::HARD_PWM)
    }
}

fn check_write_access() -> Result<(), Error> {
    let export_path = format!("{}/export", SYSFS_ROOT);
    let unexport_path = format!("{}/unexport", SYSFS_ROOT);

    let export_metadata = fs::metadata(&export_path).unwrap();
    let unexport_metadata = fs::metadata(&unexport_path).unwrap();

    let export_permissions = export_metadata.permissions();
    let unexport_permissions = unexport_metadata.permissions();

    if !export_permissions.readonly() && !unexport_permissions.readonly() {
        Ok(())
    } else {
        Err(Error::msg("You do not have write access to the GPIO sysfs interface."))
    }
}

fn sysfs_channel_configuration(ch_info: ChannelInfo) -> Option<Direction> {
    // """Return the current configuration of a channel as reported by sysfs. Any
    // of IN, OUT, PWM, or None may be returned."""

    if let Some(pwm_chip_dir) = &ch_info.pwm_chip_dir {
        let pwm_dir = format!("{}/pwm{}", pwm_chip_dir, ch_info.pwm_id?);
        if Path::new(&pwm_dir).exists() {
            return Some(Direction::HARD_PWM);
        }
    }

    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    if !Path::new(&gpio_dir).exists() {
        return None;
    }

    let direction_path = format!("{}/direction", gpio_dir);
    let gpio_direction = fs::read_to_string(&direction_path).unwrap_or_default();
    match gpio_direction.as_str() {
        "in" => Some(Direction::IN),
        "out" => Some(Direction::OUT),
        _ => None,
    }
}

fn export_gpio(ch_info: &ChannelInfo) {
    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    if !Path::new(&gpio_dir).exists() {
        let export_path = format!("{}/export", SYSFS_ROOT);
        let mut f_export = fs::OpenOptions::new()
            .write(true)
            .open(&export_path)
            .unwrap();
        f_export
            .write_all(ch_info.global_gpio.to_string().as_bytes())
            .unwrap();
    }

    let value_path = format!("{}/value", gpio_dir);
    while !Path::new(&value_path).exists() {
        thread::sleep(Duration::from_millis(10));
    }
}

fn unexport_gpio(ch_info: &ChannelInfo) {
    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    if Path::new(&gpio_dir).exists() {
        let unexport_path = format!("{}/unexport", SYSFS_ROOT);
        let mut f_unexport = fs::OpenOptions::new()
            .write(true)
            .open(&unexport_path)
            .unwrap();
        f_unexport
            .write_all(ch_info.global_gpio.to_string().as_bytes())
            .unwrap();
    }
}

fn write_sysfs_file(path: &str, data: &[u8]) {
    let mut f = fs::OpenOptions::new().write(true).open(path).unwrap();
    f.write_all(data).unwrap();
}

fn write_direction(ch_info: ChannelInfo, direction: &str) {
    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    write_sysfs_file(&format!("{}/direction", gpio_dir), direction.as_bytes());
}

fn write_value(ch_info: &ChannelInfo, value: &str) {
    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    write_sysfs_file(&format!("{}/value", gpio_dir), value.as_bytes());
}

fn read_value(ch_info: ChannelInfo) -> String {
    let gpio_dir = format!("{}/{}", SYSFS_ROOT, ch_info.global_gpio_name);
    let value_path = format!("{}/value", gpio_dir);
    fs::read_to_string(&value_path).unwrap_or_default()
}

fn output_one(ch_info: &ChannelInfo, value: Level) {
    let value_str = match value {
        Level::HIGH => "1",
        Level::LOW => "0",
    };

    write_value(ch_info, value_str);
}

/// A public struct that holds state information about the GPIO pins.
///
/// Public fields:
/// * `model` - The model of the Jetson board
/// * `jetson_info` - A `JetsonInfo` struct that holds information about the Jetson board
///
/// # Example
///
/// ```rust
/// use jetson_gpio::GPIO;
///
/// let gpio = GPIO::new();
/// ```
pub struct GPIO {
    pub model: String,
    pub jetson_info: JetsonInfo,
    channel_data_by_mode: HashMap<Mode, HashMap<u32, ChannelInfo>>,

    // # Dictionary objects used as lookup tables for pin to linux gpio mapping
    channel_data: HashMap<u32, ChannelInfo>,

    gpio_warnings: bool,
    gpio_mode: Option<Mode>,
    channel_configuration: HashMap<u32, Direction>,
}

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

            channel_data: HashMap::new(),

            gpio_warnings: true,
            gpio_mode: None,
            channel_configuration: HashMap::new(),
        }
    }

    /// Enable or disable warnings during setup and cleanup.
    ///
    /// # Arguments
    ///
    /// * `warnings` - `true` to enable warnings, `false` to disable warnings
    pub fn setwarnings(&mut self, warnings: bool) {
        self.gpio_warnings = warnings;
    }

    /// Sets the pin mumbering mode.
    ///
    /// Possible mode values are
    /// * `Mode::BOARD`
    /// * `Mode::BCM`
    /// * `Mode::TEGRA_SOC`
    /// * `Mode::CVM`
    ///
    /// # Arguments
    ///
    /// * `mode` - The pin numbering mode to use
    pub fn setmode(&mut self, mode: Mode) -> Result<(), Error> {
        // check if a different mode has been set already
        if let Some(current_mode) = self.gpio_mode {
            if current_mode != mode {
                return Err(Error::msg("A different mode has already been set!"));
            }
        }

        // check if mode parameter is valid
        if !mode.is_valid() {
            return Err(Error::msg("An invalid mode was passed to setmode!"));
        }

        self.channel_data = self.channel_data_by_mode.get(&mode).unwrap().clone();
        self.gpio_mode = Some(mode);

        Ok(())
    }

    /// Returns the currently set pin numbering mode as an `Option<String>`.
    pub fn getmode(&self) -> Option<String> {
        match self.gpio_mode {
            Some(mode) => Some(String::from(mode.to_str())),
            None => None,
        }
    }

    fn validate_mode_set(&self) -> Result<(), Error> {
        match self.gpio_mode {
            Some(_) => Ok(()),
            None => Err(Error::msg("Please set pin numbering mode using GPIO.setmode(Mode::BOARD), GPIO.setmode(Mode::BCM), GPIO.setmode(Mode::TEGRA_SOC) or GPIO.setmode(Mode::CVM)")),
        }
    }

    fn channel_to_info_lookup(
        &self,
        channel: u32,
        need_gpio: bool,
        need_pwm: bool,
    ) -> Result<ChannelInfo, Error> {
        if !self.channel_data.contains_key(&channel) {
            return Err(Error::msg(format!(
                "The channel sent is invalid: {}",
                channel
            )));
        }

        let ch_info = self.channel_data.get(&channel).unwrap().clone();

        if need_gpio && ch_info.gpio_chip_dir == "" {
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
    ) -> Result<ChannelInfo, Error> {
        self.validate_mode_set()?;
        self.channel_to_info_lookup(channel, need_gpio, need_pwm)
    }

    fn channels_to_infos(
        &self,
        channels: Vec<u32>,
        need_gpio: bool,
        need_pwm: bool,
    ) -> Result<Vec<ChannelInfo>, Error> {
        self.validate_mode_set()?;
        let mut ret: Vec<ChannelInfo> = Vec::new();
        for channel in channels {
            ret.push(self.channel_to_info_lookup(channel, need_gpio, need_pwm)?);
        }

        Ok(ret)
    }

    fn app_channel_configuration(&self, ch_info: &ChannelInfo) -> Option<Direction> {
        // """Return the current configuration of a channel as requested by this
        // module in this process. Any of IN, OUT, or None may be returned."""

        self.channel_configuration.get(&ch_info.channel).copied()
    }

    fn cleanup_one(&mut self, ch_info: ChannelInfo) {
        match self.channel_configuration.get(&ch_info.channel) {
            Some(direction) => {
                if direction == &Direction::HARD_PWM {
                    // _disable_pwm(ch_info);
                    // _unexport_pwm(ch_info);
                } else {
                    // event::event_cleanup(ch_info.gpio, ch_info.gpio_name);
                    unexport_gpio(&ch_info);
                }
            }
            None => {}
        }

        self.channel_configuration.remove(&ch_info.channel);
    }

    fn cleanup_all(&mut self) -> Result<(), Error> {
        for (channel, _) in self.channel_configuration.clone().iter() {
            let ch_info = self.channel_to_info(*channel, false, false)?;
            self.cleanup_one(ch_info);
        }

        self.gpio_mode = None;

        Ok(())
    }

    fn setup_single_out(&mut self, ch_info: ChannelInfo, initial: Option<Level>) {
        export_gpio(&ch_info);
        write_direction(ch_info.clone(), "out");

        if let Some(level) = initial {
            output_one(&ch_info, level);
        }

        self.channel_configuration
            .insert(ch_info.channel, Direction::OUT);
    }

    fn setup_single_in(&mut self, ch_info: ChannelInfo) {
        export_gpio(&ch_info);
        write_direction(ch_info.clone(), "in");

        self.channel_configuration
            .insert(ch_info.channel, Direction::IN);
    }

    /// Setup a channel or list of channels with a direction and (optional) pull/up down control and (optional) initial value.
    ///
    /// # Arguments
    ///
    /// * `channels` - A list of channels to setup.
    /// * `direction` - `Level::IN` or `Level::OUT`
    /// * `initial` - An optional initial level for an output channel.
    ///
    /// # Example
    ///
    /// ```rust
    /// use jetson_gpio::{GPIO, Direction, Mode};
    ///
    /// let mut gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![7], Direction::OUT, None).unwrap();
    /// ```
    pub fn setup(&mut self, channels: Vec<u32>, direction: Direction, initial: Option<Level>) -> Result<(), Error> {
        check_write_access()?;

        // if pull_up_down in setup.__defaults__:
        //     pull_up_down_explicit = False
        //     pull_up_down = pull_up_down.val
        // else:
        //     pull_up_down_explicit = True

        let ch_infos = self.channels_to_infos(channels, true, false)?;

        // check direction is valid
        if !direction.is_valid() {
            return Err(Error::msg("An invalid direction was passed to setup()"));
        }

        // // check if pullup/down is used with output
        // if direction == OUT and pull_up_down != PUD_OFF:
        //     raise ValueError("pull_up_down parameter is not valid for outputs")

        // // check if pullup/down value is specified and/or valid
        // if pull_up_down_explicit:
        //     warnings.warn("Jetson.GPIO ignores setup()'s pull_up_down parameter")
        // if (pull_up_down != PUD_OFF and pull_up_down != PUD_UP and
        //         pull_up_down != PUD_DOWN):
        //     raise ValueError("Invalid value for pull_up_down; should be one of"
        //                      "PUD_OFF, PUD_UP or PUD_DOWN")

        if self.gpio_warnings {
            for ch_info in ch_infos.clone() {
                let sysfs_cfg = sysfs_channel_configuration(ch_info.clone());
                let app_cfg = self.app_channel_configuration(&ch_info);

                // warn if channel has been setup external to current program
                if app_cfg.is_none() && sysfs_cfg.is_some() {
                    println!("This channel is already in use, continuing anyway. Use GPIO.setwarnings(False) to disable warnings");
                }
            }
        }

        // cleanup if the channel is already setup
        for ch_info in ch_infos.clone() {
            // if ch_info.channel in channel_configuration:
            //     cleanup_one(ch_info)
            if self.channel_configuration.contains_key(&ch_info.channel) {
                self.cleanup_one(ch_info.clone());
            }
        }

        match direction {
            Direction::OUT => {
                for ch_info in &ch_infos {
                    self.setup_single_out(ch_info.clone(), initial.clone());
                }
            }
            _ => {
                if initial.is_some() {
                    return Err(Error::msg("initial parameter is not valid for inputs"));
                }
                for ch_info in &ch_infos {
                    self.setup_single_in(ch_info.clone());
                }
            }
        }

        Ok(())
    }

    /// Cleans up channels at the end of the program.
    ///
    /// # Arguments
    ///
    /// * `channels` - An optional list of channels to cleanup. If no channel is provided, all channels are cleaned.
    pub fn cleanup(&mut self, channels: Option<Vec<u32>>) -> Result<(), Error> {
        // warn if no channel is setup
        if self.gpio_mode.is_none() {
            if self.gpio_warnings {
                println!("No channels have been set up yet - nothing to clean up! Try cleaning up at the end of your program instead!");
            }
            return Ok(());
        }

        // clean all channels if no channel param provided
        if channels.is_none() {
            self.cleanup_all()?;
            return Ok(());
        }

        let ch_infos = self.channels_to_infos(channels.unwrap(), false, false)?;
        for ch_info in ch_infos {
            if self.channel_configuration.contains_key(&ch_info.channel) {
                self.cleanup_one(ch_info);
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
        let ch_info = self.channel_to_info(channel, true, false)?;

        let app_cfg = self.app_channel_configuration(&ch_info);
        if app_cfg.is_none() || ![Direction::IN, Direction::OUT].contains(&app_cfg.unwrap()) {
            return Err(Error::msg("You must setup() the GPIO channel first"));
        }

        match read_value(ch_info.clone()).as_str() {
            "0" => Ok(Level::LOW),
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
    /// use jetson_gpio::{GPIO, Direction, Level, Mode};
    ///
    /// let mut gpio = GPIO::new();
    /// gpio.setmode(Mode::BOARD).unwrap();
    /// gpio.setup(vec![7], Direction::OUT, None).unwrap();
    /// gpio.output(vec![7], vec![Level::HIGH]).unwrap();
    /// ```
    pub fn output(&self, channels: Vec<u32>, values: Vec<Level>) -> Result<(), Error> {
        let ch_infos = self.channels_to_infos(channels.clone(), true, false)?;

        if values.len() != ch_infos.len() {
            return Err(Error::msg("Number of values != number of channels"));
        }

        // check that channels have been set as output
        for ch_info in &ch_infos {
            let app_cfg = self.app_channel_configuration(ch_info);
            if app_cfg.is_none() || app_cfg.unwrap() != Direction::OUT {
                return Err(Error::msg("The GPIO channel has not been set up as an OUTPUT"));
            }
        }

        for (ch_info, value) in ch_infos.iter().zip(values.iter()) {
            output_one(ch_info, value.clone());
        }

        Ok(())
    }
}