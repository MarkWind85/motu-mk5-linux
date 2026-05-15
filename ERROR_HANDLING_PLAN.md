# Error Handling Overhaul

Three releases, each a minor version bump from 0.2.1:

- **0.3.0** -- Runtime error handling (Pillar A)
- **0.4.0** -- Installation error handling (Pillar B)
- **0.5.0** -- Diagnostics and issue registration (Pillar C)

---

## Pillar A: Runtime Error Handling (0.3.0)

### A1. WebSocket disconnect detection and reconnect

**The problem:** `DeviceConnection::recv()` (`connection.rs:62`) returns `Option` -- callers can't tell "no data yet" from "connection is dead." All errors, including `ConnectionClosed` and `ConnectionReset`, are mapped to `None` and logged at `debug!` level only. When the device disconnects, the daemon's inner loop (`daemon.rs:91-108`) keeps spinning forever, silently doing nothing. State changes on the device are lost. On next daemon restart, stale saved state is pushed back to the device.

**The fix:**

1. Add a `ConnectionError` enum to `connection.rs`:
   - `Disconnected(String)` -- connection closed, reset, broken pipe
   - `Protocol(String)` -- tungstenite protocol errors

2. Change `recv()` return type: `Option<...>` -> `Result<Option<(u16, u16, Vec<u8>)>, ConnectionError>`
   - `Ok(Some(...))` -- got a message
   - `Ok(None)` -- no data (WouldBlock/TimedOut), or ping handled
   - `Err(ConnectionError::Disconnected(...))` -- connection dead

3. Change `DeviceManager::process_incoming()` (`state.rs:139`) return type: `usize` -> `Result<usize>`
   - Propagate `ConnectionError` as `anyhow::Error`

4. In `daemon.rs` inner loop (line 91): on connection error, log at `error!`, save state, break to outer reconnect loop.

**Files:** `src/device/connection.rs`, `src/device/state.rs`, `src/daemon.rs`

### A2. ALSA discovery retry with backoff

**The problem:** `discover_alsa_nodes()` (`daemon.rs:12-43`) runs once at startup. If PipeWire hasn't finished enumerating nodes yet (common at boot), the daemon exits. Systemd brings it back 3 seconds later, but there's no backoff limit, so it can crash-loop.

**The fix:** Retry loop inside `main()` around `discover_alsa_nodes()`. Up to 10 attempts, backoff: 1s, 2s, 4s, ... capped at 16s. Check `running` between retries. If all attempts fail, continue without audio routing (device control still works). Log guidance: "MOTU ALSA nodes not found after 10 attempts. Audio routing unavailable. Check that PipeWire is running and device is connected."

**Files:** `src/daemon.rs`

### A3. Corrupted state file handling

**The problem:** `DeviceState::load()` (`state.rs:30`) uses `serde_json::from_str(&data).unwrap_or_default()`. Corrupted JSON (e.g., partial write during power loss) is silently discarded -- user loses all saved settings with no log entry.

**The fix:** Replace `unwrap_or_default()` with explicit match. On parse error: `warn!` the error and file path, copy corrupted file to `device-state.json.corrupt`, return default state.

**Files:** `src/device/state.rs`

### A4. Atomic audio router start

**The problem:** `AudioRouter::start()` (`router.rs:76-97`) pushes each spawned child to `self.children` as it goes. If spawn #5 fails, children 1-4 are running but the daemon logs the error and continues. Result: partial audio routing with no clear indication which channels are missing. The `is_running()` check then sees incomplete children, triggers restart, which also fails (same error), creating a log-spam loop.

**The fix:** Collect children into a local `Vec<Child>`. If any spawn fails, kill all already-spawned local children, return error. Only move to `self.children` on full success.

**Files:** `src/audio/router.rs`

### A5. Signal registration error logging

**The problem:** `ctrlc_handler()` (`daemon.rs:147-148`) uses `let _ =` on `signal::signal()` calls. If signal setup fails (containers, restricted environments), the daemon has no graceful shutdown path and the user has no idea.

**The fix:** Log `warn!` on failure instead of silently ignoring. Don't bail -- the daemon is still useful without signal handling.

**Files:** `src/daemon.rs`

### A6. pw-metadata and pactl error visibility

**The problem:** `enforce_rate()` (`router.rs:56-64`) and `enforce_alsa_volume()` (`router.rs:66-74`) discard stdout, stderr, and exit status with `let _ =`. If `pw-metadata` or `pactl` isn't installed or fails, sample rate and volume aren't enforced, and the user has no diagnostics.

**The fix:** Capture stderr (piped, not null). Check exit status. On failure, log at `warn!` with the stderr content. Still non-fatal.

**Files:** `src/audio/router.rs`

### A7. Systemd service hardening

**The problem:** `motu-mk5d.service` has `Restart=on-failure` + `RestartSec=3` but no restart limits. Crash-looping daemon restarts indefinitely at 3-second intervals.

**The fix:** Add `StartLimitBurst=5` and `StartLimitIntervalSec=60`. After 5 crashes in 60 seconds, systemd gives up and the user can inspect via `systemctl --user status motu-mk5d`.

**Files:** `install/systemd/motu-mk5d.service`

### A8. Log level consistency

Mostly resolved by A1 (connection errors escalate from `debug!` to `error!`). Remaining policy:

| Level | Use |
|-------|-----|
| `error!` | Core functionality broken (connection lost, state save failed, router start failed) |
| `warn!` | Degraded but non-fatal (child died, pactl failed, corrupted state) |
| `info!` | Operational milestones (connected, synced N properties, router started) |
| `debug!` | Per-message detail (individual property updates, ping/pong) |

---

## Pillar B: Installation Error Handling (0.4.0)

### B1. postinst script: phased error handling

**The problem:** `debian/postinst` has 11 `|| true` clauses. Every failure is silenced. The user always sees "motu-mk5 installed" regardless of whether anything actually worked.

**The fix:** Split into phases:

1. **Critical** (fail install if broken):
   - Binary existence check: `command -v motu-mk5d`
   - System file installs (wireplumber conf, pipewire conf) -- remove `|| true`

2. **Best-effort** (log warnings, continue):
   - Legacy cleanup (`rm -f` old files) -- keep `|| true`
   - Profile cache clearing -- keep `|| true`
   - Per-user config installation -- log per-user failures
   - udev reload -- keep `|| true`

3. **Audio restart** -- add per-user error reporting:
   - On failure: `echo "WARNING: failed to restart audio for $su_user" >&2`
   - Track success count, report summary

### B2. Makefile: preflight checks

**The fix:** Add a `preflight` target that checks for required tools before install:

- `pw-loopback` (required)
- `pw-cli` (required)
- `pactl` (optional, warn if missing)
- `systemctl` (required)

Make `install` depend on `preflight`. Add post-restart verification (check PipeWire came back).

### B3. postinst post-install validation

After all phases complete, verify:
- `motu-mk5d` binary is executable and on PATH
- Systemd unit file is loadable (`systemctl --user cat motu-mk5d.service`)
- ALSA profile file exists at expected path
- udev rule exists

Report any failures as warnings (don't fail the install, since the files are already placed).

---

## Pillar C: Diagnostics and Issue Registration (0.5.0)

### C1. `motu-ctl diagnose` command

New subcommand that gathers system context and outputs markdown ready to paste into a GitHub issue.

**Sections gathered:**
1. System: kernel (`uname -r`), distro (`/etc/os-release`), arch
2. Package version: from `env!("CARGO_PKG_VERSION")`
3. USB device: `lsusb | grep 07fd` (MOTU vendor ID)
4. Network interface: CDC Ethernet link-local addresses (`ip addr` filtered for 169.254.x.x)
5. PipeWire: `pw-cli info 0` (version, status), MOTU node listing
6. WirePlumber: `wpctl status` filtered for MOTU entries
7. Device connection: attempt `DeviceConnection::open()`, report success or the specific error
8. Daemon status: `systemctl --user status motu-mk5d`
9. Recent logs: `journalctl --user-unit motu-mk5d -n 30 --no-pager`
10. Audio router: `pgrep -a pw-loopback`

**Output format:** Markdown with headers per section. Each section is self-contained (never fails -- captures errors as text in the report).

**Refactoring:** Extract `discover_alsa_nodes()` from `daemon.rs` into `src/audio/discovery.rs` (or `src/audio/mod.rs`) so both the daemon and diagnostic command can call it.

**Files:** new `src/diagnostics.rs`, update `src/lib.rs`, update `src/ctl.rs`, refactor `src/daemon.rs`

### C2. Actionable error messages

Every user-facing error gets three parts: what broke, likely cause, what to do.

| Current message | Improved |
|---|---|
| `discovery timed out — is the MOTU connected?` | `MOTU device not found on network. Check: (1) USB connected, (2) CDC Ethernet interface exists (run: ip link). Run 'motu-ctl diagnose' for full report.` |
| `MOTU ALSA nodes not found in PipeWire` | `MOTU ALSA nodes not visible in PipeWire. Device may still be initializing, or pro-audio profile not set. Check 'wpctl status'. Run 'motu-ctl diagnose' for details.` |
| `failed to start audio router: {e}` | `Audio router failed: {e}. Check that pw-loopback is installed (part of pipewire package).` |
| `device not available: {e}` | `Device not available: {e}. Retrying in 5s. If this persists, run 'motu-ctl diagnose'.` |

### C3. GitHub issue templates

Create `.github/ISSUE_TEMPLATE/` with structured forms:

1. **bug_report.yml** -- general bug with diagnostic output field
2. **no_audio.yml** -- symptom checkboxes (no sound, crackling, wrong device, etc.)
3. **connection_issue.yml** -- device not found, connection drops, crash-loops
4. **feature_request.yml** -- simple description + use case

Plus `config.yml` to disable blank issues and point users to `motu-ctl diagnose`.

### C4. README troubleshooting section

Add to README:
- How to check daemon status
- How to view logs
- How to generate a diagnostic report
- How to file an issue with context

---

## Implementation Order

```
Phase 1 (independent, parallelizable):
  A3  corrupted state handling       state.rs
  A5  signal registration logging    daemon.rs
  A6  pw-metadata/pactl visibility   router.rs
  A7  systemd hardening              service file

Phase 2 (sequential, core error propagation):
  A1  websocket disconnect           connection.rs -> state.rs -> daemon.rs
  A4  atomic router start            router.rs
  A2  ALSA discovery retry           daemon.rs (touches same main loop as A1)
  A8  log level audit                cross-cutting

Phase 3 (Pillar B):
  B1  postinst phases
  B2  Makefile preflight
  B3  post-install validation

Phase 4 (Pillar C):
  C1  motu-ctl diagnose
  C2  actionable error messages
  C3  GitHub issue templates
  C4  README troubleshooting
```

Each phase gates on the previous: Phase 2 depends on Phase 1 changes being stable. Pillar B and C are independent of each other but both depend on Pillar A being done (C2 references error paths created in A1/A2).
