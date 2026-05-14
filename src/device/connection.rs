use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context, Result};
use log::{info, warn, debug};
use midir::{MidiInput, MidiOutput, MidiInputConnection, MidiOutputConnection};

use crate::protocol::sysex;

const MOTU_PORT_SUBSTR: &str = "UltraLite-mk5";
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

pub struct MidiConnection {
    _input: MidiInputConnection<()>,
    output: MidiOutputConnection,
    rx: mpsc::Receiver<Vec<u8>>,
}

impl MidiConnection {
    pub fn open() -> Result<Self> {
        let (tx, rx) = mpsc::channel();

        let midi_out = MidiOutput::new("motu-mk5-out")
            .context("failed to create MIDI output")?;
        let midi_in = MidiInput::new("motu-mk5-in")
            .context("failed to create MIDI input")?;

        let out_port = find_port(&midi_out, MOTU_PORT_SUBSTR)
            .context("MOTU UltraLite mk5 MIDI output port not found")?;
        let in_port = find_port(&midi_in, MOTU_PORT_SUBSTR)
            .context("MOTU UltraLite mk5 MIDI input port not found")?;

        let output = midi_out
            .connect(&out_port, "motu-mk5")
            .map_err(|e| anyhow::anyhow!("failed to open MIDI output: {e}"))?;

        let input = midi_in
            .connect(
                &in_port,
                "motu-mk5",
                move |_timestamp, message, _| {
                    if message.len() >= 6
                        && message[0] == 0xF0
                        && message[1] == 0x00
                        && message[2] == 0x00
                        && message[3] == 0x3B
                    {
                        let _ = tx.send(message.to_vec());
                    }
                },
                (),
            )
            .map_err(|e| anyhow::anyhow!("failed to open MIDI input: {e}"))?;

        info!("connected to MOTU UltraLite mk5 via MIDI");

        Ok(MidiConnection {
            _input: input,
            output,
            rx,
        })
    }

    pub fn probe(&mut self) -> Result<bool> {
        debug!("sending protocol probe");
        self.output
            .send(&sysex::build_probe())
            .map_err(|e| anyhow::anyhow!("send failed: {e}"))?;

        match self.rx.recv_timeout(CONNECT_TIMEOUT) {
            Ok(msg) => {
                if let Some(parsed) = sysex::parse_message(&msg) {
                    if parsed.request_id == sysex::RequestId::ProtocolProbe {
                        info!("MOTU protocol probe confirmed");
                        return Ok(true);
                    }
                }
                warn!("unexpected response to probe");
                Ok(false)
            }
            Err(_) => {
                warn!("probe timed out");
                Ok(false)
            }
        }
    }

    pub fn enable_api(&mut self) -> Result<()> {
        debug!("enabling SysEx property API");
        self.output
            .send(&sysex::build_enable_api())
            .map_err(|e| anyhow::anyhow!("send failed: {e}"))?;
        info!("SysEx property API enabled");
        Ok(())
    }

    pub fn send_property(&mut self, prop_id: u16, index: u16, data: &[u8]) -> Result<()> {
        let msg = sysex::build_set_property(prop_id, index, data);
        self.output
            .send(&msg)
            .map_err(|e| anyhow::anyhow!("send failed: {e}"))?;
        Ok(())
    }

    pub fn recv(&self) -> Option<Vec<u8>> {
        self.rx.try_recv().ok()
    }

    pub fn recv_timeout(&self, timeout: Duration) -> Option<Vec<u8>> {
        self.rx.recv_timeout(timeout).ok()
    }
}

fn find_port<T: midir::MidiIO>(midi: &T, name_substr: &str) -> Option<T::Port> {
    for port in midi.ports() {
        if let Ok(name) = midi.port_name(&port) {
            debug!("found MIDI port: {name}");
            if name.contains(name_substr) {
                info!("matched MOTU port: {name}");
                return Some(port);
            }
        }
    }
    None
}
