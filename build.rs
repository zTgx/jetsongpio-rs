use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

const PYTHON_GPIO_URL: &str = "https://raw.githubusercontent.com/NVIDIA/jetson-gpio/master/lib/python/Jetson/GPIO/gpio_pin_data.py";
const LOCAL_CACHE_PATH: &str = ".cargo/cache/gpio_pin_data.py";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=UPDATE_PINMUX_DATA");

    let python_lib_path = Path::new(LOCAL_CACHE_PATH);
    let rust_src_path = Path::new("src/gpio_pin_data.rs");

    // Ensure cache directory exists
    if let Some(parent) = python_lib_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Download or use cached Python file
    if !python_lib_path.exists() {
        if let Err(e) = download_python_gpio(python_lib_path) {
            println!("cargo:warning=Failed to download Python GPIO library: {}", e);
            return;
        }
    }

    // Print trigger source if set
    if let Ok(var) = env::var("UPDATE_PINMUX_DATA") {
        println!("Pinmux update triggered by UPDATE_PINMUX_DATA={}", var);
    } else if python_source_changed(python_lib_path, rust_src_path) {
        println!("Pinmux update triggered: Python source has newer changes");
    } else {
        println!("Pinmux data is up to date (set UPDATE_PINMUX_DATA=1 to force update)");
        return;
    }

    // Execute update
    if let Err(e) = update_pinmux_data(python_lib_path, rust_src_path) {
        eprintln!("ERROR: Pinmux data update failed: {}", e);
        eprintln!("Set UPDATE_PINMUX_DATA=1 to try again");
    }
}

/// Download Python GPIO library from GitHub
fn download_python_gpio(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Downloading Python GPIO library from GitHub...");
    let response = ureq::get(PYTHON_GPIO_URL).call()?;
    let content = response.into_string()?;

    fs::write(path, content)?;
    println!("Downloaded to: {}", path.display());
    Ok(())
}

/// Check if Python source has changed and needs update
fn python_source_changed(python_path: &Path, rust_path: &Path) -> bool {
    // Check if Rust file is readable
    match fs::read_to_string(rust_path) {
        Ok(_) => {}
        Err(_) => return true, // File does not exist, need to create
    }

    // Get current commit hash from Python cache
    // Since we download from a URL, use file modification time instead
    compare_file_mtime(python_path, rust_path)
}

/// Compare file modification time
fn compare_file_mtime(python_path: &Path, rust_path: &Path) -> bool {
    let python_mtime = fs::metadata(python_path).and_then(|m| m.modified()).ok();
    let rust_mtime = fs::metadata(rust_path).and_then(|m| m.modified()).ok();

    match (python_mtime, rust_mtime) {
        (Some(py), Some(rs)) => py > rs,
        _ => true,
    }
}

/// Extract and update pinmux register data from Python library
fn update_pinmux_data(python_path: &Path, rust_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("Extracting pinmux data from: {}", python_path.display());

    // Read Python file
    let python_content = fs::read_to_string(python_path)?;

    // Extract all PIN_DEFS array pin data
    let pinmux_data = extract_pinmux_from_python(&python_content)?;
    println!("Extracted {} pinmux entries from Python", pinmux_data.len());

    // Read Rust source file
    let rust_content = fs::read_to_string(rust_path)?;

    // Update pinmux data in Rust source file
    let updated_content = update_rust_file(&rust_content, &pinmux_data, python_path)?;

    // Write updated content
    fs::write(rust_path, updated_content)?;

    println!("Pinmux data updated successfully!");
    println!("  Written to: {}", rust_path.display());

    Ok(())
}

/// Extract pinmux register data from Python file
///
/// Returns HashMap<(model_name, linux_gpio), register_addr>
fn extract_pinmux_from_python(content: &str) -> Result<HashMap<(String, u32), u32>, Box<dyn std::error::Error>> {
    let mut pinmux_data = HashMap::new();

    // Regex to match PIN_DEFS array pin entries
    // Format: (linux_gpio, 'gpio_name', "chip_name", board_pin, bcm_pin, 'cvm_name', 'tegra_soc_name', pwm_chip_dir, pwm_id, register_addr)
    let re = regex::Regex::new(r#"(\d+),\s*'([^']+)',\s*"([^"]+)",\s*(\d+),\s*(\d+),\s*'([^']+)',\s*'([^']+)',\s*([^,)]+),\s*([^,)]+),\s*(0x[0-9a-fA-F]+|None)"#)?;

    // Find all PIN_DEFS definitions
    let array_pattern = regex::Regex::new(r"(JETSON_[A-Z_]+_PIN_DEFS)\s*=\s*\[")?;

    let mut current_model = String::new();
    let mut updated_count = 0;

    for line in content.lines() {
        // Detect new array definition
        if let Some(caps) = array_pattern.captures(line) {
            current_model = caps.get(1).unwrap().as_str().to_string();
        }

        // Parse pin definition
        if let Some(caps) = re.captures(line) {
            let linux_gpio: u32 = caps.get(1).unwrap().as_str().parse()?;
            let _gpio_name = caps.get(2).unwrap().as_str().to_string();

            // Check if valid register address
            let register_str = caps.get(10).unwrap().as_str();
            if register_str == "None" {
                continue;
            }

            let register_addr: u32 = u32::from_str_radix(&register_str.trim_start_matches("0x"), 16)?;

            let key = (current_model.clone(), linux_gpio);
            pinmux_data.insert(key, register_addr);
            updated_count += 1;
        }
    }

    println!("  Found {} pins with valid register addresses", updated_count);

    Ok(pinmux_data)
}

/// Update pinmux data in Rust source file
fn update_rust_file(
    rust_content: &str,
    pinmux_data: &HashMap<(String, u32), u32>,
    python_path: &Path,
) -> Result<String, Box<dyn std::error::Error>> {
    let lines: Vec<&str> = rust_content.lines().collect();
    let mut updated_lines = Vec::new();

    // Skip existing header comments (if any)
    // Skip until we find a non-comment, non-empty line (start of actual code)
    let start_idx = lines.iter()
        .position(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("//") && !trimmed.is_empty()
        })
        .unwrap_or(0);

    let code_lines = &lines[start_idx..];

    // Generate new header comment
    let timestamp = chrono::Utc::now().to_rfc3339();
    let header = format!(r#"////////////////////////////////////////////////////////////////////////////////
// AUTO-GENERATED FILE - DO NOT EDIT MANUALLY
//
// This file is automatically generated from the NVIDIA Jetson GPIO Python library.
// Any manual changes will be overwritten when updating.
//
// Source file: {}
// Last updated: {}
//
// To regenerate:
//   UPDATE_PINMUX_DATA=1 cargo build
////////////////////////////////////////////////////////////////////////////////

"#, python_path.display(), timestamp);

    updated_lines.push(header);

    // Parse and update pin definitions
    let mut current_function = String::new();
    let mut model_found = false;
    let mut current_gpio: Option<u32> = None;
    let mut update_count = 0;

    for line in code_lines.iter() {
        let trimmed = line.trim();

        // Detect function definition
        if trimmed.starts_with("pub fn get_") && trimmed.contains("_pin_defs()") {
            current_function = trimmed.split_whitespace()
                .nth(2)
                .unwrap_or("")
                .replace("()", "")
                .to_string();

            // Map Rust function name to Python array name
            current_function = map_rust_func_to_python_array(&current_function);
            model_found = true;
        } else if trimmed.starts_with("pub fn get_") {
            current_function.clear();
            model_found = false;
        }

        // Extract linux_gpio from GpioPin::new
        if trimmed.starts_with("GpioPin::new(") {
            if let Some(gpio) = parse_gpio_from_line(line) {
                current_gpio = Some(gpio);
            }
        }

        // Detect and update padctl_addr (last parameter)
        if model_found && current_gpio.is_some() && (trimmed.contains("Some(0x") || trimmed.contains("None),")) {
            let gpio = current_gpio.unwrap();

            // Find corresponding register address
            if let Some(&addr) = pinmux_data.get(&(current_function.clone(), gpio)) {
                // Replace last parameter (padctl_addr)
                if let Some(updated_line) = update_padctl_addr(line, addr) {
                    updated_lines.push(updated_line);
                    update_count += 1;
                    current_gpio = None;
                    continue;
                }
            }
            current_gpio = None;
        }

        updated_lines.push(line.to_string());
    }

    println!("  Updated {} pin definitions", update_count);

    Ok(updated_lines.join("\n"))
}

/// Map Rust function name to Python array name
fn map_rust_func_to_python_array(func_name: &str) -> String {
    // get_jetson_orin_nx_pin_defs -> JETSON_ORIN_NX_PIN_DEFS
    // get_clara_agx_xavier_pin_defs -> CLARA_AGX_XAVIER_PIN_DEFS
    let name = func_name
        .strip_prefix("get_")
        .unwrap_or(func_name)
        .strip_suffix("_pin_defs")
        .unwrap_or(func_name)
        .to_uppercase();

    // Handle special case for CLARA_AGX_XAVIER (no JETSON_ prefix)
    if name == "CLARA_AGX_XAVIER" || name == "JETSON_THOR_REFERENCE" {
        format!("{}_PIN_DEFS", name)
    } else {
        format!("{}_PIN_DEFS", name)
    }
}

/// Parse linux_gpio from GpioPin::new line
fn parse_gpio_from_line(line: &str) -> Option<u32> {
    // Match first number after GpioPin::new(
    let re = regex::Regex::new(r"GpioPin::new\(\s*(\d+),").unwrap();
    re.captures(line)?
        .get(1)?
        .as_str()
        .parse()
        .ok()
}

/// Update padctl_addr parameter in line
fn update_padctl_addr(line: &str, addr: u32) -> Option<String> {
    let new_addr = format!("Some(0x{:X})", addr);

    // Replace last parameter
    // Match: Some(0x...), or None),
    let re = regex::Regex::new(r"(Some\(0x[0-9a-fA-F]+\)|None),\s*\)?\s*$").unwrap();
    if re.is_match(line) {
        Some(re.replace(line, &format!("{},", new_addr)).to_string())
    } else {
        None
    }
}