use std::process::Command;

use crate::audio::discovery::discover_alsa_nodes;
use crate::device::connection::DeviceConnection;

fn run_cmd(cmd: &str, args: &[&str]) -> String {
    match Command::new(cmd).args(args).output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            if out.status.success() {
                stdout
            } else if !stderr.is_empty() {
                format!("(exit {}) {}", out.status, stderr)
            } else {
                format!("(exit {})", out.status)
            }
        }
        Err(e) => format!("(not available: {e})"),
    }
}

fn section_system() -> String {
    let kernel = run_cmd("uname", &["-r"]);
    let arch = run_cmd("uname", &["-m"]);

    let distro = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|text| {
            text.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.trim_start_matches("PRETTY_NAME=").trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".into());

    format!(
        "### System\n\
         - Kernel: {kernel}\n\
         - Distro: {distro}\n\
         - Arch: {arch}"
    )
}

fn section_version() -> String {
    format!("### Package\n- Version: {}", env!("CARGO_PKG_VERSION"))
}

fn section_usb() -> String {
    let lsusb = run_cmd("lsusb", &[]);
    let motu_lines: Vec<&str> = lsusb
        .lines()
        .filter(|l| l.contains("07fd") || l.to_lowercase().contains("motu"))
        .collect();

    let usb = if motu_lines.is_empty() {
        "No MOTU USB device found (vendor 07fd)".into()
    } else {
        motu_lines.join("\n")
    };

    format!("### USB Device\n```\n{usb}\n```")
}

fn section_network() -> String {
    let ip_out = run_cmd("ip", &["-br", "addr"]);
    let cdc_lines: Vec<&str> = ip_out
        .lines()
        .filter(|l| l.contains("169.254.") || l.to_lowercase().contains("enx"))
        .collect();

    let net = if cdc_lines.is_empty() {
        "No CDC Ethernet / link-local interface found".into()
    } else {
        cdc_lines.join("\n")
    };

    format!("### Network Interface\n```\n{net}\n```")
}

fn section_pipewire() -> String {
    let pw_info = run_cmd("pw-cli", &["info", "0"]);
    let pw_version: String = pw_info
        .lines()
        .find(|l| l.contains("version"))
        .unwrap_or("version: unknown")
        .trim()
        .into();

    let nodes = run_cmd("pw-cli", &["ls", "Node"]);
    let motu_nodes: Vec<&str> = nodes
        .lines()
        .filter(|l| {
            let t = l.trim();
            t.contains("MOTU") || t.contains("motu")
        })
        .map(|l| l.trim())
        .collect();

    let node_text = if motu_nodes.is_empty() {
        "No MOTU nodes found in PipeWire".into()
    } else {
        motu_nodes.join("\n")
    };

    format!(
        "### PipeWire\n\
         - {pw_version}\n\
         \n```\n{node_text}\n```"
    )
}

fn section_wireplumber() -> String {
    let wpctl = run_cmd("wpctl", &["status"]);
    let motu_lines: Vec<&str> = wpctl
        .lines()
        .filter(|l| l.to_lowercase().contains("motu"))
        .map(|l| l.trim())
        .collect();

    let wp_text = if motu_lines.is_empty() {
        "No MOTU entries in WirePlumber".into()
    } else {
        motu_lines.join("\n")
    };

    format!("### WirePlumber\n```\n{wp_text}\n```")
}

fn section_alsa_discovery() -> String {
    match discover_alsa_nodes() {
        Ok((out, inp)) => format!(
            "### ALSA Discovery\n\
             - Output: `{out}`\n\
             - Input: `{inp}`"
        ),
        Err(e) => format!("### ALSA Discovery\n- Failed: {e}"),
    }
}

fn section_device_connection() -> String {
    match DeviceConnection::open() {
        Ok(conn) => format!(
            "### Device Connection\n\
             - Discovery: OK\n\
             - IP: {}\n\
             - WebSocket: connected",
            conn.device_ip
        ),
        Err(e) => format!("### Device Connection\n- Failed: {e}"),
    }
}

fn section_daemon() -> String {
    let status = run_cmd(
        "systemctl",
        &["--user", "status", "motu-mk5d.service", "--no-pager", "-l"],
    );
    format!("### Daemon Status\n```\n{status}\n```")
}

fn section_logs() -> String {
    let logs = run_cmd(
        "journalctl",
        &["--user-unit", "motu-mk5d", "-n", "30", "--no-pager"],
    );
    format!("### Recent Logs (last 30 lines)\n```\n{logs}\n```")
}

fn section_audio_router() -> String {
    let procs = run_cmd("pgrep", &["-a", "pw-loopback"]);
    let count = if procs.is_empty() {
        0
    } else {
        procs.lines().count()
    };

    let proc_text = if count == 0 {
        "No pw-loopback processes running".into()
    } else {
        format!("{count} pw-loopback processes running")
    };

    format!("### Audio Router\n- {proc_text}")
}

pub fn generate_report() -> String {
    let timestamp = run_cmd("date", &["-Iseconds"]);

    let sections = [
        format!(
            "## motu-mk5 diagnostic report\nGenerated: {timestamp}\n"
        ),
        section_version(),
        section_system(),
        section_usb(),
        section_network(),
        section_pipewire(),
        section_wireplumber(),
        section_alsa_discovery(),
        section_device_connection(),
        section_audio_router(),
        section_daemon(),
        section_logs(),
        "---\n\nTo report an issue, copy this output and paste it into a new issue at:\nhttps://github.com/MarkWind85/motu-mk5-linux/issues/new/choose".to_string(),
    ];

    sections.join("\n\n")
}
