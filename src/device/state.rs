use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use log::{info, debug, warn};
use serde::{Deserialize, Serialize};

use crate::protocol::{properties, types::*};
use super::connection::DeviceConnection;

const STATE_DIR: &str = "motu-mk5";
const STATE_FILE: &str = "device-state.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeviceState {
    pub values: HashMap<String, Vec<PropertyValue>>,
}

impl DeviceState {
    pub fn state_path() -> PathBuf {
        let config_dir = dirs_or_default();
        config_dir.join(STATE_DIR).join(STATE_FILE)
    }

    pub fn load() -> Self {
        let path = Self::state_path();
        match fs::read_to_string(&path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(state) => state,
                Err(e) => {
                    log::warn!("corrupted state file at {}: {e}", path.display());
                    let corrupt = path.with_extension("json.corrupt");
                    if let Err(copy_err) = fs::copy(&path, &corrupt) {
                        log::warn!("failed to back up corrupted state: {copy_err}");
                    } else {
                        log::info!("corrupted state backed up to {}", corrupt.display());
                    }
                    Self::default()
                }
            },
            Err(_) => {
                debug!("no saved state at {}", path.display());
                Self::default()
            }
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::state_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("failed to create state directory")?;
        }
        let json = serde_json::to_string_pretty(self)?;
        fs::write(&path, json)?;
        debug!("state saved to {}", path.display());
        Ok(())
    }

    pub fn set(&mut self, name: &str, index: usize, value: PropertyValue) {
        let entry = self.values.entry(name.to_string()).or_default();
        if index >= entry.len() {
            entry.resize(index + 1, PropertyValue::Byte(0));
        }
        entry[index] = value;
    }

    pub fn get(&self, name: &str, index: usize) -> Option<&PropertyValue> {
        self.values.get(name).and_then(|v| v.get(index))
    }
}

/// Saved values that must never be pushed back to the device. Both encode
/// output attenuation in dB, so a stale value silently changes monitoring
/// level with nothing on screen to explain it.
/// See https://github.com/MarkWind85/motu-mk5-linux/issues/4
const NEVER_RESTORE: &[&str] = &["main_trim", "output_trim"];

/// Past this many differing properties the saved state no longer plausibly
/// describes the attached device, and restoring it would do more harm than
/// leaving the device alone. Sized well above a real repair — recovering a
/// device whose stored settings had been corrupted took 63 writes — because
/// what protects the firmware is the pacing below, not the ceiling.
const MAX_RESTORE_WRITES: usize = 256;

/// The mk5 closes the WebSocket if property writes arrive faster than it
/// applies them.
const WRITE_PACING: Duration = Duration::from_millis(20);

pub struct DeviceManager {
    conn: DeviceConnection,
    pub state: DeviceState,
    /// On-disk state as it was at connect time. `state` is overwritten by
    /// `sync_from_device`, so the saved values need their own copy to stay
    /// comparable.
    desired: DeviceState,
}

impl DeviceManager {
    pub fn connect() -> Result<Self> {
        let conn = DeviceConnection::open()?;
        let state = DeviceState::load();
        let desired = state.clone();
        info!("device manager ready");
        Ok(DeviceManager { conn, state, desired })
    }

    pub fn sync_from_device(&mut self) -> Result<usize> {
        // The device streams property updates continuously while signal is
        // present (meter data), so waiting for a quiet window alone can block
        // forever. The deadline bounds the initial sync; anything that arrives
        // later is folded in by process_incoming().
        let quiet = Duration::from_millis(500);
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut count = 0;

        loop {
            match self.conn.recv_timeout(quiet) {
                Ok(Some((prop_id, index, data))) => {
                    if let Some(def) = properties::find_by_id(prop_id) {
                        if let Some(value) = PropertyValue::decode(def.prop_type, &data) {
                            self.state.set(def.name, index as usize, value);
                            count += 1;
                        }
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => return Err(anyhow::anyhow!(e)),
            }
        }

        if count > 0 {
            debug!("synced {count} properties from device");
        }
        Ok(count)
    }

    /// Writes back only the saved values the device does not already hold.
    ///
    /// Call after `sync_from_device`, which fills `state` with what the device
    /// reports; anything the device never reported is left alone, since there
    /// is no reading to justify a write against.
    pub fn restore_to_device(&mut self) -> Result<usize> {
        let mut pending = Vec::new();

        for def in properties::PROPERTIES {
            if !def.writable || NEVER_RESTORE.contains(&def.name) {
                continue;
            }

            let desired = match self.desired.values.get(def.name) {
                Some(v) => v,
                None => continue,
            };
            let current = match self.state.values.get(def.name) {
                Some(v) => v,
                None => continue,
            };

            for (i, want) in desired.iter().enumerate() {
                if current.get(i) != Some(want) {
                    pending.push((def.id, i as u16, want.encode(), def.name));
                }
            }
        }

        if pending.is_empty() {
            debug!("device already matches saved state");
            return Ok(0);
        }

        if pending.len() > MAX_RESTORE_WRITES {
            warn!(
                "saved state differs from the device in {} properties, over the {MAX_RESTORE_WRITES} \
                 limit — not restoring. The device keeps its own settings. Delete {} to stop \
                 seeing this.",
                pending.len(),
                DeviceState::state_path().display()
            );
            return Ok(0);
        }

        for (id, index, data, name) in &pending {
            self.conn.send_property(*id, *index, data)?;
            debug!("restored {name}[{index}]");
            thread::sleep(WRITE_PACING);
        }

        info!("restored {} properties to device", pending.len());
        Ok(pending.len())
    }

    pub fn set_property(&mut self, name: &str, index: u16, value: PropertyValue) -> Result<()> {
        let def = properties::find_by_name(name)
            .ok_or_else(|| anyhow::anyhow!("unknown property: {name}"))?;

        if !def.writable {
            anyhow::bail!("property {name} is read-only");
        }

        let data = value.encode();
        self.conn.send_property(def.id, index, &data)?;
        self.state.set(name, index as usize, value);
        Ok(())
    }

    pub fn get_property(&self, name: &str, index: u16) -> Option<&PropertyValue> {
        self.state.get(name, index as usize)
    }

    pub fn save_state(&self) -> Result<()> {
        self.state.save()
    }

    pub fn process_incoming(&mut self) -> Result<usize> {
        // Bounded batch: a continuously-streaming device would otherwise keep
        // this drain loop running forever, starving the caller (router upkeep,
        // shutdown flag, state saves).
        let deadline = Instant::now() + Duration::from_millis(100);
        let mut count = 0;
        loop {
            match self.conn.recv() {
                Ok(Some((prop_id, index, data))) => {
                    if let Some(def) = properties::find_by_id(prop_id) {
                        if let Some(value) = PropertyValue::decode(def.prop_type, &data) {
                            self.state.set(def.name, index as usize, value);
                            count += 1;
                        }
                    }
                    if Instant::now() >= deadline {
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => return Err(anyhow::anyhow!(e)),
            }
        }
        Ok(count)
    }
}

fn dirs_or_default() -> PathBuf {
    if let Ok(dir) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(dir)
    } else if let Ok(home) = std::env::var("HOME") {
        Path::new(&home).join(".config")
    } else {
        PathBuf::from("/tmp")
    }
}
