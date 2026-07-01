use std::collections::HashMap;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

const GITHUB_REPO: &str = "farrelaby/dirsweep";
const MAX_RETRIES: u32 = 3;

pub fn detect_target() -> &'static str {
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    {
        "x86_64-unknown-linux-gnu"
    }

    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        "aarch64-unknown-linux-gnu"
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "aarch64-apple-darwin"
    }

    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-msvc"
    }
}

pub fn parse_version(tag: &str) -> Option<String> {
    tag.strip_prefix('v').map(|s| s.to_string())
}

pub fn version_is_newer(current: &str, new: &str) -> bool {
    let current_parts: Vec<u32> = current.split('.').filter_map(|s| s.parse().ok()).collect();
    let new_parts: Vec<u32> = new.split('.').filter_map(|s| s.parse().ok()).collect();

    for (a, b) in current_parts.iter().zip(new_parts.iter()) {
        if a < b {
            return true;
        }
        if a > b {
            return false;
        }
    }
    new_parts.len() > current_parts.len()
}

pub fn parse_checksums(content: &str) -> HashMap<String, String> {
    content
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            if parts.len() == 2 {
                Some((parts[1].to_string(), parts[0].to_string()))
            } else {
                None
            }
        })
        .collect()
}

pub fn find_asset_url(assets: &[serde_json::Value], target: &str) -> Option<String> {
    for asset in assets {
        if let Some(name) = asset["name"].as_str()
            && name.contains(target)
            && !name.ends_with(".txt")
        {
            return asset["browser_download_url"]
                .as_str()
                .map(|s| s.to_string());
        }
    }
    None
}

pub fn find_checksums_url(assets: &[serde_json::Value]) -> Option<String> {
    for asset in assets {
        if let Some(name) = asset["name"].as_str()
            && name == "checksums.txt"
        {
            return asset["browser_download_url"]
                .as_str()
                .map(|s| s.to_string());
        }
    }
    None
}

fn download_file(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let response = ureq::get(url).set("User-Agent", "dirsweep").call()?;

    let mut file = std::fs::File::create(dest)?;
    let mut reader = response.into_reader();
    io::copy(&mut reader, &mut file)?;
    Ok(())
}

fn verify_checksum(file_path: &Path, expected_hash: &str) -> bool {
    let output = if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "(Get-FileHash -Path '{}' -Algorithm SHA256).Hash.ToLower()",
                    file_path.display()
                ),
            ])
            .output()
    } else {
        Command::new("sha256sum").arg(file_path).output()
    };

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let actual_hash = if cfg!(target_os = "windows") {
                stdout.trim().to_string()
            } else {
                stdout.split_whitespace().next().unwrap_or("").to_string()
            };
            actual_hash == expected_hash
        }
        Err(_) => false,
    }
}

fn extract_archive(
    archive_path: &Path,
    dest_dir: &Path,
) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    if cfg!(target_os = "windows") {
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                    archive_path.display(),
                    dest_dir.display()
                ),
            ])
            .output()?;
    } else {
        Command::new("tar")
            .args([
                "-xzf",
                &archive_path.to_string_lossy(),
                "-C",
                &dest_dir.to_string_lossy(),
            ])
            .output()?;
    }

    let binary_name = if cfg!(target_os = "windows") {
        "dirsweep.exe"
    } else {
        "dirsweep"
    };
    let binary_path = dest_dir.join(binary_name);

    if binary_path.exists() {
        Ok(binary_path)
    } else {
        Err("Extracted binary not found".into())
    }
}

fn prompt_confirm(message: &str, force: bool) -> bool {
    if force {
        return true;
    }

    print!("{} [y/N] ", message);
    io::stdout().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).ok();

    input.trim().to_lowercase() == "y"
}

pub fn cmd_update() -> Result<(), Box<dyn std::error::Error>> {
    let current_version = env!("CARGO_PKG_VERSION");
    let target = detect_target();

    println!("Checking for updates...");

    let url = format!(
        "https://api.github.com/repos/{}/releases/latest",
        GITHUB_REPO
    );
    let response: serde_json::Value = ureq::get(&url)
        .set("User-Agent", "dirsweep")
        .call()?
        .into_json()?;

    let tag = response["tag_name"]
        .as_str()
        .ok_or("Failed to parse release tag")?;

    let latest_version = parse_version(tag).ok_or("Failed to parse version from tag")?;

    if !version_is_newer(current_version, &latest_version) {
        println!("dirsweep is already up to date (v{})", current_version);
        return Ok(());
    }

    println!("Updating to v{}...", latest_version);

    let assets = response["assets"]
        .as_array()
        .ok_or("No assets found in release")?;

    let asset_url = find_asset_url(assets, target).ok_or("No download found for your platform")?;

    let checksums_url = find_checksums_url(assets).ok_or("No checksums found in release")?;

    let temp_dir = tempfile::tempdir()?;

    let mut success = false;
    for attempt in 1..=MAX_RETRIES {
        let checksums_path = temp_dir.path().join("checksums.txt");
        if download_file(&checksums_url, &checksums_path).is_err() {
            if attempt == MAX_RETRIES {
                println!("Update failed. Please try again later.");
                return Err("Download failed".into());
            }
            println!("Download failed. Retrying... ({}/{})", attempt, MAX_RETRIES);
            continue;
        }

        let archive_name = asset_url.split('/').next_back().unwrap_or("archive");
        let archive_path = temp_dir.path().join(archive_name);
        if download_file(&asset_url, &archive_path).is_err() {
            if attempt == MAX_RETRIES {
                println!("Update failed. Please try again later.");
                return Err("Download failed".into());
            }
            println!("Download failed. Retrying... ({}/{})", attempt, MAX_RETRIES);
            continue;
        }

        let checksums_content = std::fs::read_to_string(&checksums_path)?;
        let checksums = parse_checksums(&checksums_content);

        if let Some(expected_hash) = checksums.get(archive_name)
            && !verify_checksum(&archive_path, expected_hash)
        {
            if attempt == MAX_RETRIES {
                println!("Update failed. Please try again later.");
                return Err("Checksum mismatch".into());
            }
            println!(
                "Download corrupted. Retrying... ({}/{})",
                attempt, MAX_RETRIES
            );
            continue;
        }

        let extract_dir = temp_dir.path().join("extract");
        std::fs::create_dir_all(&extract_dir)?;

        let new_binary = extract_archive(&archive_path, &extract_dir)?;

        self_replace::self_replace(&new_binary)?;

        std::fs::remove_file(&new_binary).ok();

        println!("Updated to v{}", latest_version);
        success = true;
        break;
    }

    if !success {
        println!("Update failed. Please try again later.");
        return Err("Update failed after retries".into());
    }

    Ok(())
}

pub fn cmd_uninstall(force: bool) -> Result<(), Box<dyn std::error::Error>> {
    let exe_path = std::env::current_exe()?;

    if !prompt_confirm("This will remove dirsweep from your system.", force) {
        println!("Uninstall cancelled.");
        return Ok(());
    }

    println!("Uninstalling from {}...", exe_path.display());

    self_replace::self_delete()?;

    println!("dirsweep has been uninstalled.");
    Ok(())
}
