use std::process::{Child, Command};

use anyhow::{Context, Result};
use log::{info, warn};

struct IoPair {
    name: &'static str,
    description: &'static str,
    channel_offset: u32,
    priority: u32,
}

const OUTPUTS: &[IoPair] = &[
    IoPair { name: "motu-main-12",    description: "MOTU Main 1/2",     channel_offset: 0,  priority: 2007 },
    IoPair { name: "motu-line-34",    description: "MOTU Line 3/4",     channel_offset: 2,  priority: 2006 },
    IoPair { name: "motu-line-56",    description: "MOTU Line 5/6",     channel_offset: 4,  priority: 2005 },
    IoPair { name: "motu-line-78",    description: "MOTU Line 7/8",     channel_offset: 6,  priority: 2004 },
    IoPair { name: "motu-line-910",   description: "MOTU Line 9/10",    channel_offset: 8,  priority: 2003 },
    IoPair { name: "motu-phones",     description: "MOTU Phones",       channel_offset: 10, priority: 2002 },
    IoPair { name: "motu-spdif-out",  description: "MOTU S/PDIF Out",   channel_offset: 12, priority: 2001 },
];

const INPUTS: &[IoPair] = &[
    IoPair { name: "motu-mic-12",     description: "MOTU Mic/Line 1/2", channel_offset: 0,  priority: 2005 },
    IoPair { name: "motu-line-in-34", description: "MOTU Line In 3/4",  channel_offset: 2,  priority: 2004 },
    IoPair { name: "motu-line-in-56", description: "MOTU Line In 5/6",  channel_offset: 4,  priority: 2003 },
    IoPair { name: "motu-line-in-78", description: "MOTU Line In 7/8",  channel_offset: 6,  priority: 2002 },
    IoPair { name: "motu-spdif-in",   description: "MOTU S/PDIF In",    channel_offset: 8,  priority: 2001 },
];

pub struct AudioRouter {
    children: Vec<Child>,
    alsa_output: String,
    alsa_input: String,
    sample_rate: u32,
}

impl AudioRouter {
    pub fn new(alsa_output: String, alsa_input: String) -> Self {
        Self {
            children: Vec::new(),
            alsa_output,
            alsa_input,
            sample_rate: 48000,
        }
    }

    pub fn set_sample_rate(&mut self, rate: u32) {
        if rate != self.sample_rate {
            info!("sample rate changed: {} → {}", self.sample_rate, rate);
            self.sample_rate = rate;
            self.enforce_rate();
        }
    }

    fn enforce_rate(&self) {
        info!("enforcing sample rate: {}Hz", self.sample_rate);
        let _ = Command::new("pw-metadata")
            .args(["-n", "settings", "0", "clock.force-rate", &self.sample_rate.to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    fn enforce_alsa_volume(&self) {
        info!("enforcing 100% volume on ALSA sink");
        let _ = Command::new("pactl")
            .args(["set-sink-volume", &self.alsa_output, "100%"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
    }

    pub fn start(&mut self) -> Result<()> {
        info!("starting audio router ({} outputs, {} inputs)", OUTPUTS.len(), INPUTS.len());

        self.enforce_rate();

        for pair in OUTPUTS {
            let child = self.spawn_output(pair)
                .with_context(|| format!("failed to spawn {}", pair.description))?;
            self.children.push(child);
        }

        for pair in INPUTS {
            let child = self.spawn_input(pair)
                .with_context(|| format!("failed to spawn {}", pair.description))?;
            self.children.push(child);
        }

        self.enforce_alsa_volume();

        info!("audio router running: {} loopback processes", self.children.len());
        Ok(())
    }

    pub fn stop(&mut self) {
        info!("stopping audio router");
        for child in &mut self.children {
            if let Err(e) = child.kill() {
                warn!("failed to kill loopback: {e}");
            }
            let _ = child.wait();
        }
        self.children.clear();
    }

    pub fn is_running(&mut self) -> bool {
        self.children.iter_mut().all(|c| {
            c.try_wait().ok().flatten().is_none()
        })
    }

    fn spawn_output(&self, pair: &IoPair) -> Result<Child> {
        let capture_props = format!(
            "{{ media.class=Audio/Sink node.name={} node.description=\"{}\" \
             audio.position=[FL,FR] priority.session={} priority.driver={} }}",
            pair.name, pair.description, pair.priority, pair.priority,
        );
        let playback_props = format!(
            "{{ node.name={}.out stream.dont-remix=true \
             audio.position=[AUX{},AUX{}] \
             target.object={} node.passive=true }}",
            pair.name, pair.channel_offset, pair.channel_offset + 1,
            self.alsa_output,
        );

        info!("  output: {} → AUX{}/AUX{}", pair.description, pair.channel_offset, pair.channel_offset + 1);

        Command::new("pw-loopback")
            .arg("--capture-props")
            .arg(&capture_props)
            .arg("--playback-props")
            .arg(&playback_props)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("pw-loopback not found")
    }

    fn spawn_input(&self, pair: &IoPair) -> Result<Child> {
        let capture_props = format!(
            "{{ node.name={}.in stream.dont-remix=true \
             audio.position=[AUX{},AUX{}] \
             target.object={} node.passive=true }}",
            pair.name, pair.channel_offset, pair.channel_offset + 1,
            self.alsa_input,
        );
        let playback_props = format!(
            "{{ media.class=Audio/Source node.name={} node.description=\"{}\" \
             audio.position=[FL,FR] priority.session={} priority.driver={} }}",
            pair.name, pair.description, pair.priority, pair.priority,
        );

        info!("  input:  AUX{}/AUX{} → {}", pair.channel_offset, pair.channel_offset + 1, pair.description);

        Command::new("pw-loopback")
            .arg("--capture-props")
            .arg(&capture_props)
            .arg("--playback-props")
            .arg(&playback_props)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("pw-loopback not found")
    }
}

impl Drop for AudioRouter {
    fn drop(&mut self) {
        self.stop();
    }
}
