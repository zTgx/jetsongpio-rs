use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

const PYTHON_PIN_DATA_PATH: &str = "vendor/jetson-gpio/lib/python/Jetson/GPIO/gpio_pin_data.py";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", PYTHON_PIN_DATA_PATH);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("gpio_pin_data_generated.rs");

    let python_content = fs::read_to_string(PYTHON_PIN_DATA_PATH)
        .expect("Failed to read Python gpio_pin_data.py from vendor/");

    let generated = generate_rust_code(&python_content);

    fs::write(&dest_path, generated).expect("Failed to write generated Rust file");
}

#[derive(Debug, Clone)]
struct PinDef {
    linux_gpio: u32,
    gpio_name: String,
    gpio_chip: String,
    board_pin: u32,
    bcm_pin: u32,
    cvm_pin: String,
    tegra_soc_pin: String,
    pwm_chip_dir: Option<String>,
    pwm_id: Option<u32>,
    padctl_addr: Option<u32>,
}

#[derive(Debug, Clone)]
struct ModelData {
    model_const_name: String,
    func_name: String,
    pin_defs: Vec<PinDef>,
    compats: Vec<String>,
    p1_revision: u32,
    ram: String,
    revision: String,
    model_type: String,
    manufacturer: String,
    processor: String,
}

#[derive(Debug)]
struct ParsedPython {
    models: Vec<ModelData>,
    model_order: Vec<String>,
}

impl ModelData {
    fn compat_func_name(&self) -> String {
        let name = self.model_const_name.to_lowercase();
        match name.as_str() {
            "jetson_orin_nx" => "get_compats_jetson_orins_nx".to_string(),
            "jetson_orin_nano" => "get_compats_jetson_orins_nano".to_string(),
            "jetson_orin" => "get_compats_jetson_orins".to_string(),
            "clara_agx_xavier" => "get_compats_clara_agx_xavier".to_string(),
            "jetson_nx" => "get_compats_nx".to_string(),
            "jetson_xavier" => "get_compats_xavier".to_string(),
            "jetson_tx2_nx" => "get_compats_tx2_nx".to_string(),
            "jetson_tx2" => "get_compats_tx2".to_string(),
            "jetson_tx1" => "get_compats_tx1".to_string(),
            "jetson_nano" => "get_compats_nano".to_string(),
            "jetson_thor_reference" => "get_compats_jetson_thor_reference".to_string(),
            _ => format!("get_compats_{}", name),
        }
    }
}

fn generate_rust_code(python_content: &str) -> String {
    let parsed = parse_python_file(python_content);
    let mut out = String::new();

    // Header
    out.push_str("////////////////////////////////////////////////////////////////////////////////
// AUTO-GENERATED FILE - DO NOT EDIT MANUALLY
//
// This file is generated from vendor/jetson-gpio/lib/python/Jetson/GPIO/gpio_pin_data.py
// by build.rs during cargo build.
//
// To update: update the git submodule in vendor/jetson-gpio and rebuild.
////////////////////////////////////////////////////////////////////////////////\n\n");

    // Model constants
    for model_const in &parsed.model_order {
        out.push_str(&format!("pub const {}: &str = \"{}\";\n", model_const, model_const));
    }
    out.push('\n');

    // get_jetson_models()
    out.push_str("pub fn get_jetson_models() -> Vec<&'static str> {\n    vec![\n");
    for model_const in &parsed.model_order {
        out.push_str(&format!("        {},\n", model_const));
    }
    out.push_str("    ]\n}\n\n");

    // Pin definitions for each model
    for model in &parsed.models {
        out.push_str(&format!("pub fn get_{}_pin_defs() -> Vec<GpioPin> {{\n    vec![\n", model.func_name));
        for pin in &model.pin_defs {
            let pwm_dir = match &pin.pwm_chip_dir {
                Some(s) => format!("Some(\"{}\")", s),
                None => "None".to_string(),
            };
            let pwm_id = match pin.pwm_id {
                Some(id) => format!("Some({})", id),
                None => "None".to_string(),
            };
            let padctl = match pin.padctl_addr {
                Some(addr) => format!("Some(0x{:X})", addr),
                None => "None".to_string(),
            };
            out.push_str(&format!(
                "        GpioPin::new({}, \"{}\", \"{}\", {}, {}, \"{}\", \"{}\", {}, {}, {}),\n",
                pin.linux_gpio, pin.gpio_name, pin.gpio_chip,
                pin.board_pin, pin.bcm_pin, pin.cvm_pin, pin.tegra_soc_pin,
                pwm_dir, pwm_id, padctl,
            ));
        }
        out.push_str("    ]\n}\n\n");
    }

    // Compat functions
    for model in &parsed.models {
        let compat_func_name = model.compat_func_name();
        out.push_str(&format!("pub fn {}() -> Vec<&'static str> {{\n    vec![\n", compat_func_name));
        for compat in &model.compats {
            out.push_str(&format!("        \"{}\",\n", compat));
        }
        out.push_str("    ]\n}\n\n");
    }

    // get_jetson_data()
    out.push_str("pub fn get_jetson_data(model: &str) -> (Vec<GpioPin>, JetsonInfo) {\n    match model {\n");

    let mut emitted_funcs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for model in &parsed.models {
        if emitted_funcs.contains(&model.func_name) {
            continue;
        }

        // Find all models that share this func_name (same pin defs)
        let sharing_models: Vec<&ModelData> = parsed.models.iter()
            .filter(|m| m.func_name == model.func_name)
            .collect();

        let match_pattern = sharing_models.iter()
            .map(|m| m.model_const_name.as_str())
            .collect::<Vec<_>>()
            .join(" | ");

        if sharing_models.len() == 1 {
            let m = &sharing_models[0];
            out.push_str(&format!(
                "        {} => (\n            get_{}_pin_defs(),\n            JetsonInfo {{\n                p1_revision: {},\n                ram: \"{}\".to_string(),\n                revision: \"{}\".to_string(),\n                r#type: \"{}\".to_string(),\n                manufacturer: \"{}\".to_string(),\n                processor: \"{}\".to_string(),\n            }},\n        ),\n",
                match_pattern, m.func_name, m.p1_revision, m.ram, m.revision,
                m.model_type, m.manufacturer, m.processor,
            ));
        } else {
            // Multiple models sharing pin defs (ORIN_NX and ORIN_NANO)
            let first = &sharing_models[0];
            let second = &sharing_models[1];
            out.push_str(&format!(
                "        {} => (\n            get_{}_pin_defs(),\n            JetsonInfo {{\n                p1_revision: {},\n                ram: \"{}\".to_string(),\n                revision: \"{}\".to_string(),\n                r#type: if model == {} {{\n                    \"{}\".to_string()\n                }} else {{\n                    \"{}\".to_string()\n                }},\n                manufacturer: \"{}\".to_string(),\n                processor: \"{}\".to_string(),\n            }},\n        ),\n",
                match_pattern, first.func_name, first.p1_revision, first.ram, first.revision,
                second.model_const_name, second.model_type, first.model_type,
                first.manufacturer, first.processor,
            ));
        }

        emitted_funcs.insert(model.func_name.clone());
    }

    // Default case
    let nano = parsed.models.iter().find(|m| m.model_const_name == "JETSON_NANO").unwrap();
    out.push_str(&format!(
        "        _ => (\n            get_{}_pin_defs(),\n            JetsonInfo {{\n                p1_revision: {},\n                ram: \"{}\".to_string(),\n                revision: \"{}\".to_string(),\n                r#type: \"{}\".to_string(),\n                manufacturer: \"{}\".to_string(),\n                processor: \"{}\".to_string(),\n            }},\n        ),\n",
        nano.func_name, nano.p1_revision, nano.ram, nano.revision,
        nano.model_type, nano.manufacturer, nano.processor,
    ));

    out.push_str("    }\n}\n");
    out
}

fn parse_python_file(content: &str) -> ParsedPython {
    // Parse JETSON_MODELS order
    let models_re = regex::Regex::new(r"JETSON_MODELS\s*=\s*\[([^\]]+)\]").unwrap();
    let model_order: Vec<String> = if let Some(caps) = models_re.captures(content) {
        caps.get(1).unwrap().as_str()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    // Parse PIN_DEFS arrays
    let pin_defs_header_re = regex::Regex::new(r"^([A-Z][A-Z0-9_]*_PIN_DEFS)\s*=\s*\[").unwrap();
    let pin_re = regex::Regex::new(
        r#"\(\s*(\d+)\s*,\s*'([^']*)'\s*,\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*'([^']*)'\s*,\s*'([^']*)'\s*,\s*(None|'[^']*'|"[^"]*")\s*,\s*(None|\d+)\s*(?:,\s*(None|0x[0-9a-fA-F]+))?\s*\)"#
    ).unwrap();

    let mut pin_defs_map: HashMap<String, Vec<PinDef>> = HashMap::new();
    let mut current_array: Option<String> = None;
    let mut bracket_depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = pin_defs_header_re.captures(trimmed) {
            current_array = Some(caps.get(1).unwrap().as_str().to_string());
            bracket_depth = 1;
            continue;
        }

        if current_array.is_some() {
            for ch in trimmed.chars() {
                match ch {
                    '[' => bracket_depth += 1,
                    ']' => {
                        bracket_depth -= 1;
                        if bracket_depth == 0 {
                            current_array = None;
                        }
                    }
                    _ => {}
                }
            }

            if trimmed.starts_with('#') || trimmed.starts_with("//") {
                continue;
            }

            if let Some(caps) = pin_re.captures(trimmed) {
                let linux_gpio: u32 = caps.get(1).unwrap().as_str().parse().unwrap();
                let gpio_name = caps.get(2).unwrap().as_str().to_string();
                let gpio_chip = caps.get(3).unwrap().as_str().to_string();
                let board_pin: u32 = caps.get(4).unwrap().as_str().parse().unwrap();
                let bcm_pin: u32 = caps.get(5).unwrap().as_str().parse().unwrap();
                let cvm_pin = caps.get(6).unwrap().as_str().to_string();
                let tegra_soc_pin = caps.get(7).unwrap().as_str().to_string();
                let pwm_chip_dir_str = caps.get(8).unwrap().as_str();
                let pwm_id_str = caps.get(9).unwrap().as_str();
                let padctl_str = caps.get(10).map(|m| m.as_str());

                let pwm_chip_dir = if pwm_chip_dir_str == "None" {
                    None
                } else {
                    Some(pwm_chip_dir_str.trim_matches(|c| c == '"' || c == '\'').to_string())
                };
                let pwm_id = if pwm_id_str == "None" {
                    None
                } else {
                    Some(pwm_id_str.parse().unwrap())
                };
                let padctl_addr = match padctl_str {
                    Some(s) if s != "None" && !s.is_empty() => {
                        Some(u32::from_str_radix(s.trim_start_matches("0x"), 16).unwrap())
                    }
                    _ => None,
                };

                if let Some(ref arr_name) = current_array {
                    pin_defs_map.entry(arr_name.clone()).or_default().push(PinDef {
                        linux_gpio, gpio_name, gpio_chip, board_pin, bcm_pin,
                        cvm_pin, tegra_soc_pin, pwm_chip_dir, pwm_id, padctl_addr,
                    });
                }
            }
        }
    }

    // Parse compat tuples
    let compat_header_re = regex::Regex::new(r"^(compats_[a-z_]+)\s*=\s*\(").unwrap();
    let compat_val_re = regex::Regex::new(r#"['"]([^'"]+)['"]"#).unwrap();
    let mut compats_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_compat: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = compat_header_re.captures(trimmed) {
            current_compat = Some(caps.get(1).unwrap().as_str().to_string());
            let vals: Vec<String> = compat_val_re.captures_iter(trimmed)
                .map(|m| m.get(1).unwrap().as_str().to_string())
                .collect();
            if let Some(ref name) = current_compat {
                compats_map.entry(name.clone()).or_default().extend(vals);
            }
            if trimmed.contains(')') {
                current_compat = None;
            }
            continue;
        }

        if current_compat.is_some() {
            let vals: Vec<String> = compat_val_re.captures_iter(trimmed)
                .map(|m| m.get(1).unwrap().as_str().to_string())
                .collect();
            if let Some(ref name) = current_compat {
                compats_map.entry(name.clone()).or_default().extend(vals);
            }
            if trimmed.contains(')') {
                current_compat = None;
            }
        }
    }

    // Parse jetson_gpio_data dict for metadata
    // Format spans multiple lines:
    //   MODEL_NAME: (
    //       PIN_DEFS_NAME,
    //       { 'KEY': value, ... }
    //   ),
    let model_entry_re = regex::Regex::new(
        r"(CLARA_AGX_XAVIER|JETSON_[A-Z0-9_]+):\s*\("
    ).unwrap();
    let pin_defs_ref_re = regex::Regex::new(r"([A-Z][A-Z0-9_]*_PIN_DEFS)\s*,").unwrap();
    let metadata_field_re = regex::Regex::new(r"'([A-Z_]+)':\s*'([^']*)'").unwrap();
    let metadata_field_num_re = regex::Regex::new(r"'([A-Z_]+)':\s*(\d+)").unwrap();

    let mut model_metadata: HashMap<String, (String, std::collections::HashMap<String, String>)> = HashMap::new();
    let mut in_gpio_data = false;
    let mut current_model_name: Option<String> = None;
    let mut current_pin_defs_ref: Option<String> = None;
    let mut current_metadata: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let mut in_metadata_dict = false;
    let mut brace_depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("jetson_gpio_data") {
            in_gpio_data = true;
            brace_depth = 0;
            // Count braces on this line (the opening {)
            for ch in trimmed.chars() {
                if ch == '{' { brace_depth += 1; }
                else if ch == '}' { brace_depth -= 1; }
            }
            continue;
        }

        if !in_gpio_data {
            continue;
        }

        // Count braces for all lines in the dict
        for ch in trimmed.chars() {
            match ch {
                '{' => {
                    brace_depth += 1;
                    if brace_depth == 2 {
                        in_metadata_dict = true;
                    }
                }
                '}' => {
                    brace_depth -= 1;
                    if brace_depth == 1 && in_metadata_dict {
                        // Closing inner metadata dict
                        if let (Some(mn), Some(pdr)) = (current_model_name.take(), current_pin_defs_ref.take()) {
                            model_metadata.insert(mn, (pdr, current_metadata.clone()));
                        }
                        current_metadata.clear();
                        in_metadata_dict = false;
                    }
                    if brace_depth == 0 {
                        in_gpio_data = false;
                    }
                }
                _ => {}
            }
        }

        // Detect model entry: "MODEL_NAME: ("
        if let Some(caps) = model_entry_re.captures(trimmed) {
            current_model_name = Some(caps.get(1).unwrap().as_str().to_string());
            current_pin_defs_ref = None;
            current_metadata.clear();
            // PIN_DEFS might be on same line
            if let Some(m) = pin_defs_ref_re.captures(trimmed) {
                current_pin_defs_ref = Some(m.get(1).unwrap().as_str().to_string());
            }
            continue;
        }

        // Look for PIN_DEFS reference if not yet found
        if current_model_name.is_some() && current_pin_defs_ref.is_none() {
            if let Some(m) = pin_defs_ref_re.captures(trimmed) {
                current_pin_defs_ref = Some(m.get(1).unwrap().as_str().to_string());
            }
            continue;
        }

        // Collect metadata fields when inside inner dict
        if in_metadata_dict {
            for m in metadata_field_re.captures_iter(trimmed) {
                current_metadata.insert(m.get(1).unwrap().as_str().to_string(), m.get(2).unwrap().as_str().to_string());
            }
            for m in metadata_field_num_re.captures_iter(trimmed) {
                current_metadata.insert(m.get(1).unwrap().as_str().to_string(), m.get(2).unwrap().as_str().to_string());
            }
        }
    }

    // Map compat variable names to model const names
    let compat_to_model: HashMap<String, String> = [
        ("compats_jetson_orins_nx", "JETSON_ORIN_NX"),
        ("compats_jetson_orins_nano", "JETSON_ORIN_NANO"),
        ("compats_jetson_orins", "JETSON_ORIN"),
        ("compats_clara_agx_xavier", "CLARA_AGX_XAVIER"),
        ("compats_nx", "JETSON_NX"),
        ("compats_xavier", "JETSON_XAVIER"),
        ("compats_tx2_nx", "JETSON_TX2_NX"),
        ("compats_tx2", "JETSON_TX2"),
        ("compats_tx1", "JETSON_TX1"),
        ("compats_nano", "JETSON_NANO"),
        ("compats_jetson_thor_reference", "JETSON_THOR_REFERENCE"),
    ].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();

    // Build ModelData for each model
    let mut models: Vec<ModelData> = Vec::new();

    for model_const in &model_order {
        let (pin_defs_var, meta) = model_metadata.get(model_const.as_str())
            .cloned()
            .unwrap_or_else(|| {
                (format!("{}_PIN_DEFS", model_const), std::collections::HashMap::new())
            });

        let pin_defs = pin_defs_map.get(&pin_defs_var).cloned().unwrap_or_default();

        let compat_var = compat_to_model.iter()
            .find(|(_, v)| *v == model_const.as_str())
            .map(|(k, _)| k.clone());
        let compats = compat_var
            .and_then(|cv| compats_map.get(&cv).cloned())
            .unwrap_or_default();

        models.push(ModelData {
            model_const_name: model_const.clone(),
            func_name: model_const.to_lowercase(),
            pin_defs,
            compats,
            p1_revision: meta.get("P1_REVISION").and_then(|v| v.parse().ok()).unwrap_or(1),
            ram: meta.get("RAM").cloned().unwrap_or_default(),
            revision: meta.get("REVISION").cloned().unwrap_or("Unknown".to_string()),
            model_type: meta.get("TYPE").cloned().unwrap_or_default(),
            manufacturer: meta.get("MANUFACTURER").cloned().unwrap_or("NVIDIA".to_string()),
            processor: meta.get("PROCESSOR").cloned().unwrap_or_default(),
        });
    }

    ParsedPython { models, model_order }
}
