use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub struct ChannelInfo {
    pub channel: u32,
    pub chip_fd: Option<i32>,
    pub line_handle: Option<i32>,
    pub line_offset: u32,
    pub direction: Option<Direction>,
    pub edge: Option<String>,
    pub consumer: String,
    pub gpio_name: String,
    pub gpio_chip: String,
    pub pwm_chip_dir: Option<String>,
    pub pwm_id: Option<u32>,
    pub reg_addr: Option<u32>,
    pub f_duty_cycle: Option<fs::File>,
}

impl Clone for ChannelInfo {
    fn clone(&self) -> Self {
        ChannelInfo {
            channel: self.channel,
            chip_fd: self.chip_fd,
            line_handle: self.line_handle,
            line_offset: self.line_offset,
            direction: self.direction,
            edge: self.edge.clone(),
            consumer: self.consumer.clone(),
            gpio_name: self.gpio_name.clone(),
            gpio_chip: self.gpio_chip.clone(),
            pwm_chip_dir: self.pwm_chip_dir.clone(),
            pwm_id: self.pwm_id,
            reg_addr: self.reg_addr,
            f_duty_cycle: None, // File is not Clone; cloned instances have no fd
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    IN = 0,
    OUT = 1,
    HardPwm = 43,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mode {
    BOARD,
    BCM,
    TegraSoc,
    CVM,
}

impl Mode {
    pub fn is_valid(&self) -> bool {
        matches!(self, Mode::BOARD | Mode::BCM | Mode::TegraSoc | Mode::CVM)
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            Mode::BOARD => "BOARD",
            Mode::BCM => "BCM",
            Mode::TegraSoc => "TegraSoc",
            Mode::CVM => "CVM",
        }
    }
}

#[derive(Debug)]
pub struct JetsonInfo {
    pub p1_revision: u32,
    pub ram: String,
    pub revision: String,
    pub r#type: String,
    pub manufacturer: String,
    pub processor: String,
}

#[derive(Debug, Clone)]
pub struct GpioPin {
    pub linux_gpio: u32,
    pub gpio_name: String,
    pub gpio_chip: String,
    pub board_pin: u32,
    pub bcm_pin: u32,
    pub cvm_pin: String,
    pub tegra_soc_pin: String,
    pub pwm_chip_sysfs_dir: Option<String>,
    pub pwm_id: Option<u32>,
    pub padctl_addr: Option<u32>,
}

impl GpioPin {
    pub fn new(
        linux_gpio: u32,
        gpio_name: &str,
        gpio_chip: &str,
        board_pin: u32,
        bcm_pin: u32,
        cvm_pin: &str,
        tegra_soc_pin: &str,
        pwm_chip_sysfs_dir: Option<&str>,
        pwm_id: Option<u32>,
        padctl_addr: Option<u32>,
    ) -> Self {
        GpioPin {
            linux_gpio,
            gpio_name: gpio_name.to_string(),
            gpio_chip: gpio_chip.to_string(),
            board_pin,
            bcm_pin,
            cvm_pin: cvm_pin.to_string(),
            tegra_soc_pin: tegra_soc_pin.to_string(),
            pwm_chip_sysfs_dir: pwm_chip_sysfs_dir.map(|s| s.to_string()),
            pwm_id,
            padctl_addr,
        }
    }
}

// Include auto-generated data (model constants, pin defs, compat strings, get_jetson_data)
include!(concat!(env!("OUT_DIR"), "/gpio_pin_data_generated.rs"));

/// Resolve raw PWM chip names (e.g. "3280000.pwm") to full sysfs pwmchip paths.
/// Mirrors Python's gpio_pin_data.py get_data() pwm_dirs resolution logic.
fn resolve_pwm_chip_dirs(pin_defs: &[GpioPin]) -> HashMap<String, String> {
    let sysfs_prefixes = [
        "/sys/devices/",
        "/sys/devices/platform/",
        "/sys/bus/platform/devices/",
    ];
    let mut pwm_dirs: HashMap<String, String> = HashMap::new();

    let pwm_chip_names: std::collections::HashSet<&str> = pin_defs
        .iter()
        .filter_map(|p| p.pwm_chip_sysfs_dir.as_deref())
        .collect();

    for pwm_chip_name in &pwm_chip_names {
        let mut pwm_chip_dir = None;
        for prefix in &sysfs_prefixes {
            let d = format!("{}{}", prefix, pwm_chip_name);
            if Path::new(&d).is_dir() {
                pwm_chip_dir = Some(d);
                break;
            }
        }
        let Some(pwm_chip_dir) = pwm_chip_dir else {
            continue;
        };
        let pwm_chip_pwm_dir = format!("{}/pwm", pwm_chip_dir);
        if !Path::new(&pwm_chip_pwm_dir).exists() {
            continue;
        }
        if let Ok(entries) = fs::read_dir(&pwm_chip_pwm_dir) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("pwmchip") {
                    pwm_dirs.insert(
                        pwm_chip_name.to_string(),
                        format!("{}/{}", pwm_chip_pwm_dir, name_str),
                    );
                    break;
                }
            }
        }
    }
    pwm_dirs
}

fn get_compatibles(path: &str) -> Result<Vec<String>, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("Failed to open {}: {}", path, e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;

    let compatibles: Vec<String> = buffer
        .split(|&b| b == 0)
        .filter(|bytes| !bytes.is_empty())
        .filter_map(|bytes| String::from_utf8(bytes.to_vec()).ok())
        .collect();

    if compatibles.is_empty() {
        return Err(format!("No compatible strings found in {}", path));
    }

    Ok(compatibles)
}

pub fn get_model() -> Result<String, String> {
    if let Ok(model_name) = std::env::var("JETSON_TESTING_MODEL_NAME") {
        let model_name = model_name.trim().to_string();
        if get_jetson_models().contains(&model_name.as_str()) {
            return Ok(model_name);
        } else {
            eprintln!(
                "Environment variable 'JETSON_TESTING_MODEL_NAME={}' is invalid.",
                model_name
            );
        }
    }

    let compatible_path = "/proc/device-tree/compatible";
    if Path::new(compatible_path).exists() {
        let compatibles = get_compatibles(compatible_path)?;

        if matches_any(&compatibles, &get_compats_tx1()) {
            warn_if_not_carrier_board(&["2597"])?;
            return Ok(JETSON_TX1.to_string());
        } else if matches_any(&compatibles, &get_compats_tx2()) {
            warn_if_not_carrier_board(&["2597"])?;
            return Ok(JETSON_TX2.to_string());
        } else if matches_any(&compatibles, &get_compats_clara_agx_xavier()) {
            warn_if_not_carrier_board(&["3900"])?;
            return Ok(CLARA_AGX_XAVIER.to_string());
        } else if matches_any(&compatibles, &get_compats_tx2_nx()) {
            warn_if_not_carrier_board(&["3509"])?;
            return Ok(JETSON_TX2_NX.to_string());
        } else if matches_any(&compatibles, &get_compats_xavier()) {
            warn_if_not_carrier_board(&["2822"])?;
            return Ok(JETSON_XAVIER.to_string());
        } else if matches_any(&compatibles, &get_compats_nano()) {
            let module_id = find_pmgr_board("3448")?;
            let revision = module_id.split('-').last().unwrap_or("");
            if revision < "200" {
                return Err("Jetson Nano module revision must be A02 or later".to_string());
            }
            warn_if_not_carrier_board(&["3449", "3542"])?;
            return Ok(JETSON_NANO.to_string());
        } else if matches_any(&compatibles, &get_compats_nx()) {
            warn_if_not_carrier_board(&["3509", "3449"])?;
            return Ok(JETSON_NX.to_string());
        } else if matches_any(&compatibles, &get_compats_jetson_orins()) {
            warn_if_not_carrier_board(&["3737"])?;
            return Ok(JETSON_ORIN.to_string());
        } else if matches_any(&compatibles, &get_compats_jetson_orins_nx()) {
            warn_if_not_carrier_board(&["3509", "3768"])?;
            return Ok(JETSON_ORIN_NX.to_string());
        } else if matches_any(&compatibles, &get_compats_jetson_orins_nano()) {
            warn_if_not_carrier_board(&["3509", "3768"])?;
            return Ok(JETSON_ORIN_NANO.to_string());
        } else if matches_any(&compatibles, &get_compats_jetson_thor_reference()) {
            warn_if_not_carrier_board(&["3971", "4071"])?;
            return Ok(JETSON_THOR_REFERENCE.to_string());
        }
    }

    if let Ok(model_name) = std::env::var("JETSON_MODEL_NAME") {
        let model_name = model_name.trim().to_string();
        if get_jetson_models().contains(&model_name.as_str()) {
            return Ok(model_name);
        } else {
            eprintln!(
                "Environment variable 'JETSON_MODEL_NAME={}' is invalid.",
                model_name
            );
        }
    }

    Err("Could not determine Jetson model".to_string())
}

fn matches_any(compatibles: &[String], patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| {
        compatibles
            .iter()
            .any(|compatible| compatible.contains(pattern))
    })
}

fn hash_string(s: &str) -> u64 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

static mut WARNED: bool = false;

fn warn_if_not_carrier_board(carrier_boards: &[&str]) -> Result<(), String> {
    let mut found = false;

    for &board in carrier_boards {
        if let Ok(found_board) = find_pmgr_board(board) {
            if found_board.starts_with(board) {
                found = true;
                break;
            }
        }
    }

    if !found {
        unsafe {
            if !WARNED {
                WARNED = true;
                eprintln!(
                    "WARNING: Carrier board is not from a Jetson Developer Kit.\n\
                     WARNING: Jetson.GPIO library has not been verified with this carrier board,\n\
                     WARNING: and in fact is unlikely to work correctly."
                );
            }
        }
    }

    Ok(())
}

fn find_pmgr_board(prefix: &str) -> Result<String, String> {
    let ids_paths = [
        "/proc/device-tree/chosen/plugin-manager/ids",
        "/proc/device-tree/chosen/ids",
    ];

    for ids_path in ids_paths {
        if Path::new(ids_path).exists() {
            if Path::new(ids_path).is_dir() {
                if let Ok(entries) = fs::read_dir(ids_path) {
                    for entry in entries.flatten() {
                        let file_name = entry.file_name();
                        if let Some(name) = file_name.to_str() {
                            if name.starts_with(prefix) {
                                return Ok(name.to_string());
                            }
                        }
                    }
                }
            } else if Path::new(ids_path).is_file() {
                if let Ok(content) = fs::read_to_string(ids_path) {
                    for s in content.split_whitespace() {
                        if s.starts_with(prefix) {
                            return Ok(s.to_string());
                        }
                    }
                }
            }
        }
    }

    Err(format!(
        "Could not find PMGR board with prefix '{}'",
        prefix
    ))
}

pub fn get_data() -> (String, JetsonInfo, HashMap<Mode, HashMap<u32, ChannelInfo>>) {
    let model = get_model().unwrap();
    let (pin_defs, jetson_info) = get_jetson_data(&model);
    let pwm_dirs = resolve_pwm_chip_dirs(&pin_defs);

    let make_channel_info = |pin: &GpioPin, channel: u32| ChannelInfo {
        channel,
        chip_fd: None,
        line_handle: None,
        line_offset: pin.linux_gpio,
        direction: None,
        edge: None,
        consumer: "jetsongpio-rs".to_string(),
        gpio_name: pin.gpio_name.clone(),
        gpio_chip: pin.gpio_chip.clone(),
        pwm_chip_dir: pin
            .pwm_chip_sysfs_dir
            .as_deref()
            .and_then(|name| pwm_dirs.get(name).cloned()),
        pwm_id: pin.pwm_id,
        reg_addr: pin.padctl_addr,
        f_duty_cycle: None,
    };

    let board_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| (pin.board_pin, make_channel_info(pin, pin.board_pin)))
        .collect();

    let bcm_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| (pin.bcm_pin, make_channel_info(pin, pin.bcm_pin)))
        .collect();

    let cvm_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| {
            let key = hash_string(&pin.cvm_pin) as u32;
            (key, make_channel_info(pin, key))
        })
        .collect();

    let tegra_soc_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| {
            let key = hash_string(&pin.tegra_soc_pin) as u32;
            (key, make_channel_info(pin, key))
        })
        .collect();

    let mut all_modes = HashMap::new();
    all_modes.insert(Mode::BOARD, board_channels);
    all_modes.insert(Mode::BCM, bcm_channels);
    all_modes.insert(Mode::CVM, cvm_channels);
    all_modes.insert(Mode::TegraSoc, tegra_soc_channels);

    (model, jetson_info, all_modes)
}
