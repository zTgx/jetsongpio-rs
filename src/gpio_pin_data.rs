use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ChannelInfo {
    pub channel: u32,
    pub chip_fd: Option<i32>,
    pub line_handle: Option<i32>,
    pub line_offset: u32, // This is the Linux GPIO pin number
    pub direction: Option<Direction>,
    pub edge: Option<String>,
    pub consumer: String,
    pub gpio_name: String, // Linux exported GPIO name
    pub gpio_chip: String, // GPIO chip name/instance
    pub pwm_chip_dir: Option<String>,
    pub pwm_id: Option<u32>,
    pub reg_addr: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Direction {
    IN = 0,
    OUT = 1,
    HardPwm = 43,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

// Constants for Jetson models
pub const CLARA_AGX_XAVIER: &str = "CLARA_AGX_XAVIER";
pub const JETSON_NX: &str = "JETSON_NX";
pub const JETSON_XAVIER: &str = "JETSON_XAVIER";
pub const JETSON_TX2: &str = "JETSON_TX2";
pub const JETSON_TX1: &str = "JETSON_TX1";
pub const JETSON_NANO: &str = "JETSON_NANO";
pub const JETSON_TX2_NX: &str = "JETSON_TX2_NX";
pub const JETSON_ORIN: &str = "JETSON_ORIN";
pub const JETSON_ORIN_NX: &str = "JETSON_ORIN_NX";
pub const JETSON_ORIN_NANO: &str = "JETSON_ORIN_NANO";
pub const JETSON_THOR_REFERENCE: &str = "JETSON_THOR_REFERENCE";

pub fn get_jetson_models() -> Vec<&'static str> {
    vec![
        JETSON_TX1,
        JETSON_TX2,
        CLARA_AGX_XAVIER,
        JETSON_TX2_NX,
        JETSON_XAVIER,
        JETSON_NANO,
        JETSON_NX,
        JETSON_ORIN,
        JETSON_ORIN_NX,
        JETSON_ORIN_NANO,
        JETSON_THOR_REFERENCE,
    ]
}

// Pin definitions for different Jetson models
pub fn get_jetson_orin_nx_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            144,
            "PAC.06",
            "tegra234-gpio",
            7,
            4,
            "GPIO09",
            "GP167",
            None,
            None,
            Some(0x2448030),
        ),
        GpioPin::new(
            112,
            "PR.04",
            "tegra234-gpio",
            11,
            17,
            "UART1_RTS",
            "GP72_UART1_RTS_N",
            None,
            None,
            Some(0x2430098),
        ),
        GpioPin::new(
            50,
            "PH.07",
            "tegra234-gpio",
            12,
            18,
            "I2S0_SCLK",
            "GP122",
            None,
            None,
            Some(0x2434088),
        ),
        GpioPin::new(
            122,
            "PY.00",
            "tegra234-gpio",
            13,
            27,
            "SPI1_SCK",
            "GP36_SPI3_CLK",
            None,
            None,
            Some(0x243D030),
        ),
        GpioPin::new(
            85,
            "PN.01",
            "tegra234-gpio",
            15,
            22,
            "GPIO12",
            "GP88_PWM1",
            Some("3280000.pwm"),
            Some(0),
            Some(0x2440020),
        ),
        GpioPin::new(
            126,
            "PY.04",
            "tegra234-gpio",
            16,
            23,
            "SPI1_CS1",
            "GP40_SPI3_CS1_N",
            None,
            None,
            Some(0x243D020),
        ),
        GpioPin::new(
            125,
            "PY.03",
            "tegra234-gpio",
            18,
            24,
            "SPI1_CS0",
            "GP39_SPI3_CS0_N",
            None,
            None,
            Some(0x243D010),
        ),
        GpioPin::new(
            135,
            "PZ.05",
            "tegra234-gpio",
            19,
            10,
            "SPI0_MOSI",
            "GP49_SPI1_MOSI",
            None,
            None,
            Some(0x243D040),
        ),
        GpioPin::new(
            134,
            "PZ.04",
            "tegra234-gpio",
            21,
            9,
            "SPI0_MISO",
            "GP48_SPI1_MISO",
            None,
            None,
            Some(0x243D018),
        ),
        GpioPin::new(
            123,
            "PY.01",
            "tegra234-gpio",
            22,
            25,
            "SPI1_MISO",
            "GP37_SPI3_MISO",
            None,
            None,
            Some(0x243D000),
        ),
        GpioPin::new(
            133,
            "PZ.03",
            "tegra234-gpio",
            23,
            11,
            "SPI0_SCK",
            "GP47_SPI1_CLK",
            None,
            None,
            Some(0x243D028),
        ),
        GpioPin::new(
            136,
            "PZ.06",
            "tegra234-gpio",
            24,
            8,
            "SPI0_CS0",
            "GP50_SPI1_CS0_N",
            None,
            None,
            Some(0x243D008),
        ),
        GpioPin::new(
            137,
            "PZ.07",
            "tegra234-gpio",
            26,
            7,
            "SPI0_CS1",
            "GP51_SPI1_CS1_N",
            None,
            None,
            Some(0x243D038),
        ),
        GpioPin::new(
            105,
            "PQ.05",
            "tegra234-gpio",
            29,
            5,
            "GPIO01",
            "GP65",
            None,
            None,
            Some(0x2430068),
        ),
        GpioPin::new(
            106,
            "PQ.06",
            "tegra234-gpio",
            31,
            6,
            "GPIO11",
            "GP66",
            None,
            None,
            Some(0x2430070),
        ),
        GpioPin::new(
            41,
            "PG.06",
            "tegra234-gpio",
            32,
            12,
            "GPIO07",
            "GP113_PWM7",
            Some("32e0000.pwm"),
            Some(0),
            Some(0x2434080),
        ),
        GpioPin::new(
            43,
            "PH.00",
            "tegra234-gpio",
            33,
            13,
            "GPIO13",
            "GP115",
            Some("32c0000.pwm"),
            Some(0),
            Some(0x2434040),
        ),
        GpioPin::new(
            53,
            "PI.02",
            "tegra234-gpio",
            35,
            19,
            "I2S0_FS",
            "GP125",
            None,
            None,
            Some(0x24340A0),
        ),
        GpioPin::new(
            113,
            "PR.05",
            "tegra234-gpio",
            36,
            16,
            "UART1_CTS",
            "GP73_UART1_CTS_N",
            None,
            None,
            Some(0x2430090),
        ),
        GpioPin::new(
            124,
            "PY.02",
            "tegra234-gpio",
            37,
            26,
            "SPI1_MOSI",
            "GP38_SPI3_MOSI",
            None,
            None,
            Some(0x243D048),
        ),
        GpioPin::new(
            52,
            "PI.01",
            "tegra234-gpio",
            38,
            20,
            "I2S0_SDIN",
            "GP124",
            None,
            None,
            Some(0x2434098),
        ),
        GpioPin::new(
            51,
            "PI.00",
            "tegra234-gpio",
            40,
            21,
            "I2S0_SDOUT",
            "GP123",
            None,
            None,
            Some(0x2434090),
        ),
    ]
}

pub fn get_jetson_orin_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            106,
            "PQ.06",
            "tegra234-gpio",
            7,
            4,
            "MCLK05",
            "GP66",
            None,
            None,
            Some(0x2430070),
        ),
        // Output-only (due to base board)
        GpioPin::new(
            112,
            "PR.04",
            "tegra234-gpio",
            11,
            17,
            "UART1_RTS",
            "GP72_UART1_RTS_N",
            None,
            None,
            Some(0x2430098),
        ),
        GpioPin::new(
            50,
            "PH.07",
            "tegra234-gpio",
            12,
            18,
            "I2S2_CLK",
            "GP122",
            None,
            None,
            Some(0x2434088),
        ),
        GpioPin::new(
            108,
            "PR.00",
            "tegra234-gpio",
            13,
            27,
            "PWM01",
            "GP68",
            Some("32f0000.pwm"),
            Some(0),
            Some(0x2430080),
        ),
        GpioPin::new(
            85,
            "PN.01",
            "tegra234-gpio",
            15,
            22,
            "GPIO27",
            "GP88_PWM1",
            Some("3280000.pwm"),
            Some(0),
            Some(0x2440020),
        ),
        GpioPin::new(
            9,
            "PBB.01",
            "tegra234-gpio-aon",
            16,
            23,
            "GPIO08",
            "GP26",
            None,
            None,
            Some(0xC303048),
        ),
        GpioPin::new(
            43,
            "PH.00",
            "tegra234-gpio",
            18,
            24,
            "GPIO35",
            "GP115",
            Some("32c0000.pwm"),
            Some(0),
            Some(0x2434040),
        ),
        GpioPin::new(
            135,
            "PZ.05",
            "tegra234-gpio",
            19,
            10,
            "SPI1_MOSI",
            "GP49_SPI1_MOSI",
            None,
            None,
            Some(0x243D040),
        ),
        GpioPin::new(
            134,
            "PZ.04",
            "tegra234-gpio",
            21,
            9,
            "SPI1_MISO",
            "GP48_SPI1_MISO",
            None,
            None,
            Some(0x243D018),
        ),
        GpioPin::new(
            96,
            "PP.04",
            "tegra234-gpio",
            22,
            25,
            "GPIO17",
            "GP56",
            None,
            None,
            Some(0x2430020),
        ),
        GpioPin::new(
            133,
            "PZ.03",
            "tegra234-gpio",
            23,
            11,
            "SPI1_CLK",
            "GP47_SPI1_CLK",
            None,
            None,
            Some(0x243D028),
        ),
        GpioPin::new(
            136,
            "PZ.06",
            "tegra234-gpio",
            24,
            8,
            "SPI1_CS0_N",
            "GP50_SPI1_CS0_N",
            None,
            None,
            Some(0x243D008),
        ),
        GpioPin::new(
            137,
            "PZ.07",
            "tegra234-gpio",
            26,
            7,
            "SPI1_CS1_N",
            "GP51_SPI1_CS1_N",
            None,
            None,
            Some(0x243D038),
        ),
        GpioPin::new(
            1,
            "PAA.01",
            "tegra234-gpio-aon",
            29,
            5,
            "CAN0_DIN",
            "GP18_CAN0_DIN",
            None,
            None,
            Some(0xC303018),
        ),
        GpioPin::new(
            0,
            "PAA.00",
            "tegra234-gpio-aon",
            31,
            6,
            "CAN0_DOUT",
            "GP17_CAN0_DOUT",
            None,
            None,
            Some(0xC303010),
        ),
        GpioPin::new(
            8,
            "PBB.00",
            "tegra234-gpio-aon",
            32,
            12,
            "GPIO09",
            "GP25",
            None,
            None,
            Some(0xC303040),
        ),
        GpioPin::new(
            2,
            "PAA.02",
            "tegra234-gpio-aon",
            33,
            13,
            "CAN1_DOUT",
            "GP19_CAN1_DOUT",
            None,
            None,
            Some(0xC303000),
        ),
        GpioPin::new(
            53,
            "PI.02",
            "tegra234-gpio",
            35,
            19,
            "I2S2_FS",
            "GP125",
            None,
            None,
            Some(0x24340A0),
        ),
        GpioPin::new(
            113,
            "PR.05",
            "tegra234-gpio",
            36,
            16,
            "UART1_CTS",
            "GP73_UART1_CTS_N",
            None,
            None,
            Some(0x2430090),
        ),
        GpioPin::new(
            3,
            "PAA.03",
            "tegra234-gpio-aon",
            37,
            26,
            "CAN1_DIN",
            "GP20_CAN1_DIN",
            None,
            None,
            Some(0xC303008),
        ),
        GpioPin::new(
            52,
            "PI.01",
            "tegra234-gpio",
            38,
            20,
            "I2S2_DIN",
            "GP124",
            None,
            None,
            Some(0x2434098),
        ),
        GpioPin::new(
            51,
            "PI.00",
            "tegra234-gpio",
            40,
            21,
            "I2S2_DOUT",
            "GP123",
            None,
            None,
            Some(0x2434090),
        ),
    ]
}

pub fn get_clara_agx_xavier_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            106,
            "PQ.06",
            "tegra194-gpio",
            7,
            4,
            "MCLK05",
            "SOC_GPIO42",
            None,
            None,
            None,
        ),
        GpioPin::new(
            112,
            "PR.04",
            "tegra194-gpio",
            11,
            17,
            "UART1_RTS",
            "UART1_RTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            51,
            "PH.07",
            "tegra194-gpio",
            12,
            18,
            "I2S2_CLK",
            "DAP2_SCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            96,
            "PP.04",
            "tegra194-gpio",
            13,
            27,
            "GPIO32",
            "SOC_GPIO04",
            None,
            None,
            None,
        ),
        // Older versions of L4T don't enable this PWM controller in DT, so this PWM
        // channel may not be available.
        GpioPin::new(
            84,
            "PN.01",
            "tegra194-gpio",
            15,
            22,
            "GPIO27",
            "SOC_GPIO54",
            Some("3280000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            8,
            "PBB.00",
            "tegra194-gpio-aon",
            16,
            23,
            "GPIO8",
            "CAN1_STB",
            None,
            None,
            None,
        ),
        GpioPin::new(
            44,
            "PH.00",
            "tegra194-gpio",
            18,
            24,
            "GPIO35",
            "SOC_GPIO12",
            Some("32c0000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            162,
            "PZ.05",
            "tegra194-gpio",
            19,
            10,
            "SPI1_MOSI",
            "SPI1_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            161,
            "PZ.04",
            "tegra194-gpio",
            21,
            9,
            "SPI1_MISO",
            "SPI1_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            101,
            "PQ.01",
            "tegra194-gpio",
            22,
            25,
            "GPIO17",
            "SOC_GPIO21",
            None,
            None,
            None,
        ),
        GpioPin::new(
            160,
            "PZ.03",
            "tegra194-gpio",
            23,
            11,
            "SPI1_CLK",
            "SPI1_SCK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            163,
            "PZ.06",
            "tegra194-gpio",
            24,
            8,
            "SPI1_CS0_N",
            "SPI1_CS0_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            164,
            "PZ.07",
            "tegra194-gpio",
            26,
            7,
            "SPI1_CS1_N",
            "SPI1_CS1_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            3,
            "PAA.03",
            "tegra194-gpio-aon",
            29,
            5,
            "CAN0_DIN",
            "CAN0_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            2,
            "PAA.02",
            "tegra194-gpio-aon",
            31,
            6,
            "CAN0_DOUT",
            "CAN0_DOUT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            9,
            "PBB.01",
            "tegra194-gpio-aon",
            32,
            12,
            "GPIO9",
            "CAN1_EN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            0,
            "PAA.00",
            "tegra194-gpio-aon",
            33,
            13,
            "CAN1_DOUT",
            "CAN1_DOUT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            54,
            "PI.02",
            "tegra194-gpio",
            35,
            19,
            "I2S2_FS",
            "DAP2_FS",
            None,
            None,
            None,
        ),
        // Input-only (due to base board)
        GpioPin::new(
            113,
            "PR.05",
            "tegra194-gpio",
            36,
            16,
            "UART1_CTS",
            "UART1_CTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            1,
            "PAA.01",
            "tegra194-gpio-aon",
            37,
            26,
            "CAN1_DIN",
            "CAN1_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            53,
            "PI.01",
            "tegra194-gpio",
            38,
            20,
            "I2S2_DIN",
            "DAP2_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            52,
            "PI.00",
            "tegra194-gpio",
            40,
            21,
            "I2S2_DOUT",
            "DAP2_DOUT",
            None,
            None,
            None,
        ),
    ]
}

pub fn get_jetson_nx_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            118,
            "PS.04",
            "tegra194-gpio",
            7,
            4,
            "GPIO09",
            "AUD_MCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            112,
            "PR.04",
            "tegra194-gpio",
            11,
            17,
            "UART1_RTS",
            "UART1_RTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            127,
            "PT.05",
            "tegra194-gpio",
            12,
            18,
            "I2S0_SCLK",
            "DAP5_SCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            149,
            "PY.00",
            "tegra194-gpio",
            13,
            27,
            "SPI1_SCK",
            "SPI3_SCK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            16,
            "PCC.04",
            "tegra194-gpio-aon",
            15,
            22,
            "GPIO12",
            "TOUCH_CLK",
            Some("c340000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            153,
            "PY.04",
            "tegra194-gpio",
            16,
            23,
            "SPI1_CS1",
            "SPI3_CS1_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            152,
            "PY.03",
            "tegra194-gpio",
            18,
            24,
            "SPI1_CS0",
            "SPI3_CS0_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            162,
            "PZ.05",
            "tegra194-gpio",
            19,
            10,
            "SPI0_MOSI",
            "SPI1_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            161,
            "PZ.04",
            "tegra194-gpio",
            21,
            9,
            "SPI0_MISO",
            "SPI1_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            150,
            "PY.01",
            "tegra194-gpio",
            22,
            25,
            "SPI1_MISO",
            "SPI3_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            160,
            "PZ.03",
            "tegra194-gpio",
            23,
            11,
            "SPI0_SCK",
            "SPI1_SCK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            163,
            "PZ.06",
            "tegra194-gpio",
            24,
            8,
            "SPI0_CS0",
            "SPI1_CS0_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            164,
            "PZ.07",
            "tegra194-gpio",
            26,
            7,
            "SPI0_CS1",
            "SPI1_CS1_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            105,
            "PQ.05",
            "tegra194-gpio",
            29,
            5,
            "GPIO01",
            "SOC_GPIO41",
            None,
            None,
            None,
        ),
        GpioPin::new(
            106,
            "PQ.06",
            "tegra194-gpio",
            31,
            6,
            "GPIO11",
            "SOC_GPIO42",
            None,
            None,
            None,
        ),
        GpioPin::new(
            108,
            "PR.00",
            "tegra194-gpio",
            32,
            12,
            "GPIO07",
            "SOC_GPIO44",
            Some("32f0000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            84,
            "PN.01",
            "tegra194-gpio",
            33,
            13,
            "GPIO13",
            "SOC_GPIO54",
            Some("3280000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            130,
            "PU.00",
            "tegra194-gpio",
            35,
            19,
            "I2S0_FS",
            "DAP5_FS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            113,
            "PR.05",
            "tegra194-gpio",
            36,
            16,
            "UART1_CTS",
            "UART1_CTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            151,
            "PY.02",
            "tegra194-gpio",
            37,
            26,
            "SPI1_MOSI",
            "SPI3_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            129,
            "PT.07",
            "tegra194-gpio",
            38,
            20,
            "I2S0_DIN",
            "DAP5_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            128,
            "PT.06",
            "tegra194-gpio",
            40,
            21,
            "I2S0_DOUT",
            "DAP5_DOUT",
            None,
            None,
            None,
        ),
    ]
}

pub fn get_jetson_xavier_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            106,
            "PQ.06",
            "tegra194-gpio",
            7,
            4,
            "MCLK05",
            "SOC_GPIO42",
            None,
            None,
            None,
        ),
        GpioPin::new(
            112,
            "PR.04",
            "tegra194-gpio",
            11,
            17,
            "UART1_RTS",
            "UART1_RTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            51,
            "PH.07",
            "tegra194-gpio",
            12,
            18,
            "I2S2_CLK",
            "DAP2_SCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            108,
            "PR.00",
            "tegra194-gpio",
            13,
            27,
            "PWM01",
            "SOC_GPIO44",
            Some("32f0000.pwm"),
            Some(0),
            None,
        ),
        // Older versions of L4T don't enable this PWM controller in DT, so this PWM
        // channel may not be available.
        GpioPin::new(
            84,
            "PN.01",
            "tegra194-gpio",
            15,
            22,
            "GPIO27",
            "SOC_GPIO54",
            Some("3280000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            8,
            "BB.00",
            "tegra194-gpio-aon",
            16,
            23,
            "GPIO8",
            "CAN1_STB",
            None,
            None,
            None,
        ),
        GpioPin::new(
            44,
            "PH.00",
            "tegra194-gpio",
            18,
            24,
            "GPIO35",
            "SOC_GPIO12",
            Some("32c0000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            162,
            "PZ.05",
            "tegra194-gpio",
            19,
            10,
            "SPI1_MOSI",
            "SPI1_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            161,
            "PZ.04",
            "tegra194-gpio",
            21,
            9,
            "SPI1_MISO",
            "SPI1_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            101,
            "PQ.01",
            "tegra194-gpio",
            22,
            25,
            "GPIO17",
            "SOC_GPIO21",
            None,
            None,
            None,
        ),
        GpioPin::new(
            160,
            "PZ.03",
            "tegra194-gpio",
            23,
            11,
            "SPI1_CLK",
            "SPI1_SCK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            163,
            "PZ.06",
            "tegra194-gpio",
            24,
            8,
            "SPI1_CS0_N",
            "SPI1_CS0_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            164,
            "PZ.07",
            "tegra194-gpio",
            26,
            7,
            "SPI1_CS1_N",
            "SPI1_CS1_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            3,
            "PAA.03",
            "tegra194-gpio-aon",
            29,
            5,
            "CAN0_DIN",
            "CAN0_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            2,
            "PAA.02",
            "tegra194-gpio-aon",
            31,
            6,
            "CAN0_DOUT",
            "CAN0_DOUT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            9,
            "PBB.01",
            "tegra194-gpio-aon",
            32,
            12,
            "GPIO9",
            "CAN1_EN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            0,
            "PAA.00",
            "tegra194-gpio-aon",
            33,
            13,
            "CAN1_DOUT",
            "CAN1_DOUT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            54,
            "PI.02",
            "tegra194-gpio",
            35,
            19,
            "I2S2_FS",
            "DAP2_FS",
            None,
            None,
            None,
        ),
        // Input-only (due to base board)
        GpioPin::new(
            113,
            "PR.05",
            "tegra194-gpio",
            36,
            16,
            "UART1_CTS",
            "UART1_CTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            1,
            "PAA.01",
            "tegra194-gpio-aon",
            37,
            26,
            "CAN1_DIN",
            "CAN1_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            53,
            "PI.01",
            "tegra194-gpio",
            38,
            20,
            "I2S2_DIN",
            "DAP2_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            52,
            "PI.00",
            "tegra194-gpio",
            40,
            21,
            "I2S2_DOUT",
            "DAP2_DOUT",
            None,
            None,
            None,
        ),
    ]
}

pub fn get_jetson_tx2_nx_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            76,
            "PJ.04",
            "tegra-gpio",
            7,
            4,
            "GPIO09",
            "AUD_MCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            28,
            "PW.04",
            "tegra-gpio-aon",
            11,
            17,
            "UART1_RTS",
            "UART3_RTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            72,
            "PJ.00",
            "tegra-gpio",
            12,
            18,
            "I2S0_SCLK",
            "DAP1_SCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            17,
            "PV.01",
            "tegra-gpio-aon",
            13,
            27,
            "SPI1_SCK",
            "GPIO_SEN1",
            None,
            None,
            None,
        ),
        GpioPin::new(
            18,
            "PC.02",
            "tegra-gpio",
            15,
            22,
            "GPIO12",
            "DAP2_DOUT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            19,
            "PC.03",
            "tegra-gpio",
            16,
            23,
            "SPI1_CS1",
            "DAP2_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            20,
            "PV.04",
            "tegra-gpio-aon",
            18,
            24,
            "SPI1_CS0",
            "GPIO_SEN4",
            None,
            None,
            None,
        ),
        GpioPin::new(
            58,
            "PH.02",
            "tegra-gpio",
            19,
            10,
            "SPI0_MOSI",
            "GPIO_WAN7",
            None,
            None,
            None,
        ),
        GpioPin::new(
            57,
            "PH.01",
            "tegra-gpio",
            21,
            9,
            "SPI0_MISO",
            "GPIO_WAN6",
            None,
            None,
            None,
        ),
        GpioPin::new(
            18,
            "PV.02",
            "tegra-gpio-aon",
            22,
            25,
            "SPI1_MISO",
            "GPIO_SEN2",
            None,
            None,
            None,
        ),
        GpioPin::new(
            56,
            "PH.00",
            "tegra-gpio",
            23,
            11,
            "SPI1_CLK",
            "GPIO_WAN5",
            None,
            None,
            None,
        ),
        GpioPin::new(
            59,
            "PH.03",
            "tegra-gpio",
            24,
            8,
            "SPI0_CS0",
            "GPIO_WAN8",
            None,
            None,
            None,
        ),
        GpioPin::new(
            163,
            "PY.03",
            "tegra-gpio",
            26,
            7,
            "SPI0_CS1",
            "GPIO_MDM4",
            None,
            None,
            None,
        ),
        GpioPin::new(
            105,
            "PN.01",
            "tegra-gpio",
            29,
            5,
            "GPIO01",
            "GPIO_CAM2",
            None,
            None,
            None,
        ),
        GpioPin::new(
            50,
            "PEE.02",
            "tegra-gpio-aon",
            31,
            6,
            "GPIO11",
            "TOUCH_CLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            8,
            "PU.00",
            "tegra-gpio-aon",
            32,
            12,
            "GPIO07",
            "GPIO_DIS0",
            Some("3280000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            13,
            "PU.05",
            "tegra-gpio-aon",
            33,
            13,
            "GPIO13",
            "GPIO_DIS5",
            Some("32a0000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            75,
            "PJ.03",
            "tegra-gpio",
            35,
            19,
            "I2S0_FS",
            "DAP1_FS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            29,
            "PW.05",
            "tegra-gpio-aon",
            36,
            16,
            "UART1_CTS",
            "UART3_CTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            19,
            "PV.03",
            "tegra-gpio-aon",
            37,
            26,
            "SPI1_MOSI",
            "GPIO_SEN3",
            None,
            None,
            None,
        ),
        GpioPin::new(
            74,
            "PJ.02",
            "tegra-gpio",
            38,
            20,
            "I2S0_DIN",
            "DAP1_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            73,
            "PJ.01",
            "tegra-gpio",
            40,
            21,
            "I2S0_DOUT",
            "DAP1_DOUT",
            None,
            None,
            None,
        ),
    ]
}

pub fn get_jetson_tx2_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            76,
            "PJ.04",
            "tegra-gpio",
            7,
            4,
            "PAUDIO_MCLK",
            "AUD_MCLK",
            None,
            None,
            None,
        ),
        // Output-only (due to base board)
        GpioPin::new(
            146,
            "PT.02",
            "tegra-gpio",
            11,
            17,
            "PUART0_RTS",
            "UART1_RTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            72,
            "PJ.00",
            "tegra-gpio",
            12,
            18,
            "PI2S0_CLK",
            "DAP1_SCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            77,
            "PJ.05",
            "tegra-gpio",
            13,
            27,
            "PGPIO20_AUD_INT",
            "GPIO_AUD0",
            None,
            None,
            None,
        ),
        GpioPin::new(
            15,
            "GPIO_EXP_P16",
            "tca9539",
            15,
            22,
            "GPIO_EXP_P17",
            "GPIO_EXP_P17",
            None,
            None,
            None,
        ),
        // Input-only (due to module):
        GpioPin::new(
            40,
            "PAA.00",
            "tegra-gpio-aon",
            16,
            23,
            "AO_DMIC_IN_DAT",
            "CAN_GPIO0",
            None,
            None,
            None,
        ),
        GpioPin::new(
            161,
            "PY.01",
            "tegra-gpio",
            18,
            24,
            "GPIO16_MDM_WAKE_AP",
            "GPIO_MDM2",
            None,
            None,
            None,
        ),
        GpioPin::new(
            109,
            "PN.05",
            "tegra-gpio",
            19,
            10,
            "SPI1_MOSI",
            "GPIO_CAM6",
            None,
            None,
            None,
        ),
        GpioPin::new(
            108,
            "PN.04",
            "tegra-gpio",
            21,
            9,
            "SPI1_MISO",
            "GPIO_CAM5",
            None,
            None,
            None,
        ),
        GpioPin::new(
            14,
            "GPIO_EXP_P16",
            "tca9539",
            22,
            25,
            "GPIO_EXP_P16",
            "GPIO_EXP_P16",
            None,
            None,
            None,
        ),
        GpioPin::new(
            107,
            "PN.03",
            "tegra-gpio",
            23,
            11,
            "SPI1_CLK",
            "GPIO_CAM4",
            None,
            None,
            None,
        ),
        GpioPin::new(
            110,
            "PN.06",
            "tegra-gpio",
            24,
            8,
            "SPI1_CS0",
            "GPIO_CAM7",
            None,
            None,
            None,
        ),
        // Board pin 26 is not available on this board
        GpioPin::new(
            78,
            "PJ.06",
            "tegra-gpio",
            29,
            5,
            "GPIO19_AUD_RST",
            "GPIO_AUD1",
            None,
            None,
            None,
        ),
        GpioPin::new(
            42,
            "PAA.02",
            "tegra-gpio-aon",
            31,
            6,
            "GPIO9_MOTION_INT",
            "CAN_GPIO2",
            None,
            None,
            None,
        ),
        // Output-only (due to module):
        GpioPin::new(
            41,
            "PAA.01",
            "tegra-gpio-aon",
            32,
            12,
            "AO_DMIC_IN_CLK",
            "CAN_GPIO1",
            None,
            None,
            None,
        ),
        GpioPin::new(
            69,
            "PI.05",
            "tegra-gpio",
            33,
            13,
            "GPIO11_AP_WAKE_BT",
            "GPIO_PQ5",
            None,
            None,
            None,
        ),
        GpioPin::new(
            75,
            "PJ.03",
            "tegra-gpio",
            35,
            19,
            "I2S0_LRCLK",
            "DAP1_FS",
            None,
            None,
            None,
        ),
        // Input-only (due to base board) IF NVIDIA debug card NOT plugged in
        // Output-only (due to base board) IF NVIDIA debug card plugged in
        GpioPin::new(
            147,
            "PT.03",
            "tegra-gpio",
            36,
            16,
            "UART0_CTS",
            "UART1_CTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            68,
            "PI.04",
            "tegra-gpio",
            37,
            26,
            "GPIO8_ALS_PROX_INT",
            "GPIO_PQ4",
            None,
            None,
            None,
        ),
        GpioPin::new(
            74,
            "PJ.02",
            "tegra-gpio",
            38,
            20,
            "I2S0_SDIN",
            "DAP1_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            73,
            "PJ.01",
            "tegra-gpio",
            40,
            21,
            "I2S0_SDOUT",
            "DAP1_DOUT",
            None,
            None,
            None,
        ),
    ]
}

pub fn get_jetson_tx1_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            216,
            "",
            "tegra-gpio",
            7,
            4,
            "AUDIO_MCLK",
            "AUD_MCLK",
            None,
            None,
            None,
        ),
        // Output-only (due to base board)
        GpioPin::new(
            162,
            "",
            "tegra-gpio",
            11,
            17,
            "UART0_RTS",
            "UART1_RTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            11,
            "",
            "tegra-gpio",
            12,
            18,
            "I2S0_CLK",
            "DAP1_SCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            38,
            "",
            "tegra-gpio",
            13,
            27,
            "GPIO20_AUD_INT",
            "GPIO_PE6",
            None,
            None,
            None,
        ),
        GpioPin::new(
            15,
            "",
            "tca9539",
            15,
            22,
            "GPIO_EXP_P17",
            "GPIO_EXP_P17",
            None,
            None,
            None,
        ),
        GpioPin::new(
            37,
            "",
            "tegra-gpio",
            16,
            23,
            "AO_DMIC_IN_DAT",
            "DMIC3_DAT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            184,
            "",
            "tegra-gpio",
            18,
            24,
            "GPIO16_MDM_WAKE_AP",
            "MODEM_WAKE_AP",
            None,
            None,
            None,
        ),
        GpioPin::new(
            16,
            "",
            "tegra-gpio",
            19,
            10,
            "SPI1_MOSI",
            "SPI1_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            17,
            "",
            "tegra-gpio",
            21,
            9,
            "SPI1_MISO",
            "SPI1_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            14,
            "",
            "tca9539",
            22,
            25,
            "GPIO_EXP_P16",
            "GPIO_EXP_P16",
            None,
            None,
            None,
        ),
        GpioPin::new(
            18,
            "",
            "tegra-gpio",
            23,
            11,
            "SPI1_CLK",
            "SPI1_SCK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            19,
            "",
            "tegra-gpio",
            24,
            8,
            "SPI1_CS0",
            "SPI1_CS0",
            None,
            None,
            None,
        ),
        GpioPin::new(
            20,
            "",
            "tegra-gpio",
            26,
            7,
            "SPI1_CS1",
            "SPI1_CS1",
            None,
            None,
            None,
        ),
        GpioPin::new(
            219,
            "",
            "tegra-gpio",
            29,
            5,
            "GPIO19_AUD_RST",
            "GPIO_X1_AUD",
            None,
            None,
            None,
        ),
        GpioPin::new(
            186,
            "",
            "tegra-gpio",
            31,
            6,
            "GPIO9_MOTION_INT",
            "MOTION_INT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            36,
            "",
            "tegra-gpio",
            32,
            12,
            "AO_DMIC_IN_CLK",
            "DMIC3_CLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            63,
            "",
            "tegra-gpio",
            33,
            13,
            "GPIO11_AP_WAKE_BT",
            "AP_WAKE_NFC",
            None,
            None,
            None,
        ),
        GpioPin::new(
            8,
            "",
            "tegra-gpio",
            35,
            19,
            "I2S0_LRCLK",
            "DAP1_FS",
            None,
            None,
            None,
        ),
        // Input-only (due to base board) IF NVIDIA debug card NOT plugged in
        // Input-only (due to base board) (always reads fixed value) IF NVIDIA debug card plugged in
        GpioPin::new(
            163,
            "",
            "tegra-gpio",
            36,
            16,
            "UART0_CTS",
            "UART1_CTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            187,
            "",
            "tegra-gpio",
            37,
            26,
            "GPIO8_ALS_PROX_INT",
            "ALS_PROX_INT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            9,
            "",
            "tegra-gpio",
            38,
            20,
            "I2S0_SDIN",
            "DAP1_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            10,
            "",
            "tegra-gpio",
            40,
            21,
            "I2S0_SDOUT",
            "DAP1_DOUT",
            None,
            None,
            None,
        ),
    ]
}

pub fn get_jetson_nano_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            216,
            "",
            "tegra-gpio",
            7,
            4,
            "GPIO9",
            "AUD_MCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            50,
            "",
            "tegra-gpio",
            11,
            17,
            "UART1_RTS",
            "UART2_RTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            79,
            "",
            "tegra-gpio",
            12,
            18,
            "I2S0_SCLK",
            "DAP4_SCLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            14,
            "",
            "tegra-gpio",
            13,
            27,
            "SPI1_SCK",
            "SPI2_SCK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            194,
            "",
            "tegra-gpio",
            15,
            22,
            "GPIO12",
            "LCD_TE",
            None,
            None,
            None,
        ),
        GpioPin::new(
            232,
            "",
            "tegra-gpio",
            16,
            23,
            "SPI1_CS1",
            "SPI2_CS1",
            None,
            None,
            None,
        ),
        GpioPin::new(
            15,
            "",
            "tegra-gpio",
            18,
            24,
            "SPI1_CS0",
            "SPI2_CS0",
            None,
            None,
            None,
        ),
        GpioPin::new(
            16,
            "",
            "tegra-gpio",
            19,
            10,
            "SPI0_MOSI",
            "SPI1_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            17,
            "",
            "tegra-gpio",
            21,
            9,
            "SPI0_MISO",
            "SPI1_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            13,
            "",
            "tegra-gpio",
            22,
            25,
            "SPI1_MISO",
            "SPI2_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            18,
            "",
            "tegra-gpio",
            23,
            11,
            "SPI0_SCK",
            "SPI1_SCK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            19,
            "",
            "tegra-gpio",
            24,
            8,
            "SPI0_CS0",
            "SPI1_CS0",
            None,
            None,
            None,
        ),
        GpioPin::new(
            20,
            "",
            "tegra-gpio",
            26,
            7,
            "SPI0_CS1",
            "SPI1_CS1",
            None,
            None,
            None,
        ),
        GpioPin::new(
            149,
            "",
            "tegra-gpio",
            29,
            5,
            "GPIO01",
            "CAM_AF_EN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            200,
            "",
            "tegra-gpio",
            31,
            6,
            "GPIO11",
            "GPIO_PZ0",
            None,
            None,
            None,
        ),
        // Older versions of L4T have a DT bug which instantiates a bogus device
        // which prevents this library from using this PWM channel.
        GpioPin::new(
            168,
            "",
            "tegra-gpio",
            32,
            12,
            "GPIO07",
            "LCD_BL_PW",
            Some("7000a000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            38,
            "",
            "tegra-gpio",
            33,
            13,
            "GPIO13",
            "GPIO_PE6",
            Some("7000a000.pwm"),
            Some(2),
            None,
        ),
        GpioPin::new(
            76,
            "",
            "tegra-gpio",
            35,
            19,
            "I2S0_FS",
            "DAP4_FS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            51,
            "",
            "tegra-gpio",
            36,
            16,
            "UART1_CTS",
            "UART2_CTS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            12,
            "",
            "tegra-gpio",
            37,
            26,
            "SPI1_MOSI",
            "SPI2_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            77,
            "",
            "tegra-gpio",
            38,
            20,
            "I2S0_DIN",
            "DAP4_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            78,
            "",
            "tegra-gpio",
            40,
            21,
            "I2S0_DOUT",
            "DAP4_DOUT",
            None,
            None,
            None,
        ),
    ]
}

pub fn get_jetson_thor_reference_pin_defs() -> Vec<GpioPin> {
    vec![
        GpioPin::new(
            86,
            "PL.06",
            "tegra264-gpio-main",
            7,
            4,
            "MCLK05",
            "GP130",
            None,
            None,
            None,
        ),
        // Output-only (due to base board)
        GpioPin::new(
            92,
            "PM.04",
            "tegra264-gpio-main",
            11,
            17,
            "UART1_RTS",
            "GP136_UART9_RTS_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            21,
            "PV.06",
            "tegra264-gpio-main",
            12,
            18,
            "I2S2_CLK",
            "GP184_DAP2_CLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            88,
            "PM.00",
            "tegra264-gpio-main",
            13,
            27,
            "PWM01",
            "GP132_PWM9",
            Some("810c610000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            127,
            "PF.07",
            "tegra264-gpio-main",
            15,
            22,
            "GPIO27",
            "GP257_PWM2",
            Some("810c5e0000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            21,
            "PDD.03",
            "tegra264-gpio-aon",
            16,
            23,
            "GPIO08",
            "GP21",
            None,
            None,
            None,
        ),
        GpioPin::new(
            14,
            "PU.07",
            "tegra264-gpio-main",
            18,
            24,
            "GPIO44",
            "GP177_PWM5",
            Some("810c600000.pwm"),
            Some(0),
            None,
        ),
        GpioPin::new(
            73,
            "PK.01",
            "tegra264-gpio-main",
            19,
            10,
            "SPI1_MOSI",
            "GP117_SPI1_MOSI",
            None,
            None,
            None,
        ),
        GpioPin::new(
            72,
            "PK.00",
            "tegra264-gpio-main",
            21,
            9,
            "SPI1_MISO",
            "GP116_SPI1_MISO",
            None,
            None,
            None,
        ),
        GpioPin::new(
            7,
            "PU.00",
            "tegra264-gpio-main",
            22,
            25,
            "I2S7_DIN",
            "GP170",
            None,
            None,
            None,
        ),
        GpioPin::new(
            71,
            "PJ.07",
            "tegra264-gpio-main",
            23,
            11,
            "SPI1_CLK",
            "GP115_SPI1_CLK",
            None,
            None,
            None,
        ),
        GpioPin::new(
            74,
            "PK.02",
            "tegra264-gpio-main",
            24,
            8,
            "SPI1_CS0_N",
            "GP118_SPI1_CS0_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            75,
            "PK.03",
            "tegra264-gpio-main",
            26,
            7,
            "SPI1_CS1_N",
            "GP119_SPI1_CS1_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            1,
            "PAD.01",
            "tegra264-gpio-aon",
            29,
            5,
            "CAN2_DIN",
            "GP211_CAN2_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            0,
            "PAD.00",
            "tegra264-gpio-aon",
            31,
            6,
            "CAN2_DOUT",
            "GP210_CAN2_DOUT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            22,
            "PDD.04",
            "tegra264-gpio-aon",
            32,
            12,
            "GPIO09",
            "GGP22_SOCKET_ID_STRA",
            None,
            None,
            None,
        ),
        GpioPin::new(
            2,
            "PAE.00",
            "tegra264-gpio-aon",
            33,
            13,
            "CAN3_DOUT",
            "GP215_CAN3_DOUT",
            None,
            None,
            None,
        ),
        GpioPin::new(
            24,
            "PW.01",
            "tegra264-gpio-main",
            35,
            19,
            "I2S2_FS",
            "GP187_DAP2_FS",
            None,
            None,
            None,
        ),
        GpioPin::new(
            93,
            "PM.05",
            "tegra264-gpio-main",
            36,
            16,
            "UART1_CTS",
            "GP137_UART9_CTS_N",
            None,
            None,
            None,
        ),
        GpioPin::new(
            3,
            "PAE.01",
            "tegra264-gpio-aon",
            37,
            26,
            "CAN3_DIN",
            "GP216_CAN3_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            23,
            "PW.00",
            "tegra264-gpio-main",
            38,
            20,
            "I2S2_DIN",
            "GP186_DAP2_DIN",
            None,
            None,
            None,
        ),
        GpioPin::new(
            22,
            "PV.07",
            "tegra264-gpio-main",
            40,
            21,
            "I2S2_DOUT",
            "GP185_DAP2_DOUT",
            None,
            None,
            None,
        ),
    ]
}

// Compatibility strings for different models
pub fn get_compats_jetson_orins_nx() -> Vec<&'static str> {
    vec![
        "nvidia,p3509-0000+p3767-0000",
        "nvidia,p3768-0000+p3767-0000",
        "nvidia,p3509-0000+p3767-0001",
        "nvidia,p3768-0000+p3767-0001",
        "nvidia,p3768-0000+p3767-0000-super",
        "nvidia,p3768-0000+p3767-0001-super",
    ]
}

pub fn get_compats_jetson_orins_nano() -> Vec<&'static str> {
    vec![
        "nvidia,p3509-0000+p3767-0003",
        "nvidia,p3768-0000+p3767-0003",
        "nvidia,p3509-0000+p3767-0004",
        "nvidia,p3768-0000+p3767-0004",
        "nvidia,p3509-0000+p3767-0005",
        "nvidia,p3768-0000+p3767-0005",
        "nvidia,p3768-0000+p3767-0005-super",
        "nvidia,p3509-0000+p3767-0005-super",
        "nvidia,p3768-0000+p3767-0003-super",
        "nvidia,p3509-0000+p3767-0003-super",
        "nvidia,p3768-0000+p3767-0004-super",
        "nvidia,p3509-0000+p3767-0004-super",
    ]
}

pub fn get_compats_jetson_orins() -> Vec<&'static str> {
    vec![
        "nvidia,p3737-0000+p3701-0000",
        "nvidia,p3737-0000+p3701-0004",
        "nvidia,p3737-0000+p3701-0008",
        "nvidia,p3737-0000+p3701-0005",
        "nvidia,p3737-0000+p3701-0001",
    ]
}

pub fn get_compats_clara_agx_xavier() -> Vec<&'static str> {
    vec!["nvidia,e3900-0000+p2888-0004"]
}

pub fn get_compats_nx() -> Vec<&'static str> {
    vec![
        "nvidia,p3509-0000+p3668-0000",
        "nvidia,p3509-0000+p3668-0001",
        "nvidia,p3449-0000+p3668-0000",
        "nvidia,p3449-0000+p3668-0001",
        "nvidia,p3449-0000+p3668-0003",
    ]
}

pub fn get_compats_xavier() -> Vec<&'static str> {
    vec![
        "nvidia,p2972-0000",
        "nvidia,p2972-0006",
        "nvidia,jetson-xavier",
        "nvidia,galen-industrial",
        "nvidia,jetson-xavier-industrial",
    ]
}

pub fn get_compats_tx2_nx() -> Vec<&'static str> {
    vec!["nvidia,p3509-0000+p3636-0001"]
}

pub fn get_compats_tx2() -> Vec<&'static str> {
    vec![
        "nvidia,p2771-0000",
        "nvidia,p2771-0888",
        "nvidia,p3489-0000",
        "nvidia,lightning",
        "nvidia,quill",
        "nvidia,storm",
    ]
}

pub fn get_compats_tx1() -> Vec<&'static str> {
    vec!["nvidia,p2371-2180", "nvidia,jetson-cv"]
}

pub fn get_compats_nano() -> Vec<&'static str> {
    vec![
        "nvidia,p3450-0000",
        "nvidia,p3450-0002",
        "nvidia,jetson-nano",
    ]
}

pub fn get_compats_jetson_thor_reference() -> Vec<&'static str> {
    vec![
        "nvidia,p3971-0050+p3834-0005",
        "nvidia,p3971-0080+p3834-0008",
        "nvidia,p3971-0089+p3834-0008",
        "nvidia,p4071-0000+p3834-0008",
    ]
}

#[derive(Debug, Clone)]
pub struct GpioPin {
    pub linux_gpio: u32,       // This is the line_offset
    pub gpio_name: String,     // Linux exported GPIO name
    pub gpio_chip: String,     // GPIO chip name/instance
    pub board_pin: u32,        // Pin number (BOARD mode)
    pub bcm_pin: u32,          // Pin number (BCM mode)
    pub cvm_pin: String,       // Pin name (CVM mode)
    pub tegra_soc_pin: String, // Pin name (TegraSoc mode)
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

pub fn get_data() -> (String, JetsonInfo, HashMap<Mode, HashMap<u32, ChannelInfo>>) {
    let model = get_model().unwrap();

    let (pin_defs, jetson_info) = get_jetson_data(&model);
    let mut all_modes = HashMap::new();

    // Create channel mappings for different modes
    let board_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| {
            (
                pin.board_pin,
                ChannelInfo {
                    channel: pin.board_pin,
                    chip_fd: None,
                    line_handle: None,
                    line_offset: pin.linux_gpio,
                    direction: None,
                    edge: None,
                    consumer: "Jetson-gpio".to_string(),
                    gpio_name: pin.gpio_name.clone(),
                    gpio_chip: pin.gpio_chip.clone(),
                    pwm_chip_dir: pin.pwm_chip_sysfs_dir.clone(),
                    pwm_id: pin.pwm_id,
                    reg_addr: pin.padctl_addr,
                },
            )
        })
        .collect();

    let bcm_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| {
            (
                pin.bcm_pin,
                ChannelInfo {
                    channel: pin.bcm_pin,
                    chip_fd: None,
                    line_handle: None,
                    line_offset: pin.linux_gpio,
                    direction: None,
                    edge: None,
                    consumer: "Jetson-gpio".to_string(),
                    gpio_name: pin.gpio_name.clone(),
                    gpio_chip: pin.gpio_chip.clone(),
                    pwm_chip_dir: pin.pwm_chip_sysfs_dir.clone(),
                    pwm_id: pin.pwm_id,
                    reg_addr: pin.padctl_addr,
                },
            )
        })
        .collect();

    let cvm_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| {
            let key = hash_string(&pin.cvm_pin) as u32;
            (
                key,
                ChannelInfo {
                    channel: key,
                    chip_fd: None,
                    line_handle: None,
                    line_offset: pin.linux_gpio,
                    direction: None,
                    edge: None,
                    consumer: "Jetson-gpio".to_string(),
                    gpio_name: pin.gpio_name.clone(),
                    gpio_chip: pin.gpio_chip.clone(),
                    pwm_chip_dir: pin.pwm_chip_sysfs_dir.clone(),
                    pwm_id: pin.pwm_id,
                    reg_addr: pin.padctl_addr,
                },
            )
        })
        .collect();

    let tegra_soc_channels: HashMap<u32, ChannelInfo> = pin_defs
        .iter()
        .map(|pin| {
            let key = hash_string(&pin.tegra_soc_pin) as u32;
            (
                key,
                ChannelInfo {
                    channel: key,
                    chip_fd: None,
                    line_handle: None,
                    line_offset: pin.linux_gpio,
                    direction: None,
                    edge: None,
                    consumer: "Jetson-gpio".to_string(),
                    gpio_name: pin.gpio_name.clone(),
                    gpio_chip: pin.gpio_chip.clone(),
                    pwm_chip_dir: pin.pwm_chip_sysfs_dir.clone(),
                    pwm_id: pin.pwm_id,
                    reg_addr: pin.padctl_addr,
                },
            )
        })
        .collect();

    all_modes.insert(Mode::BOARD, board_channels);
    all_modes.insert(Mode::BCM, bcm_channels);
    all_modes.insert(Mode::CVM, cvm_channels);
    all_modes.insert(Mode::TegraSoc, tegra_soc_channels);

    (model, jetson_info, all_modes)
}

fn get_jetson_data(model: &str) -> (Vec<GpioPin>, JetsonInfo) {
    match model {
        JETSON_ORIN_NX | JETSON_ORIN_NANO => (
            get_jetson_orin_nx_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "32768M, 65536M".to_string(),
                revision: "Unknown".to_string(),
                r#type: if model == JETSON_ORIN_NANO {
                    "JETSON_ORIN_NANO".to_string()
                } else {
                    "JETSON_ORIN_NX".to_string()
                },
                manufacturer: "NVIDIA".to_string(),
                processor: "A78AE".to_string(),
            },
        ),
        JETSON_ORIN => (
            get_jetson_orin_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "32768M, 65536M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "JETSON_ORIN".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "A78AE".to_string(),
            },
        ),
        CLARA_AGX_XAVIER => (
            get_clara_agx_xavier_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "16384M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "CLARA_AGX_XAVIER".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM Carmel".to_string(),
            },
        ),
        JETSON_NX => (
            get_jetson_nx_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "16384M, 8192M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "Jetson NX".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM Carmel".to_string(),
            },
        ),
        JETSON_XAVIER => (
            get_jetson_xavier_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "65536M, 32768M, 16384M, 8192M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "Jetson Xavier".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM Carmel".to_string(),
            },
        ),
        JETSON_TX2_NX => (
            get_jetson_tx2_nx_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "4096M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "Jetson TX2 NX".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM A57 + Denver".to_string(),
            },
        ),
        JETSON_TX2 => (
            get_jetson_tx2_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "8192M, 4096M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "Jetson TX2".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM A57 + Denver".to_string(),
            },
        ),
        JETSON_TX1 => (
            get_jetson_tx1_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "4096M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "Jetson TX1".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM A57".to_string(),
            },
        ),
        JETSON_NANO => (
            get_jetson_nano_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "4096M, 2048M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "Jetson Nano".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM A57".to_string(),
            },
        ),
        JETSON_THOR_REFERENCE => (
            get_jetson_thor_reference_pin_defs(),
            JetsonInfo {
                p1_revision: 1,
                ram: "4096M, 2048M".to_string(),
                revision: "Unknown".to_string(),
                r#type: "Jetson THOR REFERENCE".to_string(),
                manufacturer: "NVIDIA".to_string(),
                processor: "ARM A57".to_string(),
            },
        ),
        _ => {
            // Default to Nano if model not recognized
            (
                get_jetson_nano_pin_defs(),
                JetsonInfo {
                    p1_revision: 1,
                    ram: "4096M, 2048M".to_string(),
                    revision: "Unknown".to_string(),
                    r#type: "Jetson Nano".to_string(),
                    manufacturer: "NVIDIA".to_string(),
                    processor: "ARM A57".to_string(),
                },
            )
        }
    }
}

use std::fs;
use std::io::Read;
use std::path::Path;
/// 读取设备树兼容性字符串
/// 兼容性字符串是以 '\0' 分隔的字符串列表
fn get_compatibles(path: &str) -> Result<Vec<String>, String> {
    let mut file = fs::File::open(path).map_err(|e| format!("Failed to open {}: {}", path, e))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("Failed to read {}: {}", path, e))?;

    // 按 '\0' 分割并过滤空字符串
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
    // 首先检查测试环境变量
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

    // 从设备树获取型号信息
    let compatible_path = "/proc/device-tree/compatible";
    if Path::new(compatible_path).exists() {
        let compatibles = get_compatibles(compatible_path)?;

        // TX1
        if matches_any(&compatibles, &get_compats_tx1()) {
            warn_if_not_carrier_board(&["2597"])?;
            return Ok(JETSON_TX1.to_string());
        }
        // TX2
        else if matches_any(&compatibles, &get_compats_tx2()) {
            warn_if_not_carrier_board(&["2597"])?;
            return Ok(JETSON_TX2.to_string());
        }
        // CLARA AGX Xavier
        else if matches_any(&compatibles, &get_compats_clara_agx_xavier()) {
            warn_if_not_carrier_board(&["3900"])?;
            return Ok(CLARA_AGX_XAVIER.to_string());
        }
        // TX2 NX
        else if matches_any(&compatibles, &get_compats_tx2_nx()) {
            warn_if_not_carrier_board(&["3509"])?;
            return Ok(JETSON_TX2_NX.to_string());
        }
        // Xavier
        else if matches_any(&compatibles, &get_compats_xavier()) {
            warn_if_not_carrier_board(&["2822"])?;
            return Ok(JETSON_XAVIER.to_string());
        }
        // Nano
        else if matches_any(&compatibles, &get_compats_nano()) {
            let module_id = find_pmgr_board("3448")?;
            let revision = module_id.split('-').last().unwrap_or("");
            // Revision is an ordered string, not a decimal integer
            if revision < "200" {
                return Err("Jetson Nano module revision must be A02 or later".to_string());
            }
            warn_if_not_carrier_board(&["3449", "3542"])?;
            return Ok(JETSON_NANO.to_string());
        }
        // NX
        else if matches_any(&compatibles, &get_compats_nx()) {
            warn_if_not_carrier_board(&["3509", "3449"])?;
            return Ok(JETSON_NX.to_string());
        }
        // Orin
        else if matches_any(&compatibles, &get_compats_jetson_orins()) {
            warn_if_not_carrier_board(&["3737"])?;
            return Ok(JETSON_ORIN.to_string());
        }
        // Orin NX
        else if matches_any(&compatibles, &get_compats_jetson_orins_nx()) {
            warn_if_not_carrier_board(&["3509", "3768"])?;
            return Ok(JETSON_ORIN_NX.to_string());
        }
        // Orin Nano
        else if matches_any(&compatibles, &get_compats_jetson_orins_nano()) {
            warn_if_not_carrier_board(&["3509", "3768"])?;
            return Ok(JETSON_ORIN_NANO.to_string());
        }
        // Thor Reference
        else if matches_any(&compatibles, &get_compats_jetson_thor_reference()) {
            warn_if_not_carrier_board(&["3971", "4071"])?;
            return Ok(JETSON_THOR_REFERENCE.to_string());
        }
    }

    // 对于 Docker 容器，从环境变量获取型号
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

/// 检查兼容性字符串列表中是否包含任意一个模式
fn matches_any(compatibles: &[String], patterns: &[&str]) -> bool {
    patterns.iter().any(|pattern| {
        compatibles
            .iter()
            .any(|compatible| compatible.contains(pattern))
    })
}

// Helper function to convert string to u32 for use as key in hash maps
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

/// 查找 Plugin Manager 中的板卡 ID
fn find_pmgr_board(prefix: &str) -> Result<String, String> {
    let ids_paths = [
        "/proc/device-tree/chosen/plugin-manager/ids",
        "/proc/device-tree/chosen/ids",
    ];

    for ids_path in ids_paths {
        if Path::new(ids_path).exists() {
            // 检查是否是目录（旧版本内核）
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
            }
            // 检查是否是文件（新版本内核 K510）
            else if Path::new(ids_path).is_file() {
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
