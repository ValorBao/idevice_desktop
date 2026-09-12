use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};

use isideload::{
    anisette::remote_v3::RemoteV3AnisetteProvider,
    auth::apple_account::{AppleAccount, TwoFactorCallbackParams, TwoFactorCallbackResponse},
    dev::{developer_session::DeveloperSession, devices::DevicesApi},
    sideload::{SideloaderBuilder, builder::MaxCertsBehavior, sideloader::Sideloader},
    util::keyring_storage::KeyringStorage,
};
use plist::Value;
use rootcause::{Report, prelude::*};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;
use zeroize::Zeroizing;

use super::signing::{inspect_account_signing_ipa, validate_account_signing_output_path};
use crate::{
    error::{CommandError, CommandResult},
    state::AppState,
    task::{OperationSlot, stage, wait_child},
    types::OperationProgress,
};

const ANISETTE_SERVER: &str = "https://ani.sidestore.io";
const KEYCHAIN_SERVICE: &str = "idevice_desktop.personal_signing";
const TWO_FACTOR_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Clone)]
struct AccountSummary {
    email: String,
    team_id: String,
    team_name: String,
}

#[derive(Clone)]
struct ActiveAccount {
    sideloader: Arc<Mutex<Sideloader>>,
    summary: AccountSummary,
}

#[derive(Default)]
pub struct AccountSigningState {
    account: Mutex<Option<ActiveAccount>>,
    login_gate: Mutex<()>,
    signing: OperationSlot,
    pending_two_factor: Mutex<Option<oneshot::Sender<TwoFactorCallbackResponse>>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalAccountStatus {
    pub logged_in: bool,
    pub email: Option<String>,
    pub team_id: Option<String>,
    pub team_name: Option<String>,
    pub anisette_server: String,
    pub credential_storage: String,
    pub password_stored: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalAccountSigningRequest {
    pub operation_id: String,
    pub ipa_path: String,
    pub output_path: String,
    pub udid: String,
    pub device_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PersonalAccountSigningResult {
    pub output_path: String,
    pub app_name: String,
    pub bundle_id: String,
    pub version: String,
    pub team_id: String,
    pub team_name: String,
    pub size_bytes: u64,
}

fn account_error(message: impl Into<String>, retryable: bool) -> CommandError {
    CommandError::new("personal_account", message, retryable)
}

fn map_report(context: &str, report: Report) -> CommandError {
    let message = report.to_string();
    let lower = message.to_ascii_lowercase();
    let retryable = lower.contains("timed out")
        || lower.contains("timeout")
        || lower.contains("connection")
        || lower.contains("anisette")
        || lower.contains("temporarily");
    account_error(format!("{context}: {message}"), retryable)
}

fn status_for(account: Option<&ActiveAccount>) -> PersonalAccountStatus {
    PersonalAccountStatus {
        logged_in: account.is_some(),
        email: account.map(|value| value.summary.email.clone()),
        team_id: account.map(|value| value.summary.team_id.clone()),
        team_name: account.map(|value| value.summary.team_name.clone()),
        anisette_server: ANISETTE_SERVER.into(),
        credential_storage: "macOS Keychain".into(),
        password_stored: false,
    }
}

#[tauri::command]
pub async fn personal_account_status(
    state: State<'_, AccountSigningState>,
) -> CommandResult<PersonalAccountStatus> {
    let account = state.account.lock().await;
    Ok(status_for(account.as_ref()))
}

async fn request_two_factor(
    app: AppHandle,
    params: TwoFactorCallbackParams,
) -> Result<TwoFactorCallbackResponse, Report> {
    let (sender, receiver) = oneshot::channel();
    let state = app.state::<AccountSigningState>();
    {
        let mut pending = state.pending_two_factor.lock().await;
        if let Some(previous) = pending.replace(sender) {
            let _ = previous.send(TwoFactorCallbackResponse::Abort);
        }
    }
    app.emit("personal-account://two-factor-required", params)
        .context("Unable to show the verification-code prompt")?;

    match tokio::time::timeout(TWO_FACTOR_TIMEOUT, receiver).await {
        Ok(Ok(response)) => Ok(response),
        Ok(Err(_)) => bail!("The verification request was canceled"),
        Err(_) => {
            let mut pending = state.pending_two_factor.lock().await;
            pending.take();
            bail!("The verification code timed out after 3 minutes")
        }
    }
}

#[tauri::command]
pub async fn personal_account_submit_two_factor(
    state: State<'_, AccountSigningState>,
    code: String,
) -> CommandResult<()> {
    let code = code.trim();
    if code.len() != 6 || !code.chars().all(|character| character.is_ascii_digit()) {
        return Err(account_error("Enter the 6-digit verification code", false));
    }
    let sender = state
        .pending_two_factor
        .lock()
        .await
        .take()
        .ok_or_else(|| account_error("No verification request is waiting", false))?;
    sender
        .send(TwoFactorCallbackResponse::SubmitCode(code.into()))
        .map_err(|_| account_error("The verification request has already ended", false))
}

#[tauri::command]
pub async fn personal_account_cancel_two_factor(
    state: State<'_, AccountSigningState>,
) -> CommandResult<()> {
    if let Some(sender) = state.pending_two_factor.lock().await.take() {
        let _ = sender.send(TwoFactorCallbackResponse::Abort);
    }
    Ok(())
}

#[tauri::command]
pub async fn personal_account_login(
    app: AppHandle,
    state: State<'_, AccountSigningState>,
    email: String,
    password: String,
) -> CommandResult<PersonalAccountStatus> {
    let _login_guard = state.login_gate.lock().await;
    let email = email.trim().to_ascii_lowercase();
    if email.is_empty() || !email.contains('@') || email.len() > 254 {
        return Err(account_error(
            "Enter a valid Apple Account email address",
            false,
        ));
    }
    if password.is_empty() {
        return Err(account_error("Enter your Apple Account password", false));
    }
    let password = Zeroizing::new(password);
    let callback_app = app.clone();
    let callback = move |params: TwoFactorCallbackParams| {
        let callback_app = callback_app.clone();
        async move { request_two_factor(callback_app, params).await }
    };
    let provider = RemoteV3AnisetteProvider::new(
        ANISETTE_SERVER,
        Box::new(KeyringStorage::new(KEYCHAIN_SERVICE.into())),
        "0".into(),
    )
    .map_err(|report| map_report("Unable to prepare Apple authentication", report))?;
    let login_result = AppleAccount::builder(&email)
        .anisette_provider(provider)
        .login(password.as_str(), callback)
        .await;
    state.pending_two_factor.lock().await.take();
    let mut apple_account =
        login_result.map_err(|report| map_report("Apple Account login failed", report))?;
    let developer_session = DeveloperSession::from_account(&mut apple_account)
        .await
        .map_err(|report| map_report("Unable to open the Apple developer session", report))?;
    let mut sideloader = SideloaderBuilder::new(developer_session, email.clone())
        .machine_name("idevice_desktop".into())
        .storage(Box::new(KeyringStorage::new(KEYCHAIN_SERVICE.into())))
        .max_certs_behavior(MaxCertsBehavior::Error)
        .delete_app_after_install(false)
        .build();
    let team = sideloader
        .get_team()
        .await
        .map_err(|report| map_report("Unable to load the Apple developer team", report))?;
    let summary = AccountSummary {
        email,
        team_id: team.team_id,
        team_name: team.name.unwrap_or_else(|| "Personal Team".into()),
    };
    let active = ActiveAccount {
        sideloader: Arc::new(Mutex::new(sideloader)),
        summary,
    };
    let response = status_for(Some(&active));
    *state.account.lock().await = Some(active);
    Ok(response)
}

#[tauri::command]
pub async fn personal_account_sign_out(state: State<'_, AccountSigningState>) -> CommandResult<()> {
    state.signing.cancel(None);
    personal_account_cancel_two_factor(state.clone()).await?;
    state.account.lock().await.take();
    Ok(())
}

struct TemporaryInput {
    ipa: PathBuf,
    extracted: PathBuf,
}

impl TemporaryInput {
    async fn copy_from(source: &Path) -> CommandResult<Self> {
        let source = source.to_path_buf();
        // The blocking worker owns the cleanup guard even if its waiter is cancelled.
        tokio::task::spawn_blocking(move || {
            let ipa = std::env::temp_dir().join(format!(
                "idevice_desktop-account-signing-{}.ipa",
                Uuid::new_v4()
            ));
            let extracted = ipa.with_file_name(format!(
                "{}_extracted",
                ipa.file_name().unwrap_or_default().to_string_lossy()
            ));
            let temporary = Self { ipa, extracted };
            std::fs::copy(source, &temporary.ipa).map_err(|error| {
                account_error(format!("Unable to prepare the IPA: {error}"), false)
            })?;
            Ok(temporary)
        })
        .await
        .map_err(|error| account_error(format!("Unable to copy the IPA: {error}"), false))?
    }
}

impl Drop for TemporaryInput {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.ipa);
        let _ = std::fs::remove_dir_all(&self.extracted);
    }
}

struct StagedOutput {
    path: PathBuf,
    committed: bool,
}

impl Drop for StagedOutput {
    fn drop(&mut self) {
        if !self.committed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

fn emit_progress(app: &AppHandle, operation_id: &str, item: &str, percent: u64) {
    let _ = app.emit(
        "personal-account://progress",
        OperationProgress {
            operation: operation_id.into(),
            item: item.into(),
            percent,
        },
    );
}

fn read_signed_bundle_info(app_path: &Path) -> CommandResult<(String, String, String)> {
    let info = Value::from_file(app_path.join("Info.plist"))
        .map_err(|error| account_error(format!("Unable to read the signed app: {error}"), false))?;
    let info = info
        .as_dictionary()
        .ok_or_else(|| account_error("The signed app Info.plist is invalid", false))?;
    let value = |key: &str| info.get(key).and_then(Value::as_string).map(str::to_owned);
    let bundle_id = value("CFBundleIdentifier")
        .ok_or_else(|| account_error("The signed app has no bundle identifier", false))?;
    let name = value("CFBundleDisplayName")
        .or_else(|| value("CFBundleName"))
        .unwrap_or_else(|| bundle_id.clone());
    let version = value("CFBundleShortVersionString")
        .or_else(|| value("CFBundleVersion"))
        .unwrap_or_else(|| "—".into());
    Ok((name, bundle_id, version))
}

#[tauri::command]
pub async fn personal_account_sign_export(
    app: AppHandle,
    state: State<'_, AccountSigningState>,
    request: PersonalAccountSigningRequest,
) -> CommandResult<PersonalAccountSigningResult> {
    let operation = state.signing.begin(&request.operation_id)?;
    let key = format!("account-signing:{}", request.operation_id);
    let tasks = app.state::<AppState>();
    tasks.replace_task(&key, operation.token.clone()).await;
    let result = sign_export(&app, &state, &request, &operation.token).await;
    tasks.cancel_task(&key).await;
    result
}

#[tauri::command]
pub async fn personal_account_sign_cancel(
    state: State<'_, AccountSigningState>,
    operation_id: String,
) -> CommandResult<()> {
    state.signing.cancel(Some(&operation_id));
    Ok(())
}

async fn sign_export(
    app: &AppHandle,
    state: &AccountSigningState,
    request: &PersonalAccountSigningRequest,
    token: &tokio_util::sync::CancellationToken,
) -> CommandResult<PersonalAccountSigningResult> {
    let active = state
        .account
        .lock()
        .await
        .clone()
        .ok_or_else(|| account_error("Sign in with your Apple Account first", false))?;
    let input = PathBuf::from(&request.ipa_path);
    let output = PathBuf::from(&request.output_path);
    let inspected = stage(token, "Inspecting IPA", Duration::from_secs(120), async {
        tokio::task::spawn_blocking({
            let input = input.clone();
            move || inspect_account_signing_ipa(&input)
        })
        .await
        .map_err(|error| account_error(format!("IPA inspection failed: {error}"), false))?
    })
    .await?;
    let output = validate_account_signing_output_path(&input, &output)?;
    if request.udid.trim().is_empty() {
        return Err(account_error(
            "Connect and select a device before signing",
            false,
        ));
    }
    let device_name = request.device_name.trim();
    let device_name = if device_name.is_empty() {
        "idevice_desktop device"
    } else {
        device_name
    };

    emit_progress(
        app,
        &request.operation_id,
        "Preparing a protected temporary copy",
        3,
    );
    let temporary = stage(
        token,
        "Preparing IPA",
        Duration::from_secs(120),
        TemporaryInput::copy_from(&input),
    )
    .await?;
    let mut sideloader = stage(
        token,
        "Opening signing session",
        Duration::from_secs(30),
        async { Ok(active.sideloader.lock().await) },
    )
    .await?;
    let team = stage(
        token,
        "Loading developer team",
        Duration::from_secs(60),
        async {
            sideloader
                .get_team()
                .await
                .map_err(|report| map_report("Unable to load the developer team", report))
        },
    )
    .await?;
    emit_progress(
        app,
        &request.operation_id,
        "Registering the selected device",
        8,
    );
    stage(
        token,
        "Registering device",
        Duration::from_secs(60),
        async {
            sideloader
                .get_dev_session()
                .ensure_device_registered(&team, device_name, request.udid.trim(), None)
                .await
                .map_err(|report| map_report("Unable to register the device", report))
        },
    )
    .await?;

    let progress_app = app.clone();
    let progress_id = request.operation_id.clone();
    let progress = move |value: f32| {
        let progress_app = progress_app.clone();
        let progress_id = progress_id.clone();
        async move {
            let percent = 10 + (f64::from(value.clamp(0.0, 1.0)) * 72.0).round() as u64;
            emit_progress(
                &progress_app,
                &progress_id,
                "Requesting profile and signing the app",
                percent,
            );
        }
    };
    let (signed_app, _) = stage(token, "Signing IPA", Duration::from_secs(600), async {
        sideloader
            .sign_app(temporary.ipa.clone(), Some(team), false, Some(progress))
            .await
            .map_err(|report| map_report("Apple account signing failed", report))
    })
    .await?;
    drop(sideloader);
    let (app_name, bundle_id, version) = read_signed_bundle_info(&signed_app)?;
    let payload = signed_app
        .parent()
        .filter(|path| path.file_name().is_some_and(|name| name == "Payload"))
        .ok_or_else(|| account_error("The signed app has an unexpected folder layout", false))?;
    let archive_root = payload
        .parent()
        .ok_or_else(|| account_error("The signed app has no archive root", false))?;

    emit_progress(app, &request.operation_id, "Packaging the signed IPA", 86);
    let staging_path = output.with_file_name(format!(
        ".idevice_desktop-account-signed-{}.ipa",
        Uuid::new_v4()
    ));
    let mut staged = StagedOutput {
        path: staging_path.clone(),
        committed: false,
    };
    let mut child = tokio::process::Command::new("/usr/bin/zip")
        .args(["-qry"])
        .arg(&staging_path)
        .arg(".")
        .current_dir(archive_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| account_error(format!("Unable to package the IPA: {error}"), false))?;
    let package = wait_child(token, "Packaging IPA", Duration::from_secs(120), &mut child).await?;
    if !package.success() {
        return Err(account_error(
            format!("Unable to package the IPA: {package}"),
            false,
        ));
    }
    // Once publishing begins, complete the atomic rename and report success.
    // Never delete an existing export before its replacement is ready.
    if token.is_cancelled() {
        return Err(CommandError::new(
            "cancelled",
            "Signing IPA cancelled",
            false,
        ));
    }
    let size_bytes = tokio::fs::metadata(&staging_path).await?.len();
    tokio::fs::rename(&staging_path, &output).await?;
    staged.committed = true;
    emit_progress(app, &request.operation_id, "Signed IPA ready", 100);

    Ok(PersonalAccountSigningResult {
        output_path: output.to_string_lossy().into_owned(),
        app_name: if app_name.is_empty() {
            inspected.app_name
        } else {
            app_name
        },
        bundle_id,
        version: if version == "—" {
            inspected.version
        } else {
            version
        },
        team_id: active.summary.team_id.clone(),
        team_name: active.summary.team_name.clone(),
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn account_status_never_claims_the_password_is_stored() {
        let status = status_for(None);
        assert!(!status.password_stored);
        assert_eq!(status.credential_storage, "macOS Keychain");
        assert!(status.anisette_server.starts_with("https://"));
    }

    #[test]
    fn report_mapping_marks_transient_network_failures_retryable() {
        let error = map_report("Login failed", report!("connection timed out"));
        assert!(error.retryable);
        assert_eq!(error.kind, "personal_account");
    }

    #[test]
    fn report_mapping_keeps_rejected_credentials_non_retryable() {
        let error = map_report("Login failed", report!("authentication rejected"));
        assert!(!error.retryable);
    }
    #[tokio::test]
    async fn failed_export_cleans_staging_and_preserves_the_previous_export() {
        let directory = std::env::temp_dir().join(format!("signing-test-{}", Uuid::new_v4()));
        std::fs::create_dir(&directory).unwrap();
        let output = directory.join("export.ipa");
        let staged = directory.join("staging.ipa");
        std::fs::write(&output, b"previous export").unwrap();
        std::fs::write(&staged, b"incomplete replacement").unwrap();
        drop(StagedOutput {
            path: staged.clone(),
            committed: false,
        });
        assert!(!staged.exists());
        assert_eq!(std::fs::read(&output).unwrap(), b"previous export");
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[tokio::test]
    async fn temporary_copy_is_removed_with_its_extracted_files() {
        let source = std::env::temp_dir().join(format!("signing-source-{}.ipa", Uuid::new_v4()));
        std::fs::write(&source, b"original").unwrap();
        let temporary = TemporaryInput::copy_from(&source).await.unwrap();
        let ipa = temporary.ipa.clone();
        let extracted = temporary.extracted.clone();
        std::fs::create_dir(&extracted).unwrap();
        std::fs::write(extracted.join("partial"), b"partial").unwrap();
        drop(temporary);
        assert!(!ipa.exists());
        assert!(!extracted.exists());
        assert_eq!(std::fs::read(&source).unwrap(), b"original");
        std::fs::remove_file(source).unwrap();
    }
}
