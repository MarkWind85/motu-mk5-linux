use std::process::Command;

use anyhow::{bail, Context, Result};

const REPO: &str = "MarkWind85/motu-mk5-linux";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

struct Release {
    version: String,
    deb_url: Option<String>,
    deb_name: Option<String>,
}

fn parse_version(s: &str) -> (u32, u32, u32) {
    let s = s.strip_prefix('v').unwrap_or(s);
    let parts: Vec<u32> = s.split('.').filter_map(|p| p.parse().ok()).collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest) > parse_version(current)
}

fn detect_arch() -> Result<String> {
    let output = Command::new("uname").arg("-m").output()
        .context("failed to detect architecture")?;
    let arch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match arch.as_str() {
        "x86_64" => Ok("amd64".into()),
        "aarch64" => Ok("arm64".into()),
        other => Ok(other.into()),
    }
}

fn fetch_latest_release() -> Result<Release> {
    let url = format!("https://api.github.com/repos/{REPO}/releases/latest");
    let output = Command::new("curl")
        .args([
            "-sL",
            "-H", "Accept: application/vnd.github+json",
            "-H", &format!("User-Agent: motu-mk5/{CURRENT_VERSION}"),
            &url,
        ])
        .output()
        .context("failed to run curl — is it installed?")?;

    if !output.status.success() {
        bail!("failed to fetch release info from GitHub");
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&body)
        .context("failed to parse GitHub API response")?;

    if let Some(msg) = json["message"].as_str() {
        bail!("GitHub API error: {msg}");
    }

    let tag = json["tag_name"]
        .as_str()
        .context("no tag_name in release")?;
    let version = tag.strip_prefix('v').unwrap_or(tag).to_string();

    let arch = detect_arch().unwrap_or_else(|_| "amd64".into());
    let deb_pattern = format!("_{arch}.deb");

    let mut deb_url = None;
    let mut deb_name = None;

    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("");
            if name.ends_with(&deb_pattern) {
                deb_url = asset["browser_download_url"].as_str().map(String::from);
                deb_name = Some(name.to_string());
                break;
            }
        }
    }

    Ok(Release { version, deb_url, deb_name })
}

fn download_deb(url: &str, dest: &str) -> Result<()> {
    println!("Downloading {}...", dest.rsplit('/').next().unwrap_or(dest));
    let status = Command::new("curl")
        .args(["-sL", "-o", dest, url])
        .status()
        .context("failed to run curl")?;

    if !status.success() {
        bail!("download failed");
    }
    Ok(())
}

fn install_deb(path: &str) -> Result<()> {
    println!("Installing (sudo required)...");
    let status = Command::new("sudo")
        .args(["dpkg", "-i", path])
        .status()
        .context("failed to run dpkg")?;

    if !status.success() {
        bail!("dpkg install failed — check output above");
    }
    Ok(())
}

pub fn check() -> Result<()> {
    println!("Current version: {CURRENT_VERSION}");
    print!("Checking for updates... ");
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let release = fetch_latest_release()?;
    println!("latest release: {}", release.version);

    if is_newer(&release.version, CURRENT_VERSION) {
        println!("\nUpdate available: {CURRENT_VERSION} → {}", release.version);
        if release.deb_url.is_some() {
            println!("Run 'motu-ctl update' to install.");
        } else {
            println!("No .deb package found for your architecture.");
            println!("Download manually: https://github.com/{REPO}/releases/latest");
        }
    } else {
        println!("You're up to date.");
    }

    Ok(())
}

pub fn update() -> Result<()> {
    println!("Current version: {CURRENT_VERSION}");
    print!("Checking for updates... ");
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let release = fetch_latest_release()?;
    println!("latest release: {}", release.version);

    if !is_newer(&release.version, CURRENT_VERSION) {
        println!("You're already on the latest version.");
        return Ok(());
    }

    let deb_url = match &release.deb_url {
        Some(url) => url,
        None => {
            println!("\nNo .deb package found for your architecture.");
            println!("Download manually: https://github.com/{REPO}/releases/latest");
            return Ok(());
        }
    };

    let deb_name = release.deb_name.as_deref().unwrap_or("motu-mk5.deb");
    let dest = format!("/tmp/{deb_name}");

    println!("\nUpdate available: {CURRENT_VERSION} → {}", release.version);
    println!("This will briefly interrupt audio.\n");

    print!("Install? [y/N] ");
    use std::io::Write;
    std::io::stdout().flush().ok();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if !input.trim().eq_ignore_ascii_case("y") {
        println!("Cancelled.");
        return Ok(());
    }

    println!();
    download_deb(deb_url, &dest)?;
    install_deb(&dest)?;

    std::fs::remove_file(&dest).ok();

    println!("\nUpdated to {}.", release.version);
    Ok(())
}
