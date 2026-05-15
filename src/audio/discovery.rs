use anyhow::{bail, Context, Result};

pub fn discover_alsa_nodes() -> Result<(String, String)> {
    let output = std::process::Command::new("pw-cli")
        .args(["ls", "Node"])
        .output()
        .context("failed to run pw-cli")?;
    let text = String::from_utf8_lossy(&output.stdout);

    let mut alsa_output = None;
    let mut alsa_input = None;

    for line in text.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("node.name = \"") {
            if let Some(name) = rest.strip_suffix('"') {
                if name.starts_with("alsa_output.usb-MOTU_UltraLite")
                    && name.ends_with("pro-output-0")
                {
                    alsa_output = Some(name.to_string());
                } else if name.starts_with("alsa_input.usb-MOTU_UltraLite")
                    && name.ends_with("pro-input-0")
                {
                    alsa_input = Some(name.to_string());
                }
            }
        }
    }

    match (alsa_output, alsa_input) {
        (Some(out), Some(inp)) => Ok((out, inp)),
        _ => bail!("MOTU ALSA nodes not visible in PipeWire. \
            Device may still be initializing, or pro-audio profile not set. \
            Check 'wpctl status'. Run 'motu-ctl diagnose' for details."),
    }
}
