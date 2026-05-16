//! GPIO Character Device API
//!
//! This module provides interface to GPIO controller using character device API.
//! It replaces the deprecated sysfs interface with direct ioctl operations.
//!
//! # File operations
//! - open, close, ioctl operations for GPIO chips and lines
//!
//! # Example
//!
//! ```ignore
//! use jetsongpio::gpio_cdev::*;
//!
//! // Open a GPIO chip by label
//! let chip_fd = chip_open_by_label("tegra234-gpio")?;
//! ```
use anyhow::{Error, Result};
use std::ffi::CStr;
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;

/// GPIO character device constants
pub const GPIOHANDLE_REQUEST_INPUT: u32 = 0x1;
pub const GPIOHANDLE_REQUEST_OUTPUT: u32 = 0x2;

pub const GPIOEVENT_REQUEST_RISING_EDGE: u32 = 0x1;
pub const GPIOEVENT_REQUEST_FALLING_EDGE: u32 = 0x2;
pub const GPIOEVENT_REQUEST_BOTH_EDGES: u32 = 0x3;

// ioctl codes (from linux/gpio.h)
pub const GPIO_GET_CHIPINFO_IOCTL: u32 = 0x8044B401;
pub const GPIO_GET_LINEINFO_IOCTL: u32 = 0xC048B402;
pub const GPIO_GET_LINEHANDLE_IOCTL: u32 = 0xC16CB403;
pub const GPIOHANDLE_GET_LINE_VALUES_IOCTL: u32 = 0xC040B408;
pub const GPIOHANDLE_SET_LINE_VALUES_IOCTL: u32 = 0xC040B409;
pub const GPIO_GET_LINEEVENT_IOCTL: u32 = 0xC030B404;

/// Information about a GPIO chip
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GpioChipInfo {
    pub name: [u8; 32],
    pub label: [u8; 32],
    pub lines: u32,
}

/// Information about a GPIO handle request
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GpioHandleRequest {
    pub lineoffsets: [u32; 64],
    pub flags: u32,
    pub default_values: [u8; 64],
    pub consumer_label: [u8; 32],
    pub lines: u32,
    pub fd: i32,
}

impl Default for GpioHandleRequest {
    fn default() -> Self {
        Self {
            lineoffsets: [0; 64],
            flags: 0,
            default_values: [0; 64],
            consumer_label: [0; 32],
            lines: 0,
            fd: -1,
        }
    }
}

/// Information of values on a GPIO handle
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GpioHandleData {
    pub values: [u8; 64],
}

/// Information about a GPIO line
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GpioLineInfo {
    pub line_offset: u32,
    pub flags: u32,
    pub name: [u8; 32],
    pub consumer: [u8; 32],
}

/// Information about a GPIO line change
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GpioLineInfoChanged {
    pub line_info: GpioLineInfo,
    pub timestamp: u64,
    pub event_type: u32,
    pub padding: [u32; 5],
}

/// Information about a GPIO event request
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GpioEventRequest {
    pub lineoffset: u32,
    pub handleflags: u32,
    pub eventflags: u32,
    pub consumer_label: [u8; 32],
    pub fd: i32,
}

impl Default for GpioEventRequest {
    fn default() -> Self {
        Self {
            lineoffset: 0,
            handleflags: 0,
            eventflags: 0,
            consumer_label: [0; 32],
            fd: -1,
        }
    }
}

/// Actual event being pushed to userspace
#[repr(C)]
#[derive(Debug, Clone)]
pub struct GpioEventData {
    pub timestamp: u64,
    pub id: u32,
}

/// GPIO error type
#[derive(Debug)]
pub struct GpioError {
    pub errno: i32,
    pub message: String,
}

impl std::fmt::Display for GpioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.errno, self.message)
    }
}

impl std::error::Error for GpioError {}

/// Open a chip by its path
///
/// # Arguments
///
/// * `gpio_chip` - Path to the GPIO chip device (e.g., "/dev/gpiochip0")
///
/// # Returns
///
/// * `Result<File>` - File handle to the GPIO chip
pub fn chip_open(gpio_chip: &str) -> Result<File> {
    let file = OpenOptions::new()
        .read(true)
        .open(gpio_chip)
        .map_err(|e| Error::msg(format!("Opening GPIO chip: {}", e)))?;

    Ok(file)
}

/// Open and check the information of chip
///
/// # Arguments
///
/// * `label` - Label of the chip to match
/// * `gpio_device` - Path to the GPIO device
///
/// # Returns
///
/// * `Result<File>` - File handle if label matches, None otherwise
pub fn chip_check_info(label: &str, gpio_device: &str) -> Result<Option<File>> {
    let chip_fd = chip_open(gpio_device)?;

    let mut chip_info = GpioChipInfo {
        name: [0; 32],
        label: [0; 32],
        lines: 0,
    };

    let fd = chip_fd.as_raw_fd();

    unsafe {
        let result = libc::ioctl(fd, GPIO_GET_CHIPINFO_IOCTL as libc::c_ulong, &mut chip_info);
        if result < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            return Err(Error::msg(format!(
                "Querying GPIO chip info: errno {}",
                errno
            )));
        }
    }

    let label_cstr = unsafe { CStr::from_ptr(chip_info.label.as_ptr() as *const std::ffi::c_char) };
    let label_str = label_cstr.to_string_lossy();

    // 使用传入的 label 参数进行比较，而不是硬编码
    if label_str != label {
        let _ = close_chip(Some(chip_fd));
        return Ok(None);
    }

    Ok(Some(chip_fd))
}

/// Open a chip by its label
///
/// # Arguments
///
/// * `label` - Label of the chip to open
///
/// # Returns
///
/// * `Result<File>` - File handle to the GPIO chip
pub fn chip_open_by_label(label: &str) -> Result<File> {
    let dev = "/dev";

    for entry in
        std::fs::read_dir(dev).map_err(|e| Error::msg(format!("Reading /dev directory: {}", e)))?
    {
        let entry = entry?;
        let device_name = entry.file_name().to_string_lossy().into_owned();
        if device_name.starts_with("gpiochip") {
            let gpio_device = format!("{}/{}", dev, device_name);
            if let Some(chip_fd) = chip_check_info(label, &gpio_device)? {
                return Ok(chip_fd);
            }
        }
    }

    Err(Error::msg(format!(
        "{}: No such gpio device registered",
        label
    )))
}

/// Close a chip
///
/// # Arguments
///
/// * `chip_fd` - File handle to the chip (can be None)
///
/// # Returns
///
/// * `Result<()>` - Success or error
pub fn close_chip(chip_fd: Option<File>) -> Result<()> {
    if let Some(fd) = chip_fd {
        // File is closed when dropped
        drop(fd);
    }
    Ok(())
}

/// Open a line of a chip
///
/// # Arguments
///
/// * `request` - GpioHandleRequest with configuration
/// * `chip_fd` - File handle to the GPIO chip
///
/// # Returns
///
/// * `Result<i32>` - File descriptor of the line handle
pub fn open_line(request: &mut GpioHandleRequest, chip_fd: &File) -> Result<i32> {
    let fd = chip_fd.as_raw_fd();

    unsafe {
        let result = libc::ioctl(
            fd,
            GPIO_GET_LINEHANDLE_IOCTL as libc::c_ulong,
            &mut *request,
        );
        if result < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            return Err(Error::msg(format!(
                "Opening output line handle: errno {}",
                errno
            )));
        }
    }

    let line_fd = request.fd;
    if line_fd < 0 {
        return Err(Error::msg("Failed to get valid line handle"));
    }

    Ok(line_fd)
}

/// Close a line
///
/// # Arguments
///
/// * `line_handle` - File descriptor of the line (can be None)
///
/// # Returns
///
/// * `Result<()>` - Success or error
pub fn close_line(line_handle: Option<i32>) -> Result<()> {
    if let Some(fd) = line_handle {
        if fd >= 0 {
            unsafe {
                let result = libc::close(fd);
                if result < 0 {
                    let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
                    return Err(Error::msg(format!(
                        "Closing existing GPIO line: errno {}",
                        errno
                    )));
                }
            }
        }
    }
    Ok(())
}

/// Build a request handle struct
///
/// # Arguments
///
/// * `line_offset` - Offset of the line to its chip
/// * `direction` - Direction of the line (IN or OUT)
/// * `initial` - Initial value of the line (for OUT only)
/// * `consumer` - Consumer label for the line
///
/// # Returns
///
/// * `GpioHandleRequest` - Configured request structure
pub fn request_handle(
    line_offset: u32,
    direction: u32,
    initial: Option<u8>,
    consumer: &str,
) -> Result<GpioHandleRequest> {
    let mut request = GpioHandleRequest::default();
    request.lineoffsets[0] = line_offset;
    request.flags = direction;

    if direction == GPIOHANDLE_REQUEST_OUTPUT {
        request.default_values[0] = initial.unwrap_or(1);
    } else if initial.is_some() {
        return Err(Error::msg("initial parameter is not valid for inputs"));
    }

    let consumer_bytes = consumer.as_bytes();
    for (i, &byte) in consumer_bytes.iter().enumerate() {
        if i < 32 {
            request.consumer_label[i] = byte;
        }
    }

    request.lines = 1;

    Ok(request)
}

/// Build a request event struct
///
/// # Arguments
///
/// * `line_offset` - Offset of the line to its chip
/// * `edge` - Event detection edge (RISING, FALLING, or BOTH)
/// * `consumer` - Consumer label for the line
///
/// # Returns
///
/// * `GpioEventRequest` - Configured event request structure
pub fn request_event(line_offset: u32, edge: u32, consumer: &str) -> Result<GpioEventRequest> {
    let mut request = GpioEventRequest::default();
    request.lineoffset = line_offset;
    request.handleflags = GPIOHANDLE_REQUEST_INPUT;
    request.eventflags = edge;

    let consumer_bytes = consumer.as_bytes();
    for (i, &byte) in consumer_bytes.iter().enumerate() {
        if i < 32 {
            request.consumer_label[i] = byte;
        }
    }

    Ok(request)
}

/// Read the value of a line
///
/// # Arguments
///
/// * `line_handle` - File descriptor of the line
///
/// # Returns
///
/// * `Result<u8>` - Current value of the line (0 or 1)
pub fn get_value(line_handle: i32) -> Result<u8> {
    let mut data = GpioHandleData { values: [0; 64] };

    unsafe {
        let result = libc::ioctl(
            line_handle,
            GPIOHANDLE_GET_LINE_VALUES_IOCTL as libc::c_ulong,
            &mut data,
        );
        if result < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            return Err(Error::msg(format!("Getting line value: errno {}", errno)));
        }
    }

    Ok(data.values[0])
}

/// Write a value to a line
///
/// # Arguments
///
/// * `line_handle` - File descriptor of the line
/// * `value` - Value to set (0 or 1)
///
/// # Returns
///
/// * `Result<()>` - Success or error
pub fn set_value(line_handle: i32, value: u8) -> Result<()> {
    let mut data = GpioHandleData { values: [0; 64] };
    data.values[0] = value;

    unsafe {
        let result = libc::ioctl(
            line_handle,
            GPIOHANDLE_SET_LINE_VALUES_IOCTL as libc::c_ulong,
            &data,
        );
        if result < 0 {
            let errno = std::io::Error::last_os_error().raw_os_error().unwrap_or(-1);
            return Err(Error::msg(format!("Setting line value: errno {}", errno)));
        }
    }

    Ok(())
}

/// Simple dataclass for parsing value of a PADCTL register.
#[derive(Debug, Clone)]
pub struct PadCtlRegister {
    pub is_gpio: bool,
    pub is_input: bool,
    pub is_tristate: bool,
}

impl PadCtlRegister {
    pub fn from_value(value: u32) -> Self {
        Self {
            is_gpio: (value & (1 << 10)) == 0,
            is_input: (value & (1 << 6)) != 0,
            is_tristate: (value & (1 << 4)) != 0,
        }
    }

    pub fn is_bidi(&self) -> bool {
        self.is_input && !self.is_tristate
    }
}

/// Check pinmux configuration and warn if mismatch
///
/// # Arguments
///
/// * `reg_addr` - Register address to check
/// * `direction` - Desired direction (0 for OUT, 1 for IN)
/// * `channel` - Channel number for warning messages
///
/// # Returns
///
/// * `Result<()>` - Success or error
pub fn check_pinmux(reg_addr: Option<u32>, direction: u32, channel: u32) -> Result<()> {
    let reg_addr = match reg_addr {
        Some(addr) => addr,
        None => {
            eprintln!("WARNING: pinmux checks not implemented for current device.");
            return Ok(());
        }
    };

    let mem_path = "/dev/mem";
    let mem_fd = OpenOptions::new()
        .read(true)
        .open(mem_path)
        .map_err(|e| Error::msg(format!("Could not open /dev/mem for pinmux check: {}", e)))?;

    const PAGE_SIZE: usize = 4096;
    const MAP_MASK: usize = PAGE_SIZE - 1;

    let reg_page_start = reg_addr & !(MAP_MASK as u32);
    let reg_page_offset = reg_addr - reg_page_start;

    // Read the register value using mmap
    let fd = mem_fd.as_raw_fd();

    unsafe {
        let ptr = libc::mmap(
            std::ptr::null_mut(),
            (PAGE_SIZE * 2) as libc::size_t,
            libc::PROT_READ,
            libc::MAP_SHARED,
            fd,
            reg_page_start as libc::off_t,
        );

        if ptr == libc::MAP_FAILED {
            let _ = close_chip(Some(mem_fd));
            return Err(Error::msg("Failed to map /dev/mem"));
        }

        let devmem = ptr as *const u8;
        let value_ptr = devmem.add(reg_page_offset as usize) as *const u8;
        let mut value_bytes = [0u8; 4];

        // Read 4 bytes (little-endian)
        for i in 0..4 {
            value_bytes[i] = *value_ptr.add(i);
        }

        let reg_value = u32::from_le_bytes(value_bytes);
        let reg = PadCtlRegister::from_value(reg_value);

        let _ = libc::munmap(ptr as *mut libc::c_void, (PAGE_SIZE * 2) as libc::size_t);
        let _ = close_chip(Some(mem_fd));

        // If register is in a bidirectional state, skip checks
        if reg.is_bidi() {
            return Ok(());
        }

        let is_out = direction == 0; // 0 is OUT, 1 is IN from constants

        // If user sets direction to input, but register is output, warn user
        if !is_out && !reg.is_input {
            let corrected_input = reg_value | ((1 << 6) | (1 << 4));
            eprintln!(
                "[WARNING] User requested input for channel \"{}\", but it is set to output in pinmux.",
                channel
            );
            eprintln!("This can be resolved *temporarily* (until next restart) by running:");
            eprintln!(
                "    sudo busybox devmem 0x{:X} w 0x{:X}",
                reg_addr, corrected_input
            );
            eprintln!("For more information on resolving this, please see:");
            eprintln!(
                "https://docs.nvidia.com/jetson/archives/r36.3/DeveloperGuide/HR/JetsonModuleAdaptationAndBringUp/JetsonOrinNxNanoSeries.html#generating-the-pinmux-dtsi-files"
            );
            return Ok(());
        }

        // Same as above, but for when user requests output
        if is_out && reg.is_input {
            let corrected_output = reg_value & !((1 << 6) | (1 << 4));
            eprintln!(
                "[WARNING] User requested output for channel \"{}\", but it is set to input in pinmux.",
                channel
            );
            eprintln!("This can be resolved *temporarily* (until next restart) by running:");
            eprintln!(
                "    sudo busybox devmem 0x{:X} w 0x{:X}",
                reg_addr, corrected_output
            );
            eprintln!("For more information on resolving this, please see:");
            eprintln!(
                "https://docs.nvidia.com/jetson/archives/r36.3/DeveloperGuide/HR/JetsonModuleAdaptationAndBringUp/JetsonOrinNxNanoSeries.html#generating-the-pinmux-dtsi-files"
            );
            return Ok(());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pad_ctl_register() {
        let reg = PadCtlRegister::from_value(0x12345678);
        assert_eq!(reg.is_gpio, false);
        assert_eq!(reg.is_input, false);
        assert_eq!(reg.is_tristate, true);
    }

    #[test]
    fn test_request_handle() {
        let request = request_handle(10, GPIOHANDLE_REQUEST_OUTPUT, Some(0), "test").unwrap();
        assert_eq!(request.lineoffsets[0], 10);
        assert_eq!(request.flags, GPIOHANDLE_REQUEST_OUTPUT);
        assert_eq!(request.default_values[0], 0);
        assert_eq!(request.lines, 1);
    }
}
