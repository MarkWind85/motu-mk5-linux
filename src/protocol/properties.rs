/// Complete property map for the MOTU UltraLite mk5.
/// Extracted from CueMix5 source (dev.js / dev_common.js).

use super::types::PropertyType;

#[derive(Debug, Clone)]
pub struct PropertyDef {
    pub name: &'static str,
    pub id: u16,
    pub prop_type: PropertyType,
    pub count: usize,
    pub writable: bool,
    pub description: &'static str,
}

// Meters special ID
pub const METERS_ID: u16 = 6000;
pub const PROXY_STATUS_ID: u16 = 0xFFFF;
pub const PROXY_MESSAGE_ID: u16 = 0xFFFE;
pub const DATASTORE_API_VERSION: u16 = 9;

// Mix matrix dimensions
pub const MIX_NUM_INPUTS: usize = 32;
pub const MIX_NUM_BUSES: usize = 14;

macro_rules! prop {
    ($name:expr, $id:expr, $t:expr, $count:expr, $desc:expr) => {
        PropertyDef {
            name: $name,
            id: $id,
            prop_type: $t,
            count: $count,
            writable: true,
            description: $desc,
        }
    };
    ($name:expr, $id:expr, $t:expr, $count:expr, ro, $desc:expr) => {
        PropertyDef {
            name: $name,
            id: $id,
            prop_type: $t,
            count: $count,
            writable: false,
            description: $desc,
        }
    };
}

use PropertyType::*;

pub static PROPERTIES: &[PropertyDef] = &[
    // ---- Device Info ----
    prop!("device_mac",     0,  Array,  1, ro, "Device MAC address (6 bytes)"),
    prop!("device_uid",     2,  Array,  1, ro, "Device unique ID / serial"),
    prop!("device_name",    3,  String, 1,     "User-assigned device name"),
    prop!("device_version", 4,  String, 2, ro, "Firmware version strings"),
    prop!("api_version",    8,  Int16,  1, ro, "Datastore API version"),
    prop!("model_id",       21, String, 1, ro, "Model identifier (ULM5 / 828M5)"),

    // ---- Clock / Sample Rate ----
    prop!("sample_rate",    10, Int32,  1,     "Sample rate in Hz"),
    prop!("clock_source",   11, Byte,   1,     "Clock source: 0=S/PDIF, 2=Optical, 3=Internal"),
    prop!("fpga_locked",    15, Byte,   1, ro, "FPGA lock status"),
    prop!("pll_locked",     16, Byte,   1, ro, "PLL lock status"),

    // ---- MIDI ----
    prop!("midi_thru",      19, Byte,   1,     "MIDI thru enable"),

    // ---- Loopback ----
    prop!("loopback_first", 20, Byte,   1,     "Loopback order: 0=USB 9-10, 1=USB 1-2"),

    // ---- Input Preamp ----
    prop!("input_gain",     5001, Byte, 8,     "Input gain: mic 0-74dB, line 0-20dB"),
    prop!("input_phase",    5002, Byte, 8,     "Input phase invert (bool per channel)"),
    prop!("input_pad",      5003, Byte, 2,     "Mic pad enable (bool, mic inputs only)"),
    prop!("input_48v",      5004, Byte, 2,     "48V phantom power (bool, mic inputs only)"),
    prop!("jack_detect",    5005, Byte, 2, ro, "Jack detect: 0=mic, 1=line (combo jacks)"),

    // ---- Output ----
    prop!("output_trim",    5000, Byte, 12,    "Output trim 0-100 (maps to 0 to -100dB)"),
    prop!("main_trim",      5011, Byte, 1,     "Main output trim 0-100"),
    prop!("main_group",     5012, Int16, 1,    "Main group bitmask (which outputs follow main)"),

    // ---- Routing ----
    prop!("fpga_patch",     5010, Byte, 80,    "FPGA routing patch table"),
    prop!("loopback_source", 5014, Byte, 2,    "Loopback source routing"),

    // ---- Channel Names ----
    prop!("input_names",    6,  String, 18,    "Input channel names (32 bytes each)"),
    prop!("output_names",   7,  String, 22,    "Output channel names (32 bytes each)"),

    // ---- Optical ----
    prop!("optical_mode",   5006, Byte, 2,     "Optical mode: 0=ADAT, 1=TOSlink (in/out)"),
    prop!("optical_expander", 5025, Byte, 1,   "Optical expander enable"),
    prop!("word_clock_out", 5024, Byte, 1,     "Word clock out: 0=Thru, 1=Out"),

    // ---- A/B/Mono/Mute buttons ----
    prop!("ab_enable",      5015, Byte, 1,     "A/B switching enable"),
    prop!("a_enable",       5016, Byte, 1,     "A output enable"),
    prop!("b_enable",       5017, Byte, 1,     "B output enable"),
    prop!("mono_enable",    5018, Byte, 1,     "Mono enable"),
    prop!("mute_enable",    5019, Byte, 1,     "Mute enable"),

    // ---- Optical expander patch ----
    prop!("optical_expander_patch", 5020, Byte, 80, "Optical expander routing table"),

    // ---- Talkback ----
    prop!("talkback_latch",       5026, Byte,  1, "Talkback latch mode"),
    prop!("talkback_enable",      5027, Byte,  1, "Talkback enable"),
    prop!("talkback_level",       5028, Float, 1, "Talkback level"),
    prop!("talkback_dim",         5029, Float, 1, "Talkback dim amount"),
    prop!("talkback_source",      5030, Byte,  1, "Talkback source channel"),
    prop!("talkback_destination", 5031, Int32, 1, "Talkback destination bitmask"),

    // ---- Footswitch ----
    prop!("footswitch_enabled",  5032, Byte,   1, "Footswitch enable"),
    prop!("footswitch_down_key", 5033, String, 1, "Footswitch down action"),
    prop!("footswitch_up_key",   5034, String, 1, "Footswitch up action"),

    // ---- Mixer: Stereo Linking ----
    prop!("mix_input_stereo",  1000, Byte, 32,  "Mix input stereo link (bool per input per rate)"),
    prop!("mix_output_stereo", 1001, Byte, 14,  "Mix output/bus stereo link"),

    // ---- Input EQ (4 bands x 8 channels = 32 entries) ----
    prop!("input_eq_mode",   1002, Byte,  32, "EQ band mode: 0=peak, 1=low shelf, 2=high shelf, 3=highpass"),
    prop!("input_eq_bypass", 1003, Byte,  32, "EQ band bypass (bool)"),
    prop!("input_eq_freq",   1004, Int32, 32, "EQ frequency in Hz"),
    prop!("input_eq_gain",   1005, Float, 32, "EQ gain in dB"),
    prop!("input_eq_bw",     1006, Float, 32, "EQ bandwidth in octaves"),

    // ---- Gate (mic inputs only, 2 channels) ----
    prop!("gate_bypass",    1007, Byte,  2,  "Gate bypass (bool)"),
    prop!("gate_threshold", 1008, Float, 2,  "Gate threshold in dB (0=-inf, 1=0dB)"),
    prop!("gate_attack",    1009, Int16, 2,  "Gate attack 1-100ms"),
    prop!("gate_release",   1010, Int16, 2,  "Gate release 50-2000ms"),

    // ---- Compressor (8 input channels) ----
    prop!("comp_bypass",    1011, Byte,  8,  "Compressor bypass (bool)"),
    prop!("comp_threshold", 1012, Float, 8,  "Compressor threshold in dB"),
    prop!("comp_attack",    1013, Int16, 8,  "Compressor attack 1-100ms"),
    prop!("comp_release",   1014, Int16, 8,  "Compressor release 50-2000ms"),
    prop!("comp_ratio",     1015, Float, 8,  "Compressor ratio 1-10"),
    prop!("comp_makeup",    1021, Float, 8,  "Compressor makeup gain"),

    // ---- Mix Matrix (32 inputs x 14 buses) ----
    prop!("mix_fader", 1016, Float, 448, "Mix fader levels (32 in x 14 bus)"),
    prop!("mix_pan",   1017, Float, 448, "Mix pan positions (32 in x 14 bus, 0.0-1.0)"),
    prop!("mix_solo",  1018, Byte,  448, "Mix solo (bool, 32 in x 14 bus)"),
    prop!("mix_mute",  1019, Byte,  448, "Mix mute (bool, 32 in x 14 bus)"),

    // ---- Output EQ (3 bands x ~12 outputs) ----
    prop!("output_eq_mode",   1022, Byte,  36, "Output EQ mode per band"),
    prop!("output_eq_bypass", 1023, Byte,  36, "Output EQ bypass"),
    prop!("output_eq_freq",   1024, Int32, 36, "Output EQ frequency"),
    prop!("output_eq_gain",   1025, Float, 36, "Output EQ gain"),
    prop!("output_eq_bw",     1026, Float, 36, "Output EQ bandwidth"),

    // ---- Reverb ----
    prop!("reverb_decay",     1030, Byte, 1, "Reverb decay 0-100 (percent)"),
    prop!("reverb_damping",   1031, Byte, 1, "Reverb damping 0-100"),
    prop!("reverb_predelay",  1032, Byte, 1, "Reverb pre-delay 0-100ms"),
    prop!("reverb_width",     1033, Byte, 1, "Reverb width 0-100"),
    prop!("reverb_preset",    1034, Byte, 1, "Reverb preset index"),
];

pub fn find_by_name(name: &str) -> Option<&'static PropertyDef> {
    PROPERTIES.iter().find(|p| p.name == name)
}

pub fn find_by_id(id: u16) -> Option<&'static PropertyDef> {
    PROPERTIES.iter().find(|p| p.id == id)
}
