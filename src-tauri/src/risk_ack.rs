use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

pub const RECOIL_RISK_ACK_VERSION: u32 = 1;
const RISK_ACK_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecoilRiskAcceptance {
    pub version: u32,
    pub accepted_at_unix: u64,
    pub app_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RiskAcknowledgementDocument {
    schema_version: u32,
    #[serde(default)]
    recoil: Option<RecoilRiskAcceptance>,
}

impl Default for RiskAcknowledgementDocument {
    fn default() -> Self {
        Self {
            schema_version: RISK_ACK_SCHEMA_VERSION,
            recoil: None,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RecoilRiskStatus {
    pub required: bool,
    pub current_version: u32,
    pub acceptance: Option<RecoilRiskAcceptance>,
}

#[derive(Clone)]
pub struct RiskAcknowledgementState {
    document: Arc<Mutex<RiskAcknowledgementDocument>>,
    path: PathBuf,
}

impl RiskAcknowledgementState {
    pub fn load(app: &AppHandle) -> Result<Self, String> {
        let dir = app
            .path()
            .app_config_dir()
            .map_err(|error| format!("failed to resolve config directory: {error}"))?;
        let path = dir.join("risk-acknowledgements.json");
        let mut document = if path.exists() {
            let bytes = fs::read(&path)
                .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
            serde_json::from_slice::<RiskAcknowledgementDocument>(&bytes)
                .map_err(|error| format!("failed to parse {}: {error}", path.display()))?
        } else {
            RiskAcknowledgementDocument::default()
        };
        if document.schema_version > RISK_ACK_SCHEMA_VERSION {
            return Err(format!(
                "risk acknowledgement schema {} is newer than supported schema {}",
                document.schema_version, RISK_ACK_SCHEMA_VERSION
            ));
        }
        document.schema_version = RISK_ACK_SCHEMA_VERSION;
        let state = Self {
            document: Arc::new(Mutex::new(document)),
            path,
        };
        state.persist()?;
        Ok(state)
    }

    fn persist(&self) -> Result<(), String> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("failed to create {}: {error}", parent.display()))?;
        }
        let document = self
            .document
            .lock()
            .map_err(|_| "risk acknowledgement mutex poisoned".to_string())?
            .clone();
        let bytes = serde_json::to_vec_pretty(&document)
            .map_err(|error| format!("failed to serialize risk acknowledgement: {error}"))?;
        fs::write(&self.path, bytes)
            .map_err(|error| format!("failed to write {}: {error}", self.path.display()))
    }

    pub fn recoil_is_accepted(&self) -> Result<bool, String> {
        let document = self
            .document
            .lock()
            .map_err(|_| "risk acknowledgement mutex poisoned".to_string())?;
        Ok(document
            .recoil
            .as_ref()
            .is_some_and(|acceptance| acceptance.version == RECOIL_RISK_ACK_VERSION))
    }
}

#[tauri::command]
pub fn recoil_risk_status(
    state: State<'_, RiskAcknowledgementState>,
) -> Result<RecoilRiskStatus, String> {
    let acceptance = state
        .document
        .lock()
        .map_err(|_| "risk acknowledgement mutex poisoned".to_string())?
        .recoil
        .clone();
    Ok(RecoilRiskStatus {
        required: acceptance
            .as_ref()
            .is_none_or(|entry| entry.version != RECOIL_RISK_ACK_VERSION),
        current_version: RECOIL_RISK_ACK_VERSION,
        acceptance,
    })
}

#[tauri::command]
pub fn accept_recoil_risk(
    confirmed_account_risk: bool,
    confirmed_third_party_rules: bool,
    state: State<'_, RiskAcknowledgementState>,
) -> Result<RecoilRiskStatus, String> {
    if !confirmed_account_risk || !confirmed_third_party_rules {
        return Err("both recoil risk acknowledgements must be accepted".into());
    }
    let acceptance = RecoilRiskAcceptance {
        version: RECOIL_RISK_ACK_VERSION,
        accepted_at_unix: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
    };
    {
        let mut document = state
            .document
            .lock()
            .map_err(|_| "risk acknowledgement mutex poisoned".to_string())?;
        document.recoil = Some(acceptance.clone());
    }
    state.persist()?;
    Ok(RecoilRiskStatus {
        required: false,
        current_version: RECOIL_RISK_ACK_VERSION,
        acceptance: Some(acceptance),
    })
}

#[cfg(test)]
mod tests {
    use super::RECOIL_RISK_ACK_VERSION;

    #[test]
    fn recoil_acknowledgement_is_versioned() {
        assert!(RECOIL_RISK_ACK_VERSION >= 1);
    }
}
