use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

const REPO: &str = "Vxiey/VxClick";
const RELEASES_API: &str = "https://api.github.com/repos/Vxiey/VxClick/releases/latest";
const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/Vxiey/VxClick/releases/download/";
const MAX_PATCH_BYTES: usize = 128 * 1024 * 1024;
const MAX_FULL_UPDATE_BYTES: usize = 512 * 1024 * 1024;
const INSTALLER_FORMAT: &str = "installer";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    published_at: Option<String>,
    assets: Vec<GitHubAsset>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct GitHubAsset {
    name: String,
    browser_download_url: String,
    size: u64,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PatchAsset {
    pub from_version: String,
    pub to_version: String,
    pub url: String,
    pub sha256: String,
    pub size: u64,
    pub format: String,
}

#[derive(Debug, Deserialize)]
struct PatchManifest {
    schema_version: u32,
    patches: Vec<PatchAsset>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FullReleaseAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
    pub sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct UpdateInfo {
    pub repository: &'static str,
    pub current_version: String,
    pub latest_version: String,
    pub available: bool,
    pub release_url: String,
    pub published_at: Option<String>,
    pub full_release: Option<FullReleaseAsset>,
    pub patch: Option<PatchAsset>,
}

#[derive(Clone, Debug, Serialize)]
pub struct StagedPatch {
    pub path: String,
    pub sha256: String,
    pub size: u64,
    pub from_version: String,
    pub to_version: String,
}

#[tauri::command]
pub fn check_for_updates() -> Result<UpdateInfo, String> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("invalid local version: {error}"))?;
    let release: GitHubRelease = github_get_json(RELEASES_API)?;
    let latest_text = release.tag_name.trim_start_matches('v').to_string();
    let latest = Version::parse(&latest_text)
        .map_err(|error| format!("invalid release version '{}': {error}", release.tag_name))?;

    let full_release = choose_windows_installer(&release.assets);
    let patch = if latest > current {
        find_patch(&release.assets, &current, &latest).unwrap_or_else(|error| {
            eprintln!("update patch manifest ignored: {error}");
            None
        })
    } else {
        None
    };

    Ok(UpdateInfo {
        repository: REPO,
        current_version: current.to_string(),
        latest_version: latest.to_string(),
        available: latest > current,
        release_url: release.html_url,
        published_at: release.published_at,
        full_release,
        patch,
    })
}

#[tauri::command]
pub fn stage_patch(app: AppHandle, patch: PatchAsset) -> Result<StagedPatch, String> {
    let current = Version::parse(env!("CARGO_PKG_VERSION"))
        .map_err(|error| format!("invalid local version: {error}"))?;
    let from = Version::parse(patch.from_version.trim_start_matches('v'))
        .map_err(|error| format!("invalid patch source version: {error}"))?;
    let to = Version::parse(patch.to_version.trim_start_matches('v'))
        .map_err(|error| format!("invalid patch target version: {error}"))?;

    if from != current {
        return Err(format!(
            "update is for version {from}, but this installation is {current}"
        ));
    }
    if to <= from {
        return Err("update target must be newer than the source version".into());
    }
    validate_release_url(&patch.url)?;
    validate_sha256(&patch.sha256)?;

    if patch.format.eq_ignore_ascii_case(INSTALLER_FORMAT) {
        return install_full_update(app, patch, &from, &to);
    }

    validate_patch_format(&patch.format)?;
    if patch.size as usize > MAX_PATCH_BYTES {
        return Err("patch is larger than the 128 MiB safety limit".into());
    }

    let bytes = download_bytes(&patch.url, MAX_PATCH_BYTES)?;
    verify_download(&bytes, patch.size, &patch.sha256, "patch")?;

    let dir = update_cache_dir(&app)?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
    let path = safe_patch_path(&dir, &from, &to, &patch.format);
    fs::write(&path, &bytes)
        .map_err(|error| format!("failed to stage {}: {error}", path.display()))?;

    Ok(StagedPatch {
        path: path.to_string_lossy().into_owned(),
        sha256: sha256_hex(&bytes),
        size: bytes.len() as u64,
        from_version: from.to_string(),
        to_version: to.to_string(),
    })
}

fn install_full_update(
    app: AppHandle,
    patch: PatchAsset,
    from: &Version,
    to: &Version,
) -> Result<StagedPatch, String> {
    if patch.size as usize > MAX_FULL_UPDATE_BYTES {
        return Err("installer exceeds the 512 MiB safety limit".into());
    }
    let installer_name = patch
        .url
        .rsplit('/')
        .next()
        .ok_or_else(|| "installer URL is missing a file name".to_string())?;
    validate_installer_name(installer_name)?;

    let bytes = download_bytes(&patch.url, MAX_FULL_UPDATE_BYTES)?;
    verify_download(&bytes, patch.size, &patch.sha256, "installer")?;

    let dir = update_cache_dir(&app)?;
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
    let path = dir.join(installer_name);
    fs::write(&path, &bytes)
        .map_err(|error| format!("failed to stage {}: {error}", path.display()))?;

    let canonical_dir = dir
        .canonicalize()
        .map_err(|error| format!("failed to resolve update cache: {error}"))?;
    let canonical_path = path
        .canonicalize()
        .map_err(|error| format!("failed to resolve staged installer: {error}"))?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err("staged installer is outside the VxClick update cache".into());
    }

    let staged_bytes = fs::read(&canonical_path)
        .map_err(|error| format!("failed to re-read staged installer: {error}"))?;
    verify_download(
        &staged_bytes,
        bytes.len() as u64,
        &patch.sha256,
        "staged installer",
    )?;

    let lower = installer_name.to_ascii_lowercase();
    let child = if lower.ends_with(".msi") {
        Command::new("msiexec.exe")
            .arg("/i")
            .arg(&canonical_path)
            .spawn()
            .map_err(|error| format!("failed to start Windows Installer: {error}"))?
    } else {
        Command::new(&canonical_path)
            .spawn()
            .map_err(|error| format!("failed to start VxClick installer: {error}"))?
    };
    drop(child);

    let staged = StagedPatch {
        path: canonical_path.to_string_lossy().into_owned(),
        sha256: sha256_hex(&staged_bytes),
        size: staged_bytes.len() as u64,
        from_version: from.to_string(),
        to_version: to.to_string(),
    };
    app.exit(0);
    Ok(staged)
}

fn choose_windows_installer(assets: &[GitHubAsset]) -> Option<FullReleaseAsset> {
    let priority = |name: &str| {
        let lower = name.to_ascii_lowercase();
        if !lower.contains("vxclick") {
            return 99;
        }
        if lower.ends_with("setup.exe") {
            0
        } else if lower.ends_with(".msi") {
            1
        } else {
            99
        }
    };

    assets
        .iter()
        .filter(|asset| {
            priority(&asset.name) < 99 && validate_release_url(&asset.browser_download_url).is_ok()
        })
        .min_by_key(|asset| priority(&asset.name))
        .map(|asset| FullReleaseAsset {
            name: asset.name.clone(),
            url: asset.browser_download_url.clone(),
            size: asset.size,
            sha256: asset.digest.as_deref().and_then(parse_github_sha256),
        })
}

fn find_patch(
    assets: &[GitHubAsset],
    current: &Version,
    latest: &Version,
) -> Result<Option<PatchAsset>, String> {
    let Some(manifest_asset) = assets.iter().find(|asset| {
        matches!(
            asset.name.to_ascii_lowercase().as_str(),
            "vxclick-patches.json" | "patches.json"
        )
    }) else {
        return Ok(None);
    };

    validate_release_url(&manifest_asset.browser_download_url)?;
    let manifest: PatchManifest = github_get_json(&manifest_asset.browser_download_url)?;
    if manifest.schema_version != 1 {
        return Err(format!(
            "unsupported patch manifest schema {}",
            manifest.schema_version
        ));
    }

    for patch in manifest.patches {
        let from = match Version::parse(patch.from_version.trim_start_matches('v')) {
            Ok(version) => version,
            Err(_) => continue,
        };
        let to = match Version::parse(patch.to_version.trim_start_matches('v')) {
            Ok(version) => version,
            Err(_) => continue,
        };
        if &from == current && &to == latest {
            validate_release_url(&patch.url)?;
            validate_patch_format(&patch.format)?;
            validate_sha256(&patch.sha256)?;
            if patch.size as usize > MAX_PATCH_BYTES {
                return Err("patch manifest exceeds the 128 MiB safety limit".into());
            }
            return Ok(Some(patch));
        }
    }
    Ok(None)
}

fn github_get_json<T: for<'de> Deserialize<'de>>(url: &str) -> Result<T, String> {
    let mut response = github_request(url)?;
    response
        .body_mut()
        .read_json::<T>()
        .map_err(|error| format!("failed to decode GitHub response: {error}"))
}

fn download_bytes(url: &str, limit: usize) -> Result<Vec<u8>, String> {
    let mut response = github_request(url)?;
    let bytes = response
        .body_mut()
        .with_config()
        .limit((limit + 1) as u64)
        .read_to_vec()
        .map_err(|error| format!("failed to download update: {error}"))?;
    if bytes.len() > limit {
        return Err("download exceeded the configured update safety limit".into());
    }
    Ok(bytes)
}

fn github_request(url: &str) -> Result<ureq::http::Response<ureq::Body>, String> {
    ureq::get(url)
        .header("User-Agent", concat!("VxClick/", env!("CARGO_PKG_VERSION")))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|error| format!("GitHub request failed: {error}"))
}

fn validate_release_url(url: &str) -> Result<(), String> {
    if !url.starts_with(RELEASE_DOWNLOAD_PREFIX) {
        return Err("update URL must point to the official VxClick GitHub release area".into());
    }
    Ok(())
}

fn validate_installer_name(name: &str) -> Result<(), String> {
    if name.contains('/') || name.contains('\\') {
        return Err("installer name contains a path separator".into());
    }
    let lower = name.to_ascii_lowercase();
    if !lower.starts_with("vxclick") || !(lower.ends_with("setup.exe") || lower.ends_with(".msi")) {
        return Err("automatic updates require an official VxClick setup EXE or MSI".into());
    }
    Ok(())
}

fn parse_github_sha256(value: &str) -> Option<String> {
    let hash = value.strip_prefix("sha256:")?;
    validate_sha256(hash).ok()?;
    Some(hash.to_ascii_lowercase())
}

fn validate_patch_format(format: &str) -> Result<(), String> {
    match format.to_ascii_lowercase().as_str() {
        "bsdiff" | "zstd" => Ok(()),
        _ => Err("unsupported patch format; expected bsdiff or zstd".into()),
    }
}

fn validate_sha256(value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("update metadata contains an invalid SHA-256 value".into());
    }
    Ok(())
}

fn verify_download(
    bytes: &[u8],
    expected_size: u64,
    expected_hash: &str,
    label: &str,
) -> Result<(), String> {
    if expected_size != 0 && bytes.len() as u64 != expected_size {
        return Err(format!(
            "{label} size mismatch: expected {expected_size} bytes, downloaded {} bytes",
            bytes.len()
        ));
    }
    let actual_hash = sha256_hex(bytes);
    if !actual_hash.eq_ignore_ascii_case(expected_hash) {
        return Err(format!("{label} SHA-256 verification failed"));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn update_cache_dir(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("failed to resolve cache directory: {error}"))?
        .join("updates"))
}

fn safe_patch_path(dir: &Path, from: &Version, to: &Version, format: &str) -> PathBuf {
    let extension = match format.to_ascii_lowercase().as_str() {
        "bsdiff" => "bsdiff",
        "zstd" => "zst",
        _ => "patch",
    };
    dir.join(format!("vxclick-{from}-to-{to}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::{
        GitHubAsset, choose_windows_installer, parse_github_sha256, validate_installer_name,
        validate_patch_format, validate_release_url,
    };

    #[test]
    fn rejects_unknown_patch_format() {
        assert!(validate_patch_format("zip").is_err());
        assert!(validate_patch_format("bsdiff").is_ok());
        assert!(validate_patch_format("zstd").is_ok());
    }

    #[test]
    fn rejects_non_official_update_urls() {
        assert!(validate_release_url("https://example.com/VxClick.exe").is_err());
        assert!(
            validate_release_url(
                "https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick.exe"
            )
            .is_ok()
        );
    }

    #[test]
    fn installer_selection_prefers_setup_exe_and_digest() {
        let hash = "a".repeat(64);
        let assets = vec![
            GitHubAsset {
                name: "VxClick_1.0.0_x64_en-US.msi".into(),
                browser_download_url: "https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64_en-US.msi".into(),
                size: 2,
                digest: Some(format!("sha256:{hash}")),
            },
            GitHubAsset {
                name: "VxClick_1.0.0_x64-setup.exe".into(),
                browser_download_url: "https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64-setup.exe".into(),
                size: 1,
                digest: Some(format!("sha256:{hash}")),
            },
            GitHubAsset {
                name: "VxClick.exe".into(),
                browser_download_url: "https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick.exe".into(),
                size: 3,
                digest: Some(format!("sha256:{hash}")),
            },
        ];
        let selected = choose_windows_installer(&assets).expect("installer");
        assert_eq!(selected.name, "VxClick_1.0.0_x64-setup.exe");
        assert_eq!(selected.sha256.as_deref(), Some(hash.as_str()));
    }

    #[test]
    fn auto_update_rejects_portable_executable() {
        assert!(validate_installer_name("VxClick.exe").is_err());
        assert!(validate_installer_name("VxClick_1.0.1_x64-setup.exe").is_ok());
        assert!(validate_installer_name("VxClick_1.0.1_x64_en-US.msi").is_ok());
    }

    #[test]
    fn parses_github_sha256_digest() {
        let hash = "b".repeat(64);
        assert_eq!(parse_github_sha256(&format!("sha256:{hash}")), Some(hash));
        assert!(parse_github_sha256("sha512:abcd").is_none());
    }
}
