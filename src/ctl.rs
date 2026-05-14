use anyhow::Result;
use clap::{Parser, Subcommand};
use motu_mk5::device::state::DeviceManager;
use motu_mk5::protocol::properties;
use motu_mk5::protocol::types::PropertyValue;

#[derive(Parser)]
#[command(name = "motu-ctl", about = "Control the MOTU UltraLite mk5")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get a property value
    Get {
        /// Property name (e.g. input_gain, mix_fader, sample_rate)
        property: String,
        /// Array index (default: 0)
        #[arg(short, long, default_value_t = 0)]
        index: u16,
    },
    /// Set a property value
    Set {
        /// Property name
        property: String,
        /// Array index
        #[arg(short, long, default_value_t = 0)]
        index: u16,
        /// Value to set
        value: String,
    },
    /// List all known properties
    List,
    /// Show current device state
    Status,
    /// Dump full device state as JSON
    Dump,
    /// Probe and identify the device
    Probe,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();

    let cli = Cli::parse();

    match cli.command {
        Commands::List => {
            println!("{:<28} {:>5}  {:<6} {:>2} {:>5}  {}", "NAME", "ID", "TYPE", "RW", "COUNT", "DESCRIPTION");
            println!("{}", "-".repeat(95));
            for p in properties::PROPERTIES {
                let rw = if p.writable { "rw" } else { "ro" };
                println!(
                    "{:<28} {:>5}  {:<6} {:>2} {:>5}  {}",
                    p.name,
                    p.id,
                    format!("{:?}", p.prop_type),
                    rw,
                    p.count,
                    p.description
                );
            }
        }
        Commands::Probe => {
            println!("searching for MOTU UltraLite mk5...");
            let mut mgr = DeviceManager::connect()?;
            println!("device found and protocol confirmed");

            std::thread::sleep(std::time::Duration::from_millis(500));
            let n = mgr.sync_from_device()?;
            println!("received {n} properties");

            if let Some(PropertyValue::String(model)) = mgr.get_property("model_id", 0) {
                println!("model: {model}");
            }
            if let Some(PropertyValue::Int32(sr)) = mgr.get_property("sample_rate", 0) {
                println!("sample rate: {sr} Hz");
            }
            if let Some(PropertyValue::String(name)) = mgr.get_property("device_name", 0) {
                if !name.is_empty() {
                    println!("device name: {name}");
                }
            }
        }
        Commands::Get { property, index } => {
            let mut mgr = DeviceManager::connect()?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            mgr.sync_from_device()?;

            match mgr.get_property(&property, index) {
                Some(val) => println!("{property}[{index}] = {}", format_value(val)),
                None => println!("{property}[{index}] = <not set>"),
            }
        }
        Commands::Set {
            property,
            index,
            value,
        } => {
            let def = properties::find_by_name(&property)
                .ok_or_else(|| anyhow::anyhow!("unknown property: {property}"))?;

            let parsed = parse_value(def.prop_type, &value)?;
            let mut mgr = DeviceManager::connect()?;
            mgr.set_property(&property, index, parsed.clone())?;
            println!("{property}[{index}] = {}", format_value(&parsed));
        }
        Commands::Status => {
            let mut mgr = DeviceManager::connect()?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            mgr.sync_from_device()?;

            print_status(&mgr);
        }
        Commands::Dump => {
            let mut mgr = DeviceManager::connect()?;
            std::thread::sleep(std::time::Duration::from_millis(500));
            mgr.sync_from_device()?;

            let json = serde_json::to_string_pretty(&mgr.state)?;
            println!("{json}");
        }
    }

    Ok(())
}

fn format_value(val: &PropertyValue) -> String {
    match val {
        PropertyValue::Byte(v) => format!("{v}"),
        PropertyValue::Int16(v) => format!("{v}"),
        PropertyValue::Int32(v) => format!("{v}"),
        PropertyValue::Float(v) => format!("{v:.4}"),
        PropertyValue::String(s) => s.clone(),
        PropertyValue::Array(a) => format!("{a:02x?}"),
    }
}

fn parse_value(
    prop_type: motu_mk5::protocol::types::PropertyType,
    s: &str,
) -> Result<PropertyValue> {
    use motu_mk5::protocol::types::PropertyType;
    match prop_type {
        PropertyType::Byte => Ok(PropertyValue::Byte(s.parse()?)),
        PropertyType::Int16 => Ok(PropertyValue::Int16(s.parse()?)),
        PropertyType::Int32 => Ok(PropertyValue::Int32(s.parse()?)),
        PropertyType::Float => Ok(PropertyValue::Float(s.parse()?)),
        PropertyType::String => Ok(PropertyValue::String(s.to_string())),
        PropertyType::Array => anyhow::bail!("cannot set array properties from CLI"),
    }
}

fn print_status(mgr: &DeviceManager) {
    let get_str = |name, idx| -> String {
        match mgr.get_property(name, idx) {
            Some(PropertyValue::String(s)) => s.clone(),
            _ => "?".to_string(),
        }
    };
    let get_int = |name, idx| -> String {
        match mgr.get_property(name, idx) {
            Some(PropertyValue::Int32(v)) => format!("{v}"),
            Some(PropertyValue::Int16(v)) => format!("{v}"),
            Some(PropertyValue::Byte(v)) => format!("{v}"),
            _ => "?".to_string(),
        }
    };

    println!("MOTU UltraLite mk5");
    println!("  model:       {}", get_str("model_id", 0));
    println!("  name:        {}", get_str("device_name", 0));
    println!("  sample rate: {} Hz", get_int("sample_rate", 0));
    println!("  clock:       {}", match mgr.get_property("clock_source", 0) {
        Some(PropertyValue::Byte(0)) => "S/PDIF",
        Some(PropertyValue::Byte(2)) => "Optical",
        Some(PropertyValue::Byte(3)) => "Internal",
        _ => "?",
    });

    println!("\n  Input Gains:");
    for i in 0..8u16 {
        let label = if i < 2 { format!("Mic {}", i + 1) } else { format!("Line {}", i + 1) };
        println!("    {:<8} {} dB", label, get_int("input_gain", i));
    }

    println!("\n  48V: mic1={} mic2={}", get_int("input_48v", 0), get_int("input_48v", 1));
    println!("  Pad: mic1={} mic2={}", get_int("input_pad", 0), get_int("input_pad", 1));
}
