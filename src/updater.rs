use std::process::Command;

use anyhow::{bail, Context, Result};

const REPO: &str = "MarkWind85/motu-mk5-linux";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

enum Distro {
    Debian,
    Fedora,
    Arch,
}

impl Distro {
    fn detect() -> Result<Self> {
        let os_release = std::fs::read_to_string("/etc/os-release")
            .context("cannot read /etc/os-release")?;

        let id_line = os_release
            .lines()
            .find(|l| l.starts_with("ID=") || l.starts_with("ID_LIKE="))
            .unwrap_or("");

        let id = id_line
            .split_once('=')
            .map(|(_, v)| v.trim_matches('"').to_lowercase())
            .unwrap_or_default();

        if id.contains("arch") || id.contains("manjaro") || id.contains("endeavouros") {
            Ok(Distro::Arch)
        } else if id.contains("fedora") || id.contains("rhel") || id.contains("centos") || id.contains("opensuse") {
            Ok(Distro::Fedora)
        } else {
            // Debian, Ubuntu, Pop!_OS, Mint, etc — default
            Ok(Distro::Debian)
        }
    }

    fn package_suffix(&self) -> &str {
        match self {
            Distro::Debian => ".deb",
            Distro::Fedora => ".rpm",
            Distro::Arch => ".pkg.tar.zst",
        }
    }

    fn install_cmd(&self, path: &str) -> Vec<String> {
        match self {
            Distro::Debian => vec!["sudo".into(), "dpkg".into(), "-i".into(), path.into()],
            Distro::Fedora => vec!["sudo".into(), "dnf".into(), "install".into(), "-y".into(), path.into()],
            Distro::Arch => vec!["sudo".into(), "pacman".into(), "-U".into(), "--noconfirm".into(), path.into()],
        }
    }

    fn name(&self) -> &str {
        match self {
            Distro::Debian => "Debian/Ubuntu",
            Distro::Fedora => "Fedora/RHEL",
            Distro::Arch => "Arch Linux",
        }
    }
}

struct Release {
    version: String,
    pkg_url: Option<String>,
    pkg_name: Option<String>,
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

fn detect_arch() -> String {
    Command::new("uname").arg("-m").output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|_| "x86_64".into())
}

fn fetch_latest_release(distro: &Distro) -> Result<Release> {
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

    let arch = detect_arch();
    let suffix = distro.package_suffix();

    let mut pkg_url = None;
    let mut pkg_name = None;

    if let Some(assets) = json["assets"].as_array() {
        for asset in assets {
            let name = asset["name"].as_str().unwrap_or("");
            if name.ends_with(suffix) && name.contains(&arch) {
                pkg_url = asset["browser_download_url"].as_str().map(String::from);
                pkg_name = Some(name.to_string());
                break;
            }
        }
    }

    Ok(Release { version, pkg_url, pkg_name })
}

fn download(url: &str, dest: &str) -> Result<()> {
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

fn install_package(distro: &Distro, path: &str) -> Result<()> {
    println!("Installing (sudo required)...");
    let args = distro.install_cmd(path);
    let status = Command::new(&args[0])
        .args(&args[1..])
        .status()
        .with_context(|| format!("failed to run {}", args[0]))?;

    if !status.success() {
        bail!("package install failed — check output above");
    }
    Ok(())
}

pub fn check() -> Result<()> {
    let distro = Distro::detect()?;
    println!("Current version: {CURRENT_VERSION}");
    println!("Detected distro: {}", distro.name());
    print!("Checking for updates... ");
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let release = fetch_latest_release(&distro)?;
    println!("latest release: {}", release.version);

    if is_newer(&release.version, CURRENT_VERSION) {
        println!("\nUpdate available: {CURRENT_VERSION} → {}", release.version);
        if release.pkg_url.is_some() {
            println!("Run 'motu-ctl update' to install.");
        } else {
            println!("No {} package found for your architecture.", distro.package_suffix());
            println!("Download manually: https://github.com/{REPO}/releases/latest");
        }
    } else {
        println!("You're up to date.");
    }

    Ok(())
}

pub fn update() -> Result<()> {
    let distro = Distro::detect()?;
    println!("Current version: {CURRENT_VERSION}");
    println!("Detected distro: {}", distro.name());
    print!("Checking for updates... ");
    std::io::Write::flush(&mut std::io::stdout()).ok();

    let release = fetch_latest_release(&distro)?;
    println!("latest release: {}", release.version);

    if !is_newer(&release.version, CURRENT_VERSION) {
        println!("You're already on the latest version.");
        return Ok(());
    }

    let pkg_url = match &release.pkg_url {
        Some(url) => url,
        None => {
            println!("\nNo {} package found for your architecture.", distro.package_suffix());
            println!("Download manually: https://github.com/{REPO}/releases/latest");
            return Ok(());
        }
    };

    let pkg_name = release.pkg_name.as_deref().unwrap_or("motu-mk5-update");
    let dest = format!("/tmp/{pkg_name}");

    println!("\nUpdate available: {CURRENT_VERSION} → {}", release.version);
    println!("Package: {pkg_name}");
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
    download(pkg_url, &dest)?;
    install_package(&distro, &dest)?;

    std::fs::remove_file(&dest).ok();

    println!("\nUpdated to {}.", release.version);
    Ok(())
}
