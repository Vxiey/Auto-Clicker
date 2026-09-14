use std::fs;
use std::io::Read;
use std::path::PathBuf;

use semver::Version;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager};

const REPO: &str = "Vxiey/VxClick";
const RELEASES_API: &str = "https://api.github.com/repos/Vxiey/VxClick/releases/latest";
const RELEASE_DOWNLOAD_PREFIX: &str = "https://github.com/Vxiey/VxClick/releases/download/";
const MAX_PATCH_BYTES: usize = 128 * 1024 * 1024;

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

#[derive(Clone, Debug, Serialize)]
pub struct FullReleaseAsset {
    pub name: String,
    pub url: String,
    pub size: u64,
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

    let full_release = choose_windows_asset(&release.assets);
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
            "patch is for version {from}, but this installation is {current}"
        ));
    }
    if to <= from {
        return Err("patch target must be newer than the source version".into());
    }
    validate_patch_url(&patch.url)?;
    validate_patch_format(&patch.format)?;
    validate_sha256(&patch.sha256)?;
    if patch.size as usize > MAX_PATCH_BYTES {
        return Err("patch is larger than the 128 MiB safety limit".into());
    }

    let response = github_request(&patch.url)?;
    let reader = response.into_reader();
    let mut bytes = Vec::with_capacity((patch.size as usize).min(MAX_PATCH_BYTES));
    reader
        .take((MAX_PATCH_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("failed to download patch: {error}"))?;
    if bytes.len() > MAX_PATCH_BYTES {
        return Err("download exceeded the 128 MiB patch safety limit".into());
    }
    if patch.size != 0 && bytes.len() as u64 != patch.size {
        return Err(format!(
            "patch size mismatch: manifest says {} bytes, downloaded {} bytes",
            patch.size,
            bytes.len()
        ));
    }

    let actual_hash = format!("{:x}", Sha256::digest(&bytes));
    if !actual_hash.eq_ignore_ascii_case(&patch.sha256) {
        return Err("patch SHA-256 verification failed".into());
    }

    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|error| format!("failed to resolve cache directory: {error}"))?
        .join("updates");
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
    let path = safe_patch_path(&dir, &from, &to, &patch.format);
    fs::write(&path, &bytes)
        .map_err(|error| format!("failed to stage {}: {error}", path.display()))?;

    Ok(StagedPatch {
        path: path.to_string_lossy().into_owned(),
        sha256: actual_hash,
        size: bytes.len() as u64,
        from_version: from.to_string(),
        to_version: to.to_string(),
    })
}

fn choose_windows_asset(assets: &[GitHubAsset]) -> Option<FullReleaseAsset> {
    let priority = |name: &str| {
        let lower = name.to_ascii_lowercase();
        if !lower.contains("vxclick") {
            return 99;
        }
        if lower.ends_with(".msi") {
            0
        } else if lower.ends_with(".exe") {
            1
        } else if lower.ends_with(".zip") {
            2
        } else {
            99
        }
    };

    assets
        .iter()
        .filter(|asset| {
            priority(&asset.name) < 99 && validate_patch_url(&asset.browser_download_url).is_ok()
        })
        .min_by_key(|asset| priority(&asset.name))
        .map(|asset| FullReleaseAsset {
            name: asset.name.clone(),
            url: asset.browser_download_url.clone(),
            size: asset.size,
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

    validate_patch_url(&manifest_asset.browser_download_url)?;
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
            validate_patch_url(&patch.url)?;
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
    github_request(url)?
        .into_json::<T>()
        .map_err(|error| format!("failed to decode GitHub response: {error}"))
}

fn github_request(url: &str) -> Result<ureq::Response, String> {
    ureq::get(url)
        .set("User-Agent", concat!("VxClick/", env!("CARGO_PKG_VERSION")))
        .set("Accept", "application/vnd.github+json")
        .set("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .map_err(|error| format!("GitHub request failed: {error}"))
}

fn validate_patch_url(url: &str) -> Result<(), String> {
    if !url.starts_with(RELEASE_DOWNLOAD_PREFIX) {
        return Err("update URL must point to the official VxClick GitHub release area".into());
    }
    Ok(())
}

fn validate_patch_format(format: &str) -> Result<(), String> {
    match format.to_ascii_lowercase().as_str() {
        "bsdiff" | "zstd" => Ok(()),
        _ => Err("unsupported patch format; expected bsdiff or zstd".into()),
    }
}

fn validate_sha256(value: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("patch manifest contains an invalid SHA-256 value".into());
    }
    Ok(())
}

fn safe_patch_path(dir: &std::path::Path, from: &Version, to: &Version, format: &str) -> PathBuf {
    let extension = match format.to_ascii_lowercase().as_str() {
        "bsdiff" => "bsdiff",
        "zstd" => "zst",
        _ => "patch",
    };
    dir.join(format!("vxclick-{from}-to-{to}.{extension}"))
}

#[cfg(test)]
mod tests {
    use super::{GitHubAsset, choose_windows_asset, validate_patch_format, validate_patch_url};

    #[test]
    fn rejects_unknown_patch_format() {
        assert!(validate_patch_format("zip").is_err());
        assert!(validate_patch_format("bsdiff").is_ok());
        assert!(validate_patch_format("zstd").is_ok());
    }

    #[test]
    fn rejects_non_official_update_urls() {
        assert!(validate_patch_url("https://example.com/VxClick.exe").is_err());
        assert!(
            validate_patch_url(
                "https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick.exe"
            )
            .is_ok()
        );
    }

    #[test]
    fn full_release_ignores_unrelated_executables() {
        let assets = vec![
            GitHubAsset {
                name: "unrelated.exe".into(),
                browser_download_url: "https://github.com/Vxiey/VxClick/releases/download/v1.0.0/unrelated.exe".into(),
                size: 1,
            },
            GitHubAsset {
                name: "VxClick_1.0.0_x64_en-US.msi".into(),
                browser_download_url: "https://github.com/Vxiey/VxClick/releases/download/v1.0.0/VxClick_1.0.0_x64_en-US.msi".into(),
                size: 2,
            },
        ];
        assert_eq!(choose_windows_asset(&assets).expect("asset").size, 2);
    }
}
