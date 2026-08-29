use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use idevice::{IdeviceService, installation_proxy::InstallationProxyClient};
use plist::{Dictionary, Value};
use tauri::State;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    device_version::{DeveloperGeneration, IosVersion, ios_version},
    error::{CommandError, CommandResult},
    provider::selected_provider,
    state::AppState,
    types::{
        XCTestPlanRequest, XCTestPreflightSnapshot, XCTestRunPlan, XCTestRunnerCandidate,
        XCTestTargetApp,
    },
    utils::dict_string,
};

const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(45);
const MAX_RUNNERS: usize = 128;
const MAX_TARGETS: usize = 512;
const MAX_TEST_FILTERS: usize = 256;
const MAX_TEST_FILTER_LENGTH: usize = 256;

struct XCTestCatalog {
    runners: Vec<XCTestRunnerCandidate>,
    targets: Vec<XCTestTargetApp>,
    runner_total: u64,
    target_total: u64,
    truncated: bool,
}

fn app_is_debuggable(dictionary: &Dictionary) -> bool {
    dictionary
        .get("Entitlements")
        .and_then(Value::as_dictionary)
        .and_then(|entitlements| entitlements.get("get-task-allow"))
        .and_then(Value::as_boolean)
        .unwrap_or(false)
}

fn app_name(bundle_id: &str, dictionary: &Dictionary) -> String {
    dict_string(dictionary, &["CFBundleDisplayName", "CFBundleName"])
        .unwrap_or_else(|| bundle_id.to_owned())
}

fn app_version(dictionary: &Dictionary) -> String {
    dict_string(
        dictionary,
        &["CFBundleShortVersionString", "CFBundleVersion"],
    )
    .unwrap_or_default()
}

fn is_runner(bundle_id: &str, dictionary: &Dictionary) -> bool {
    bundle_id.to_ascii_lowercase().ends_with(".xctrunner")
        || dict_string(dictionary, &["CFBundleExecutable"])
            .is_some_and(|executable| executable.ends_with("-Runner"))
}

fn runner_candidate(bundle_id: String, dictionary: &Dictionary) -> Option<XCTestRunnerCandidate> {
    if !is_runner(&bundle_id, dictionary) {
        return None;
    }
    let name = app_name(&bundle_id, dictionary);
    let version = app_version(dictionary);
    let executable = dict_string(dictionary, &["CFBundleExecutable"]);
    let mut required_issues = Vec::new();
    if dict_string(dictionary, &["Path"]).is_none() {
        required_issues.push("Application path is missing".into());
    }
    if dict_string(dictionary, &["Container"]).is_none() {
        required_issues.push("Application container is missing".into());
    }
    if !executable
        .as_deref()
        .is_some_and(|value| value.ends_with("-Runner"))
    {
        required_issues.push("Executable must end with -Runner".into());
    }
    let configuration_ready = required_issues.is_empty();
    let debuggable = app_is_debuggable(dictionary);
    let mut issues = required_issues;
    if !debuggable {
        issues.push("get-task-allow is not enabled".into());
    }
    let runner_identity =
        format!("{bundle_id} {name} {}", executable.as_deref().unwrap_or("")).to_ascii_lowercase();
    Some(XCTestRunnerCandidate {
        bundle_id,
        name,
        version,
        executable,
        debuggable,
        is_webdriver_agent: runner_identity.contains("webdriveragent"),
        configuration_ready,
        issues,
    })
}

fn target_candidate(bundle_id: String, dictionary: &Dictionary) -> Option<XCTestTargetApp> {
    let debuggable = app_is_debuggable(dictionary);
    let is_user = dict_string(dictionary, &["ApplicationType"])
        .is_some_and(|value| value.eq_ignore_ascii_case("user"));
    if !is_user && !debuggable {
        return None;
    }
    Some(XCTestTargetApp {
        name: app_name(&bundle_id, dictionary),
        version: app_version(dictionary),
        bundle_id,
        debuggable,
    })
}

fn catalog_apps(apps: HashMap<String, Value>) -> XCTestCatalog {
    let mut runners = Vec::new();
    let mut targets = Vec::new();
    for (bundle_id, value) in apps {
        let Some(dictionary) = value.as_dictionary() else {
            continue;
        };
        if let Some(runner) = runner_candidate(bundle_id.clone(), dictionary) {
            runners.push(runner);
        } else if let Some(target) = target_candidate(bundle_id, dictionary) {
            targets.push(target);
        }
    }
    runners.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.bundle_id.cmp(&right.bundle_id))
    });
    targets.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.bundle_id.cmp(&right.bundle_id))
    });
    let runner_total = u64::try_from(runners.len()).unwrap_or(u64::MAX);
    let target_total = u64::try_from(targets.len()).unwrap_or(u64::MAX);
    let truncated = runners.len() > MAX_RUNNERS || targets.len() > MAX_TARGETS;
    runners.truncate(MAX_RUNNERS);
    targets.truncate(MAX_TARGETS);
    XCTestCatalog {
        runners,
        targets,
        runner_total,
        target_total,
        truncated,
    }
}

fn route(version: IosVersion) -> (String, bool, Option<String>) {
    if version.major < 11 {
        return (
            "XCTest unavailable".into(),
            false,
            Some("XCTest execution requires iOS 11 or later".into()),
        );
    }
    match version.developer_generation() {
        DeveloperGeneration::Legacy => ("Lockdown TestManager/DVT".into(), true, None),
        DeveloperGeneration::CoreDeviceRemote => (
            "RemotePairing/RSD route pending".into(),
            false,
            Some(
                "iOS 17.0–17.3 requires a RemotePairing/RSD XCTest adapter before run controls can be enabled"
                    .into(),
            ),
        ),
        DeveloperGeneration::CoreDeviceLockdown => {
            ("CoreDeviceProxy/RSD TestManager/DVT".into(), true, None)
        }
    }
}

fn version_label(version: IosVersion) -> String {
    if version.patch > 0 {
        format!("{}.{}.{}", version.major, version.minor, version.patch)
    } else {
        format!("{}.{}", version.major, version.minor)
    }
}

async fn load_preflight(
    state: &AppState,
    udid: Option<String>,
) -> CommandResult<XCTestPreflightSnapshot> {
    let (_, provider) = selected_provider(state, udid).await?;
    let version = ios_version(&provider).await?;
    let (transport, execution_supported, limitation) = route(version);
    let mut client = InstallationProxyClient::connect(&provider)
        .await
        .map_err(CommandError::from)?;
    let apps = client
        .get_apps(None, None)
        .await
        .map_err(CommandError::from)?;
    let catalog = catalog_apps(apps);
    Ok(XCTestPreflightSnapshot {
        ios_version: version_label(version),
        transport,
        execution_supported,
        limitation,
        runner_total: catalog.runner_total,
        target_total: catalog.target_total,
        truncated: catalog.truncated,
        runners: catalog.runners,
        targets: catalog.targets,
    })
}

fn configuration_error(message: impl Into<String>) -> CommandError {
    CommandError::new("configuration", message, false)
}

fn normalize_filters(values: Vec<String>, label: &str) -> CommandResult<Vec<String>> {
    if values.len() > MAX_TEST_FILTERS {
        return Err(configuration_error(format!(
            "{label} accepts at most {MAX_TEST_FILTERS} test identifiers"
        )));
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = value.trim();
        if value.is_empty() {
            continue;
        }
        if value.chars().count() > MAX_TEST_FILTER_LENGTH {
            return Err(configuration_error(format!(
                "{label} test identifiers must be at most {MAX_TEST_FILTER_LENGTH} characters"
            )));
        }
        if value.chars().any(char::is_control) {
            return Err(configuration_error(format!(
                "{label} test identifiers cannot contain control characters"
            )));
        }
        if seen.insert(value.to_owned()) {
            normalized.push(value.to_owned());
        }
    }
    Ok(normalized)
}

fn prepare_plan(
    snapshot: XCTestPreflightSnapshot,
    request: XCTestPlanRequest,
) -> CommandResult<XCTestRunPlan> {
    if !snapshot.execution_supported {
        return Err(CommandError::new(
            "xctest",
            snapshot.limitation.unwrap_or_else(|| {
                "XCTest execution is unavailable for this device generation".into()
            }),
            false,
        ));
    }
    let runner = snapshot
        .runners
        .iter()
        .find(|candidate| candidate.bundle_id == request.runner_bundle_id)
        .ok_or_else(|| configuration_error("The selected XCTest runner is no longer installed"))?;
    if !runner.configuration_ready {
        return Err(configuration_error(format!(
            "{} is missing required runner metadata",
            runner.name
        )));
    }
    if !runner.debuggable {
        return Err(configuration_error(format!(
            "{} does not allow debugger attachment",
            runner.name
        )));
    }

    let (target_bundle_id, target_name) = match request.target_bundle_id.as_deref() {
        Some(bundle_id) => {
            let target = snapshot
                .targets
                .iter()
                .find(|candidate| candidate.bundle_id == bundle_id)
                .ok_or_else(|| {
                    configuration_error("The selected target application is no longer installed")
                })?;
            (Some(target.bundle_id.clone()), Some(target.name.clone()))
        }
        None => (None, None),
    };

    let tests_to_run = normalize_filters(request.tests_to_run, "Included tests")?;
    let tests_to_skip = normalize_filters(request.tests_to_skip, "Skipped tests")?;
    let skipped: HashSet<&str> = tests_to_skip.iter().map(String::as_str).collect();
    if let Some(overlap) = tests_to_run
        .iter()
        .find(|identifier| skipped.contains(identifier.as_str()))
    {
        return Err(configuration_error(format!(
            "{overlap} cannot be both included and skipped"
        )));
    }

    let wda_bridge = match request.mode.as_str() {
        "test" => {
            if !(30..=7200).contains(&request.timeout_seconds) {
                return Err(configuration_error(
                    "XCTest timeout must be between 30 and 7200 seconds",
                ));
            }
            false
        }
        "wda" => {
            if !runner.is_webdriver_agent {
                return Err(configuration_error(
                    "WebDriverAgent mode requires a WebDriverAgent runner",
                ));
            }
            if target_bundle_id.is_some() || !tests_to_run.is_empty() || !tests_to_skip.is_empty() {
                return Err(configuration_error(
                    "WebDriverAgent mode does not accept a target application or test filters",
                ));
            }
            if !(5..=300).contains(&request.timeout_seconds) {
                return Err(configuration_error(
                    "WebDriverAgent readiness timeout must be between 5 and 300 seconds",
                ));
            }
            true
        }
        _ => {
            return Err(configuration_error(
                "XCTest plan mode must be either test or wda",
            ));
        }
    };

    Ok(XCTestRunPlan {
        runner_bundle_id: runner.bundle_id.clone(),
        runner_name: runner.name.clone(),
        target_bundle_id,
        target_name,
        mode: request.mode,
        tests_to_run,
        tests_to_skip,
        timeout_seconds: request.timeout_seconds,
        wda_bridge,
        transport: snapshot.transport,
    })
}

#[tauri::command]
pub async fn xctest_preflight(
    state: State<'_, AppState>,
    udid: Option<String>,
) -> CommandResult<XCTestPreflightSnapshot> {
    let token = CancellationToken::new();
    let task_key = format!("xctest-preflight-{}", Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let result = tokio::select! {
        biased;
        _ = token.cancelled() => Err(CommandError::new(
            "cancelled",
            "XCTest preflight stopped because the device session changed",
            false,
        )),
        result = tokio::time::timeout(PREFLIGHT_TIMEOUT, load_preflight(&state, udid)) => result
            .map_err(|_| CommandError::new(
                "xctest",
                "Timed out inspecting XCTest runners. Keep the device unlocked and retry.",
                true,
            ))?,
    };
    state.cancel_task(&task_key).await;
    result
}

#[tauri::command]
pub async fn xctest_plan_prepare(
    state: State<'_, AppState>,
    udid: Option<String>,
    request: XCTestPlanRequest,
) -> CommandResult<XCTestRunPlan> {
    let token = CancellationToken::new();
    let task_key = format!("xctest-plan-{}", Uuid::new_v4());
    state.replace_task(task_key.clone(), token.clone()).await;
    let result = tokio::select! {
        biased;
        _ = token.cancelled() => Err(CommandError::new(
            "cancelled",
            "XCTest plan validation stopped because the device session changed",
            false,
        )),
        result = tokio::time::timeout(PREFLIGHT_TIMEOUT, load_preflight(&state, udid)) => {
            let snapshot = result.map_err(|_| CommandError::new(
                "xctest",
                "Timed out validating the XCTest plan. Keep the device unlocked and retry.",
                true,
            ))??;
            prepare_plan(snapshot, request)
        },
    };
    state.cancel_task(&task_key).await;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(
        application_type: &str,
        executable: Option<&str>,
        path: bool,
        container: bool,
        debuggable: bool,
    ) -> Value {
        let mut dictionary = Dictionary::new();
        dictionary.insert("ApplicationType".into(), application_type.into());
        dictionary.insert("CFBundleDisplayName".into(), "Example App".into());
        dictionary.insert("CFBundleShortVersionString".into(), "1.2.3".into());
        if let Some(executable) = executable {
            dictionary.insert("CFBundleExecutable".into(), executable.into());
        }
        if path {
            dictionary.insert("Path".into(), "/private/Example.app".into());
        }
        if container {
            dictionary.insert("Container".into(), "/private/container".into());
        }
        if debuggable {
            let mut entitlements = Dictionary::new();
            entitlements.insert("get-task-allow".into(), true.into());
            dictionary.insert("Entitlements".into(), entitlements.into());
        }
        Value::Dictionary(dictionary)
    }

    #[test]
    fn classifies_ready_wda_and_preserves_malformed_runner_issues() {
        let ready = runner_candidate(
            "com.example.WebDriverAgentRunner.xctrunner".into(),
            app(
                "User",
                Some("WebDriverAgentRunner-Runner"),
                true,
                true,
                true,
            )
            .as_dictionary()
            .unwrap(),
        )
        .unwrap();
        assert!(ready.configuration_ready);
        assert!(ready.debuggable);
        assert!(ready.is_webdriver_agent);
        assert!(ready.issues.is_empty());

        let malformed = runner_candidate(
            "com.example.Broken.xctrunner".into(),
            app("User", None, false, false, false)
                .as_dictionary()
                .unwrap(),
        )
        .unwrap();
        assert!(!malformed.configuration_ready);
        assert!(!malformed.debuggable);
        assert_eq!(malformed.issues.len(), 4);
    }

    #[test]
    fn catalogs_user_and_debuggable_targets_with_bounded_output() {
        let mut apps = HashMap::new();
        apps.insert(
            "com.example.Runner.xctrunner".into(),
            app("User", Some("Example-Runner"), true, true, true),
        );
        apps.insert(
            "com.example.Target".into(),
            app("User", Some("Target"), true, true, false),
        );
        apps.insert(
            "com.example.SideLoaded".into(),
            app("System", Some("SideLoaded"), true, true, true),
        );
        apps.insert(
            "com.apple.Settings".into(),
            app("System", Some("Preferences"), true, true, false),
        );
        let catalog = catalog_apps(apps);
        assert_eq!(catalog.runners.len(), 1);
        assert_eq!(catalog.targets.len(), 2);
        assert!(
            catalog
                .targets
                .iter()
                .any(|target| target.bundle_id == "com.example.SideLoaded")
        );
        assert!(
            catalog
                .targets
                .iter()
                .all(|target| target.bundle_id != "com.apple.Settings")
        );
    }

    #[test]
    fn exposes_only_execution_routes_the_app_can_currently_complete() {
        let legacy = route(IosVersion {
            major: 16,
            minor: 7,
            patch: 0,
        });
        assert!(legacy.1);
        assert_eq!(legacy.0, "Lockdown TestManager/DVT");

        let remote = route(IosVersion {
            major: 17,
            minor: 0,
            patch: 0,
        });
        assert!(!remote.1);
        assert!(remote.2.unwrap().contains("RemotePairing/RSD"));

        let modern = route(IosVersion {
            major: 17,
            minor: 4,
            patch: 0,
        });
        assert!(modern.1);
        assert_eq!(modern.0, "CoreDeviceProxy/RSD TestManager/DVT");
    }

    fn ready_snapshot(is_webdriver_agent: bool) -> XCTestPreflightSnapshot {
        XCTestPreflightSnapshot {
            ios_version: "17.4".into(),
            transport: "CoreDeviceProxy/RSD TestManager/DVT".into(),
            execution_supported: true,
            limitation: None,
            runner_total: 1,
            target_total: 1,
            truncated: false,
            runners: vec![XCTestRunnerCandidate {
                bundle_id: "com.example.Runner.xctrunner".into(),
                name: "Example Runner".into(),
                version: "1.0".into(),
                executable: Some("Example-Runner".into()),
                debuggable: true,
                is_webdriver_agent,
                configuration_ready: true,
                issues: Vec::new(),
            }],
            targets: vec![XCTestTargetApp {
                bundle_id: "com.example.Target".into(),
                name: "Example Target".into(),
                version: "2.0".into(),
                debuggable: true,
            }],
        }
    }

    fn request(mode: &str) -> XCTestPlanRequest {
        XCTestPlanRequest {
            runner_bundle_id: "com.example.Runner.xctrunner".into(),
            target_bundle_id: None,
            mode: mode.into(),
            tests_to_run: Vec::new(),
            tests_to_skip: Vec::new(),
            timeout_seconds: if mode == "wda" { 30 } else { 900 },
        }
    }

    #[test]
    fn normalizes_bounded_filters_and_rejects_ambiguous_values() {
        assert_eq!(
            normalize_filters(
                vec![" Suite/Test ".into(), "Suite/Test".into(), "".into()],
                "Included tests"
            )
            .unwrap(),
            vec!["Suite/Test"]
        );
        assert!(normalize_filters(vec!["Suite/\u{7}Test".into()], "Included tests").is_err());
        assert!(
            normalize_filters(vec!["Test".into(); MAX_TEST_FILTERS + 1], "Included tests").is_err()
        );

        let mut overlapping = request("test");
        overlapping.tests_to_run = vec!["Suite/Test".into()];
        overlapping.tests_to_skip = vec!["Suite/Test".into()];
        assert!(prepare_plan(ready_snapshot(false), overlapping).is_err());
    }

    #[test]
    fn prepares_a_standard_plan_from_fresh_runner_and_target_metadata() {
        let mut request = request("test");
        request.target_bundle_id = Some("com.example.Target".into());
        request.tests_to_run = vec![" Suite/TestA ".into(), "Suite/TestA".into()];
        request.tests_to_skip = vec!["Suite/TestB".into()];
        let plan = prepare_plan(ready_snapshot(false), request).unwrap();

        assert_eq!(plan.runner_name, "Example Runner");
        assert_eq!(plan.target_name.as_deref(), Some("Example Target"));
        assert_eq!(plan.tests_to_run, vec!["Suite/TestA"]);
        assert_eq!(plan.tests_to_skip, vec!["Suite/TestB"]);
        assert!(!plan.wda_bridge);
        assert_eq!(plan.timeout_seconds, 900);
    }

    #[test]
    fn constrains_webdriveragent_plans_to_bridge_readiness_intent() {
        assert!(prepare_plan(ready_snapshot(false), request("wda")).is_err());

        let valid = prepare_plan(ready_snapshot(true), request("wda")).unwrap();
        assert!(valid.wda_bridge);
        assert_eq!(valid.mode, "wda");

        let mut with_target = request("wda");
        with_target.target_bundle_id = Some("com.example.Target".into());
        assert!(prepare_plan(ready_snapshot(true), with_target).is_err());

        let mut invalid_timeout = request("wda");
        invalid_timeout.timeout_seconds = 301;
        assert!(prepare_plan(ready_snapshot(true), invalid_timeout).is_err());
    }
}
