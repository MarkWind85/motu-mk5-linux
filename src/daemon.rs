use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Result;
use log::{error, info, warn};

use motu_mk5::audio::discovery::discover_alsa_nodes;
use motu_mk5::audio::router::AudioRouter;
use motu_mk5::device::state::DeviceManager;

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
                error!("audio router failed to start: {e}. Check that pw-loopback is installed (part of pipewire package).");
            }
            Some(r)
        }
        None => {
            error!("MOTU ALSA nodes not found after 10 attempts. Audio routing unavailable \
                    until the device appears — will keep checking every 30s. \
                    Check that PipeWire is running and the device is connected.");
            None
        }
    };

    let mut next_discovery = Instant::now() + DISCOVERY_RETRY_INTERVAL;

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        maintain_router(&mut router, &mut next_discovery);

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
                            error!("device connection lost: {e}. Will attempt to reconnect.");
                            if let Err(e) = mgr.save_state() {
                                error!("failed to save state before reconnect: {e}");
                            }
                            break;
                        }
                    }

                    maintain_router(&mut router, &mut next_discovery);

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
                info!("retrying in 5s. If this persists, run 'motu-ctl diagnose'.");
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

const DISCOVERY_RETRY_INTERVAL: Duration = Duration::from_secs(30);

/// Keep a dead router restarted, and keep looking for the MOTU ALSA nodes if
/// the device wasn't there at startup (late USB enumeration, device powered
/// on after boot). Discovery retries are silent; success is logged.
fn maintain_router(router: &mut Option<AudioRouter>, next_discovery: &mut Instant) {
    match router {
        Some(r) => {
            if !r.is_running() {
                warn!("audio router died, restarting");
                r.stop();
                if let Err(e) = r.start() {
                    error!("audio router restart failed: {e}. Check that pw-loopback is installed.");
                }
            }
        }
        None => {
            if Instant::now() < *next_discovery {
                return;
            }
            *next_discovery = Instant::now() + DISCOVERY_RETRY_INTERVAL;
            if let Ok((alsa_output, alsa_input)) = discover_alsa_nodes() {
                info!("MOTU ALSA nodes appeared, starting audio router");
                info!("ALSA output: {alsa_output}");
                info!("ALSA input:  {alsa_input}");
                let mut r = AudioRouter::new(alsa_output, alsa_input);
                if let Err(e) = r.start() {
                    error!("audio router failed to start: {e}. Check that pw-loopback is installed (part of pipewire package).");
                }
                *router = Some(r);
            }
        }
    }
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
