use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;

const PYTHON_PIN_DATA_PATH: &str =
    "vendor/jetson-gpio/lib/python/Jetson/GPIO/gpio_pin_data.py";

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", PYTHON_PIN_DATA_PATH);

    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest_path = Path::new(&out_dir).join("gpio_pin_data_generated.rs");

    let python_content = fs::read_to_string(PYTHON_PIN_DATA_PATH)
        .unwrap_or_else(|e| panic!("Failed to read {}: {}", PYTHON_PIN_DATA_PATH, e));

    let generated = generate_rust_code(&python_content);

    fs::write(&dest_path, generated).expect("Failed to write generated Rust file");
}

// ---------------------------------------------------------------------------
// Data structures
// ---------------------------------------------------------------------------

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
    compat_func_name: String,
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

// ---------------------------------------------------------------------------
// Code generation
// ---------------------------------------------------------------------------

fn generate_rust_code(python_content: &str) -> String {
    let parsed = parse_python_file(python_content);

    // ── Validation ──────────────────────────────────────────────────────
    assert!(
        !parsed.models.is_empty(),
        "build.rs: no models parsed from Python pin data — parser may be broken"
    );

    for model in &parsed.models {
        assert!(
            !model.pin_defs.is_empty(),
            "build.rs: model {} has no pin definitions — pin_defs parsing may be broken",
            model.model_const_name
        );
        assert!(
            (15..=30).contains(&model.pin_defs.len()),
            "build.rs: model {} has {} pins, expected 15-30 — data may be corrupt",
            model.model_const_name,
            model.pin_defs.len()
        );
        assert!(
            !model.compats.is_empty(),
            "build.rs: model {} has no compatibility strings — \
             compat_to_model mapping may be missing or Python get_model() changed",
            model.model_const_name
        );
    }

    // ── Generation ──────────────────────────────────────────────────────
    let mut out = String::new();

    // Header
    out.push_str(
        "////////////////////////////////////////////////////////////////////////////////
// AUTO-GENERATED FILE - DO NOT EDIT MANUALLY
//
// This file is generated from vendor/jetson-gpio/lib/python/Jetson/GPIO/gpio_pin_data.py
// by build.rs during cargo build.
//
// To update: update the git submodule in vendor/jetson-gpio and rebuild.
////////////////////////////////////////////////////////////////////////////////\n\n",
    );

    // Model constants
    for model_const in &parsed.model_order {
        out.push_str(&format!(
            "pub const {}: &str = \"{}\";\n",
            model_const, model_const
        ));
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
        out.push_str(&format!(
            "pub fn get_{}_pin_defs() -> Vec<GpioPin> {{\n    vec![\n",
            model.func_name
        ));
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
                pin.linux_gpio,
                pin.gpio_name,
                pin.gpio_chip,
                pin.board_pin,
                pin.bcm_pin,
                pin.cvm_pin,
                pin.tegra_soc_pin,
                pwm_dir,
                pwm_id,
                padctl,
            ));
        }
        out.push_str("    ]\n}\n\n");
    }

    // Compat functions
    for model in &parsed.models {
        out.push_str(&format!(
            "pub fn {}() -> Vec<&'static str> {{\n    vec![\n",
            model.compat_func_name
        ));
        for compat in &model.compats {
            out.push_str(&format!("        \"{}\",\n", compat));
        }
        out.push_str("    ]\n}\n\n");
    }

    // get_jetson_data()
    out.push_str(
        "pub fn get_jetson_data(model: &str) -> (Vec<GpioPin>, JetsonInfo) {\n    match model {\n",
    );

    let mut emitted_funcs: std::collections::HashSet<String> = std::collections::HashSet::new();

    for model in &parsed.models {
        if emitted_funcs.contains(&model.func_name) {
            continue;
        }

        // Find all models that share this func_name (same pin defs)
        let sharing_models: Vec<&ModelData> = parsed
            .models
            .iter()
            .filter(|m| m.func_name == model.func_name)
            .collect();

        let match_pattern = sharing_models
            .iter()
            .map(|m| m.model_const_name.as_str())
            .collect::<Vec<_>>()
            .join(" | ");

        if sharing_models.len() == 1 {
            let m = &sharing_models[0];
            out.push_str(&format!(
                "        {} => (\n            \
                 get_{}_pin_defs(),\n            \
                 JetsonInfo {{\n                \
                 p1_revision: {},\n                \
                 ram: \"{}\".to_string(),\n                \
                 revision: \"{}\".to_string(),\n                \
                 r#type: \"{}\".to_string(),\n                \
                 manufacturer: \"{}\".to_string(),\n                \
                 processor: \"{}\".to_string(),\n            \
                 }},\n        ),\n",
                match_pattern,
                m.func_name,
                m.p1_revision,
                m.ram,
                m.revision,
                m.model_type,
                m.manufacturer,
                m.processor,
            ));
        } else {
            // Multiple models sharing pin defs (e.g. ORIN_NX and ORIN_NANO)
            let first = &sharing_models[0];
            let second = &sharing_models[1];
            out.push_str(&format!(
                "        {} => (\n            \
                 get_{}_pin_defs(),\n            \
                 JetsonInfo {{\n                \
                 p1_revision: {},\n                \
                 ram: \"{}\".to_string(),\n                \
                 revision: \"{}\".to_string(),\n                \
                 r#type: if model == {} {{\n                    \
                 \"{}\".to_string()\n                \
                 }} else {{\n                    \
                 \"{}\".to_string()\n                \
                 }},\n                \
                 manufacturer: \"{}\".to_string(),\n                \
                 processor: \"{}\".to_string(),\n            \
                 }},\n        ),\n",
                match_pattern,
                first.func_name,
                first.p1_revision,
                first.ram,
                first.revision,
                second.model_const_name,
                second.model_type,
                first.model_type,
                first.manufacturer,
                first.processor,
            ));
        }

        emitted_funcs.insert(model.func_name.clone());
    }

    // No default fallback — unknown model is a bug, fail at runtime
    out.push_str(
        "        _ => panic!(\"get_jetson_data: unknown Jetson model: {}\", model),\n",
    );

    out.push_str("    }\n}\n");
    out
}

// ---------------------------------------------------------------------------
// Python parser
// ---------------------------------------------------------------------------

fn parse_python_file(content: &str) -> ParsedPython {
    // ── 1. Parse JETSON_MODELS order ────────────────────────────────────
    let models_re =
        regex::Regex::new(r"JETSON_MODELS\s*=\s*\[([^\]]+)\]").expect("models_re is valid");
    let model_order: Vec<String> = if let Some(caps) = models_re.captures(content) {
        caps.get(1)
            .expect("models_re group 1")
            .as_str()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    assert!(
        !model_order.is_empty(),
        "build.rs: failed to parse JETSON_MODELS from Python source"
    );

    // ── 2. Parse PIN_DEFS arrays (with multi-line tuple support) ────────
    let pin_defs_header_re =
        regex::Regex::new(r"^([A-Z][A-Z0-9_]*_PIN_DEFS)\s*=\s*\[").expect("pin_defs_header_re");
    let pin_re = regex::Regex::new(
        r#"\(\s*(\d+)\s*,\s*'([^']*)'\s*,\s*"([^"]+)"\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*'([^']*)'\s*,\s*'([^']*)'\s*,\s*(None|'[^']*'|"[^"]*")\s*,\s*(None|\d+)\s*(?:,\s*(None|0x[0-9a-fA-F]+))?\s*\)"#
    ).expect("pin_re is valid");

    let mut pin_defs_map: HashMap<String, Vec<PinDef>> = HashMap::new();
    let mut current_array: Option<String> = None;
    let mut bracket_depth: i32 = 0;
    let mut tuple_buf = String::new();   // accumulates multi-line tuples
    let mut in_tuple = false;
    let mut tuple_paren_depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        // Detect array header
        if !in_tuple {
            if let Some(caps) = pin_defs_header_re.captures(trimmed) {
                current_array = Some(
                    caps.get(1)
                        .expect("pin_defs_header_re group 1")
                        .as_str()
                        .to_string(),
                );
                bracket_depth = 1;
                continue;
            }
        }

        if current_array.is_none() || in_tuple {
            // skip unless inside an array or already accumulating a tuple
            if !in_tuple {
                continue;
            }
        }

        // Skip pure comment lines
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }

        // Track array bracket depth
        for ch in trimmed.chars() {
            match ch {
                '[' => bracket_depth += 1,
                ']' => {
                    bracket_depth -= 1;
                    if bracket_depth == 0 {
                        current_array = None;
                        in_tuple = false;
                        tuple_buf.clear();
                    }
                }
                _ => {}
            }
        }

        if current_array.is_none() {
            continue;
        }

        // ── Multi-line tuple accumulation ──
        // Look for the start of a tuple on this line
        if !in_tuple {
            if let Some(open_pos) = trimmed.find('(') {
                // Check it's actually a pin tuple (starts with digit after '(')
                let after = trimmed[open_pos + 1..].trim_start();
                if after.chars().next().map_or(false, |c| c.is_ascii_digit()) {
                    in_tuple = true;
                    tuple_buf.clear();
                    tuple_paren_depth = 0;
                }
            }
        }

        if in_tuple {
            tuple_buf.push(' ');
            tuple_buf.push_str(trimmed);

            // Count parentheses to know when the tuple closes
            for ch in trimmed.chars() {
                match ch {
                    '(' => tuple_paren_depth += 1,
                    ')' => {
                        tuple_paren_depth -= 1;
                    }
                    _ => {}
                }
            }

            // Tuple is complete when parens are balanced
            if tuple_paren_depth == 0 {
                if let Some(caps) = pin_re.captures(&tuple_buf) {
                    let linux_gpio: u32 = caps
                        .get(1)
                        .expect("pin_re group 1 (linux_gpio)")
                        .as_str()
                        .parse()
                        .unwrap_or_else(|_| {
                            panic!(
                                "build.rs: failed to parse linux_gpio as u32 in: {}",
                                tuple_buf
                            )
                        });
                    let gpio_name = caps
                        .get(2)
                        .expect("pin_re group 2 (gpio_name)")
                        .as_str()
                        .to_string();
                    let gpio_chip = caps
                        .get(3)
                        .expect("pin_re group 3 (gpio_chip)")
                        .as_str()
                        .to_string();
                    let board_pin: u32 = caps
                        .get(4)
                        .expect("pin_re group 4 (board_pin)")
                        .as_str()
                        .parse()
                        .unwrap_or_else(|_| {
                            panic!(
                                "build.rs: failed to parse board_pin as u32 in: {}",
                                tuple_buf
                            )
                        });
                    let bcm_pin: u32 = caps
                        .get(5)
                        .expect("pin_re group 5 (bcm_pin)")
                        .as_str()
                        .parse()
                        .unwrap_or_else(|_| {
                            panic!(
                                "build.rs: failed to parse bcm_pin as u32 in: {}",
                                tuple_buf
                            )
                        });
                    let cvm_pin = caps
                        .get(6)
                        .expect("pin_re group 6 (cvm_pin)")
                        .as_str()
                        .to_string();
                    let tegra_soc_pin = caps
                        .get(7)
                        .expect("pin_re group 7 (tegra_soc_pin)")
                        .as_str()
                        .to_string();
                    let pwm_chip_dir_str = caps
                        .get(8)
                        .expect("pin_re group 8 (pwm_chip_dir)")
                        .as_str();
                    let pwm_id_str = caps
                        .get(9)
                        .expect("pin_re group 9 (pwm_id)")
                        .as_str();
                    let padctl_str = caps.get(10).map(|m| m.as_str());

                    let pwm_chip_dir = if pwm_chip_dir_str == "None" {
                        None
                    } else {
                        Some(
                            pwm_chip_dir_str
                                .trim_matches(|c| c == '"' || c == '\'')
                                .to_string(),
                        )
                    };
                    let pwm_id = if pwm_id_str == "None" {
                        None
                    } else {
                        Some(pwm_id_str.parse().unwrap_or_else(|_| {
                            panic!(
                                "build.rs: failed to parse pwm_id as u32 in: {}",
                                tuple_buf
                            )
                        }))
                    };
                    let padctl_addr = match padctl_str {
                        Some(s) if s != "None" && !s.is_empty() => Some(
                            u32::from_str_radix(s.trim_start_matches("0x"), 16)
                                .unwrap_or_else(|_| {
                                    panic!(
                                        "build.rs: failed to parse padctl_addr as hex u32 in: {}",
                                        tuple_buf
                                    )
                                }),
                        ),
                        _ => None,
                    };

                    if let Some(ref arr_name) = current_array {
                        pin_defs_map
                            .entry(arr_name.clone())
                            .or_default()
                            .push(PinDef {
                                linux_gpio,
                                gpio_name,
                                gpio_chip,
                                board_pin,
                                bcm_pin,
                                cvm_pin,
                                tegra_soc_pin,
                                pwm_chip_dir,
                                pwm_id,
                                padctl_addr,
                            });
                    }
                }
                in_tuple = false;
                tuple_buf.clear();
            }
        }
    }

    // ── 3. Parse compat tuples ──────────────────────────────────────────
    let compat_header_re =
        regex::Regex::new(r"^(compats_[a-z0-9_]+)\s*=\s*\(").expect("compat_header_re");
    let compat_val_re =
        regex::Regex::new(r#"['"]([^'"]+)['"]"#).expect("compat_val_re");
    let mut compats_map: HashMap<String, Vec<String>> = HashMap::new();
    let mut current_compat: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = compat_header_re.captures(trimmed) {
            current_compat = Some(
                caps.get(1)
                    .expect("compat_header_re group 1")
                    .as_str()
                    .to_string(),
            );
            let vals: Vec<String> = compat_val_re
                .captures_iter(trimmed)
                .map(|m| {
                    m.get(1)
                        .expect("compat_val_re group 1")
                        .as_str()
                        .to_string()
                })
                .collect();
            if let Some(ref name) = current_compat {
                compats_map
                    .entry(name.clone())
                    .or_default()
                    .extend(vals);
            }
            if trimmed.contains(')') {
                current_compat = None;
            }
            continue;
        }

        if current_compat.is_some() {
            let vals: Vec<String> = compat_val_re
                .captures_iter(trimmed)
                .map(|m| {
                    m.get(1)
                        .expect("compat_val_re group 1")
                        .as_str()
                        .to_string()
                })
                .collect();
            if let Some(ref name) = current_compat {
                compats_map
                    .entry(name.clone())
                    .or_default()
                    .extend(vals);
            }
            if trimmed.contains(')') {
                current_compat = None;
            }
        }
    }

    // ── 4. Parse jetson_gpio_data dict for metadata ─────────────────────
    let model_entry_re = regex::Regex::new(r"(CLARA_AGX_XAVIER|JETSON_[A-Z0-9_]+):\s*\(")
        .expect("model_entry_re");
    let pin_defs_ref_re =
        regex::Regex::new(r"([A-Z][A-Z0-9_]*_PIN_DEFS)\s*,").expect("pin_defs_ref_re");
    let metadata_field_re =
        regex::Regex::new(r"'([A-Z_]+)':\s*'([^']*)'").expect("metadata_field_re");
    let metadata_field_num_re =
        regex::Regex::new(r"'([A-Z_]+)':\s*(\d+)").expect("metadata_field_num_re");

    let mut model_metadata: HashMap<
        String,
        (String, std::collections::HashMap<String, String>),
    > = HashMap::new();
    let mut in_gpio_data = false;
    let mut current_model_name: Option<String> = None;
    let mut current_pin_defs_ref: Option<String> = None;
    let mut current_metadata: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut in_metadata_dict = false;
    let mut brace_depth: i32 = 0;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("jetson_gpio_data") {
            in_gpio_data = true;
            brace_depth = 0;
            for ch in trimmed.chars() {
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth -= 1;
                }
            }
            continue;
        }

        if !in_gpio_data {
            continue;
        }

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
                        if let (Some(mn), Some(pdr)) =
                            (current_model_name.take(), current_pin_defs_ref.take())
                        {
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

        if let Some(caps) = model_entry_re.captures(trimmed) {
            current_model_name = Some(
                caps.get(1)
                    .expect("model_entry_re group 1")
                    .as_str()
                    .to_string(),
            );
            current_pin_defs_ref = None;
            current_metadata.clear();
            if let Some(m) = pin_defs_ref_re.captures(trimmed) {
                current_pin_defs_ref = Some(
                    m.get(1)
                        .expect("pin_defs_ref_re group 1")
                        .as_str()
                        .to_string(),
                );
            }
            continue;
        }

        if current_model_name.is_some() && current_pin_defs_ref.is_none() {
            if let Some(m) = pin_defs_ref_re.captures(trimmed) {
                current_pin_defs_ref = Some(
                    m.get(1)
                        .expect("pin_defs_ref_re group 1")
                        .as_str()
                        .to_string(),
                );
            }
            continue;
        }

        if in_metadata_dict {
            for m in metadata_field_re.captures_iter(trimmed) {
                current_metadata.insert(
                    m.get(1).expect("metadata_field_re group 1").as_str().to_string(),
                    m.get(2).expect("metadata_field_re group 2").as_str().to_string(),
                );
            }
            for m in metadata_field_num_re.captures_iter(trimmed) {
                current_metadata.insert(
                    m.get(1)
                        .expect("metadata_field_num_re group 1")
                        .as_str()
                        .to_string(),
                    m.get(2)
                        .expect("metadata_field_num_re group 2")
                        .as_str()
                        .to_string(),
                );
            }
        }
    }

    // ── 5. Auto-derive compat→model mapping from Python's get_model() ────
    let compat_to_model = parse_compat_model_mapping(content);

    // ── 6. Build ModelData for each model ───────────────────────────────
    let mut models: Vec<ModelData> = Vec::new();

    for model_const in &model_order {
        let (pin_defs_var, meta) = model_metadata
            .get(model_const.as_str())
            .cloned()
            .unwrap_or_else(|| {
                (
                    format!("{}_PIN_DEFS", model_const),
                    std::collections::HashMap::new(),
                )
            });

        let pin_defs = pin_defs_map
            .get(&pin_defs_var)
            .cloned()
            .unwrap_or_default();

        // Find compat variable name for this model via auto-derived mapping
        let compat_var = compat_to_model
            .iter()
            .find(|(_, v)| *v == model_const.as_str())
            .map(|(k, _)| k.clone());
        let compats = compat_var
            .as_ref()
            .and_then(|cv| compats_map.get(cv).cloned())
            .unwrap_or_default();

        // Derive the Rust compat function name from the compat variable name
        // e.g. "compats_tx1" → "get_compats_tx1"
        let compat_func_name = compat_var
            .map(|cv| format!("get_{}", cv))
            .unwrap_or_else(|| format!("get_compats_{}", model_const.to_lowercase()));

        models.push(ModelData {
            model_const_name: model_const.clone(),
            func_name: model_const.to_lowercase(),
            pin_defs,
            compats,
            compat_func_name,
            p1_revision: meta
                .get("P1_REVISION")
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            ram: meta.get("RAM").cloned().unwrap_or_default(),
            revision: meta
                .get("REVISION")
                .cloned()
                .unwrap_or_else(|| "Unknown".to_string()),
            model_type: meta.get("TYPE").cloned().unwrap_or_default(),
            manufacturer: meta
                .get("MANUFACTURER")
                .cloned()
                .unwrap_or_else(|| "NVIDIA".to_string()),
            processor: meta.get("PROCESSOR").cloned().unwrap_or_default(),
        });
    }

    ParsedPython { models, model_order }
}

/// Parse Python's `get_model()` function body to automatically derive the
/// mapping from compat variable names to model constant names.
///
/// Python's `get_model()` follows a regular pattern:
///     if matches(compats_tx1()):
///         ... return JETSON_TX1
///     elif matches(compats_nx()):
///         ... return JETSON_NX
///
/// We extract (compat_var_name, MODEL_CONST) pairs from this structure.
fn parse_compat_model_mapping(content: &str) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();

    // Find the get_model() function body
    let func_start = content
        .find("def get_model()")
        .expect("build.rs: cannot find 'def get_model()' in Python source");

    // Find the next top-level function or class definition (marks end of get_model)
    let func_body = &content[func_start..];
    let func_end = func_body[1..] // skip the 'def' line itself
        .find("\ndef ")
        .unwrap_or(func_body.len());

    let body = &func_body[..func_end.min(func_body.len())];

    // Match patterns like: compats_xxx ... return MODEL_NAME
    // The compat name and return may be separated by warn_if_not_carrier_board etc.
    // In the Python source, compat names are used as bare variable references:
    //   if matches(compats_tx1):
    let re = regex::Regex::new(r"\b(compats_[a-z0-9_]+)\b")
        .expect("compat_call_re");

    // We walk through the body and maintain a stack: each time we see a
    // compat call, we record it; each time we see a `return MODEL_NAME`,
    // we pop the most recent compat call.
    let return_re =
        regex::Regex::new(r"return\s+([A-Z][A-Z0-9_]+)").expect("return_re");

    let mut pending_compat: Option<String> = None;

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }

        if let Some(caps) = re.captures(trimmed) {
            let compat_name = caps.get(1).expect("compat call group 1").as_str().to_string();
            pending_compat = Some(compat_name);
        }

        if let Some(caps) = return_re.captures(trimmed) {
            let model_name = caps.get(1).expect("return group 1").as_str().to_string();
            if let Some(compat_name) = pending_compat.take() {
                map.insert(compat_name, model_name);
            }
        }
    }

    assert!(
        !map.is_empty(),
        "build.rs: failed to auto-derive compat→model mapping from Python get_model()"
    );

    map
}
