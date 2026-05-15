use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use log::{error, info, warn};

use motu_mk5::audio::router::AudioRouter;
use motu_mk5::device::state::DeviceManager;

fn discover_alsa_nodes() -> Result<(String, String)> {
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
        _ => bail!("MOTU ALSA nodes not found in PipeWire — is the device connected?"),
    }
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("motu-mk5d starting");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_handler(r);

    let alsa_nodes = {
        let mut result = None;
        let mut delay = 1u64;
        for attempt in 1..=10 {
            match discover_alsa_nodes() {
                Ok(nodes) => {
                    result = Some(nodes);
                    break;
                }
                Err(e) => {
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }
                    warn!("ALSA discovery attempt {attempt}/10 failed: {e}");
                    info!("retrying in {delay}s...");
                    for _ in 0..(delay * 10) {
                        if !running.load(Ordering::Relaxed) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(100));
                    }
                    delay = (delay * 2).min(16);
                }
            }
        }
        result
    };

    let mut router = match alsa_nodes {
        Some((ref alsa_output, ref alsa_input)) => {
            info!("ALSA output: {alsa_output}");
            info!("ALSA input:  {alsa_input}");
            let mut r = AudioRouter::new(alsa_output.clone(), alsa_input.clone());
            if let Err(e) = r.start() {
                error!("failed to start audio router: {e}");
            }
            Some(r)
        }
        None => {
            error!("MOTU ALSA nodes not found after 10 attempts. Audio routing unavailable. \
                    Check that PipeWire is running and the device is connected.");
            None
        }
    };

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        if let Some(ref mut r) = router {
            if !r.is_running() {
                warn!("audio router died, restarting");
                r.stop();
                if let Err(e) = r.start() {
                    error!("failed to restart audio router: {e}");
                }
            }
        }

        match DeviceManager::connect() {
            Ok(mut mgr) => {
                info!("connected to device, syncing state...");

                thread::sleep(Duration::from_millis(500));
                match mgr.sync_from_device() {
                    Ok(received) => info!("received {received} properties from device"),
                    Err(e) => {
                        error!("lost connection during initial sync: {e}");
                        continue;
                    }
                }

                if !mgr.state.values.is_empty() {
                    match mgr.restore_to_device() {
                        Ok(n) => info!("restored {n} saved properties"),
                        Err(e) => warn!("failed to restore state: {e}"),
                    }
                }

                while running.load(Ordering::Relaxed) {
                    match mgr.process_incoming() {
                        Ok(count) => {
                            if count > 0 {
                                if let Err(e) = mgr.save_state() {
                                    warn!("failed to save state: {e}");
                                }
                            }
                        }
                        Err(e) => {
                            error!("device connection lost: {e}");
                            if let Err(e) = mgr.save_state() {
                                error!("failed to save state before reconnect: {e}");
                            }
                            break;
                        }
                    }

                    if let Some(ref mut r) = router {
                        if !r.is_running() {
                            warn!("audio router died, restarting");
                            r.stop();
                            if let Err(e) = r.start() {
                                error!("failed to restart audio router: {e}");
                            }
                        }
                    }

                    thread::sleep(Duration::from_millis(10));
                }

                if !running.load(Ordering::Relaxed) {
                    if let Err(e) = mgr.save_state() {
                        error!("failed to save final state: {e}");
                    }
                    info!("state saved, shutting down");
                }
            }
            Err(e) => {
                if !running.load(Ordering::Relaxed) {
                    break;
                }
                warn!("device not available: {e}");
                info!("retrying in 5 seconds...");
                for _ in 0..50 {
                    if !running.load(Ordering::Relaxed) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

    if let Some(ref mut r) = router {
        r.stop();
    }

    info!("motu-mk5d stopped");
    Ok(())
}

fn ctrlc_handler(running: Arc<AtomicBool>) {
    use nix::sys::signal::{self, SigHandler, Signal};

    static SIGNAL_RECEIVED: AtomicBool = AtomicBool::new(false);

    extern "C" fn handler(_: i32) {
        SIGNAL_RECEIVED.store(true, Ordering::Relaxed);
    }

    unsafe {
        if let Err(e) = signal::signal(Signal::SIGINT, SigHandler::Handler(handler)) {
            warn!("failed to register SIGINT handler: {e}");
        }
        if let Err(e) = signal::signal(Signal::SIGTERM, SigHandler::Handler(handler)) {
            warn!("failed to register SIGTERM handler: {e}");
        }
    }

    thread::spawn(move || {
        while !SIGNAL_RECEIVED.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(50));
        }
        running.store(false, Ordering::Relaxed);
    });
}
