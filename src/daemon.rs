use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use log::{error, info, warn};

use motu_mk5::device::state::DeviceManager;

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("motu-mk5d starting");

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc_handler(r);

    loop {
        if !running.load(Ordering::Relaxed) {
            break;
        }

        match DeviceManager::connect() {
            Ok(mut mgr) => {
                info!("connected to device, syncing state...");

                // Initial sync: receive current state from device
                thread::sleep(Duration::from_millis(500));
                let received = mgr.sync_from_device()?;
                info!("received {received} properties from device");

                // Restore saved state if we have any
                if !mgr.state.values.is_empty() {
                    match mgr.restore_to_device() {
                        Ok(n) => info!("restored {n} saved properties"),
                        Err(e) => warn!("failed to restore state: {e}"),
                    }
                }

                // Main loop: process incoming property changes
                while running.load(Ordering::Relaxed) {
                    let count = mgr.process_incoming();
                    if count > 0 {
                        if let Err(e) = mgr.save_state() {
                            warn!("failed to save state: {e}");
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
                        return Ok(());
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

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

    // Bridge the static flag to the caller's Arc
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
