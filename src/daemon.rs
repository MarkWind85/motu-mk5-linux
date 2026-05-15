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

    let (alsa_output, alsa_input) = discover_alsa_nodes()?;
    info!("ALSA output: {alsa_output}");
    info!("ALSA input:  {alsa_input}");

    let mut router = AudioRouter::new(alsa_output, alsa_input);
    if let Err(e) = router.start() {
        error!("failed to start audio router: {e}");
    }

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        if !router.is_running() {
            warn!("audio router died, restarting");
            router.stop();
            if let Err(e) = router.start() {
                error!("failed to restart audio router: {e}");
            }
        }

        match DeviceManager::connect() {
            Ok(mut mgr) => {
                info!("connected to device, syncing state...");

                thread::sleep(Duration::from_millis(500));
                let received = mgr.sync_from_device()?;
                info!("received {received} properties from device");

                if !mgr.state.values.is_empty() {
                    match mgr.restore_to_device() {
                        Ok(n) => info!("restored {n} saved properties"),
                        Err(e) => warn!("failed to restore state: {e}"),
                    }
                }

                while running.load(Ordering::Relaxed) {
                    let count = mgr.process_incoming();
                    if count > 0 {
                        if let Err(e) = mgr.save_state() {
                            warn!("failed to save state: {e}");
                        }
                    }

                    if !router.is_running() {
                        warn!("audio router died, restarting");
                        router.stop();
                        if let Err(e) = router.start() {
                            error!("failed to restart audio router: {e}");
                        }
                    }

                    thread::sleep(Duration::from_millis(10));
                }

                if let Err(e) = mgr.save_state() {
                    error!("failed to save final state: {e}");
                }
                info!("state saved, shutting down");
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

    router.stop();

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
        let _ = signal::signal(Signal::SIGINT, SigHandler::Handler(handler));
        let _ = signal::signal(Signal::SIGTERM, SigHandler::Handler(handler));
    }

    thread::spawn(move || {
        while !SIGNAL_RECEIVED.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(50));
        }
        running.store(false, Ordering::Relaxed);
    });
}
