use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use log::{info, debug};
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
    fn state_path() -> PathBuf {
        let config_dir = dirs_or_default();
        config_dir.join(STATE_DIR).join(STATE_FILE)
    }

    pub fn load() -> Self {
        let path = Self::state_path();
        match fs::read_to_string(&path) {
            Ok(data) => serde_json::from_str(&data).unwrap_or_default(),
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

pub struct DeviceManager {
    conn: DeviceConnection,
    pub state: DeviceState,
}

impl DeviceManager {
    pub fn connect() -> Result<Self> {
        let conn = DeviceConnection::open()?;
        let state = DeviceState::load();
        info!("device manager ready");
        Ok(DeviceManager { conn, state })
    }

    pub fn sync_from_device(&mut self) -> Result<usize> {
        let timeout = Duration::from_millis(500);
        let mut count = 0;

        while let Some((prop_id, index, data)) = self.conn.recv_timeout(timeout) {
            if let Some(def) = properties::find_by_id(prop_id) {
                if let Some(value) = PropertyValue::decode(def.prop_type, &data) {
                    self.state.set(def.name, index as usize, value);
                    count += 1;
                }
            }
        }

        if count > 0 {
            debug!("synced {count} properties from device");
        }
        Ok(count)
    }

    pub fn restore_to_device(&mut self) -> Result<usize> {
        let mut count = 0;

        for def in properties::PROPERTIES {
            if !def.writable {
                continue;
            }
            if let Some(values) = self.state.values.get(def.name) {
                for (i, val) in values.iter().enumerate() {
                    let data = val.encode();
                    self.conn.send_property(def.id, i as u16, &data)?;
                    count += 1;
                }
            }
        }

        if count > 0 {
            info!("restored {count} properties to device");
        }
        Ok(count)
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

    pub fn process_incoming(&mut self) -> usize {
        let mut count = 0;
        while let Some((prop_id, index, data)) = self.conn.recv() {
            if let Some(def) = properties::find_by_id(prop_id) {
                if let Some(value) = PropertyValue::decode(def.prop_type, &data) {
                    self.state.set(def.name, index as usize, value);
                    count += 1;
                }
            }
        }
        count
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
