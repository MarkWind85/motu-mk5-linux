use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use log::{error, info, warn};

use motu_mk5::audio::router::AudioRouter;
use motu_mk5::device::state::DeviceManager;

const ALSA_OUTPUT: &str = "alsa_output.usb-MOTU_UltraLite-mk5_ULM5FFE434-00.pro-output-0";
const ALSA_INPUT: &str = "alsa_input.usb-MOTU_UltraLite-mk5_ULM5FFE434-00.pro-input-0";

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("motu-mk5d starting");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_handler(r);

    let mut router = AudioRouter::new(
        ALSA_OUTPUT.to_string(),
        ALSA_INPUT.to_string(),
    );
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
    use std::sync::atomic::AtomicPtr;

    static FLAG: AtomicPtr<AtomicBool> = AtomicPtr::new(std::ptr::null_mut());

    let leaked = Box::into_raw(Box::new(AtomicBool::new(true)));
    FLAG.store(leaked, Ordering::Release);

    let r = running.clone();
    extern "C" fn handler(_: i32) {
        let ptr = FLAG.load(Ordering::Acquire);
        if !ptr.is_null() {
            unsafe { &*ptr }.store(false, Ordering::Relaxed);
        }
    }

    unsafe {
        let _ = signal::signal(Signal::SIGINT, SigHandler::Handler(handler));
        let _ = signal::signal(Signal::SIGTERM, SigHandler::Handler(handler));
    }

    thread::spawn(move || {
        let flag = unsafe { &*FLAG.load(Ordering::Acquire) };
        loop {
            if !flag.load(Ordering::Relaxed) {
                r.store(false, Ordering::Relaxed);
                break;
            }
            thread::sleep(Duration::from_millis(50));
        }
    });
}
