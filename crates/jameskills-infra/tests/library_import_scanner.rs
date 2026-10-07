use async_trait::async_trait;
use jameskills_core::{
    AppResult,
    domain::{ImportScanStatus, ToolId},
    ports::{
        ImportScanPort,
        filesystem::BundleFiles,
        process::{ApprovedEnv, ExecutableFingerprint, ProcessOutput, ProcessPort, ProcessSpec},
    },
};
use jameskills_infra::{
    fs::GitleaksImportScanner,
    platform::{
        HostPlatform, ToolCandidate, ToolProfile, find_tool_candidates, load_tool_profiles,
    },
};
use std::{
    collections::BTreeMap,
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

static NEXT_ROOT: AtomicU64 = AtomicU64::new(0);

fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap()
        .block_on(future)
}

struct TestRoot(PathBuf);

impl TestRoot {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "jameskills-import-scan-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, Ordering::Relaxed),
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        Self(root)
    }
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[derive(Clone, Copy)]
enum Report {
    Clean,
    Findings,
    Malformed,
}

struct FakeProcess {
    version_args: Vec<OsString>,
    skill_bytes: Vec<u8>,
    report: Report,
    calls: Mutex<Vec<Vec<OsString>>>,
}

#[async_trait]
impl ProcessPort for FakeProcess {
    async fn run(&self, spec: ProcessSpec) -> AppResult<ProcessOutput> {
        let args = spec.args().to_vec();
        self.calls.lock().unwrap().push(args.clone());
        if args == self.version_args {
            return Ok(ProcessOutput::new(
                Some(0),
                b"8.30.1\n".to_vec(),
                Vec::new(),
            ));
        }
        assert!(matches!(spec.tool_id(), Some(ToolId::Gitleaks)));
        assert_eq!(
            std::fs::read(spec.cwd().path().join("SKILL.md")).unwrap(),
            self.skill_bytes
        );
        match self.report {
            Report::Clean => Ok(ProcessOutput::new(Some(0), b"[]".to_vec(), Vec::new())),
            Report::Findings => Ok(ProcessOutput::new(
                Some(3),
                include_bytes!("../../../tests/fixtures/repo-policy/gitleaks-findings.json")
                    .to_vec(),
                Vec::new(),
            )),
            Report::Malformed => Ok(ProcessOutput::new(
                Some(0),
                b"not-json".to_vec(),
                Vec::new(),
            )),
        }
    }
}

fn gitleaks_profile_and_candidate(root: &Path) -> (ToolProfile, ToolCandidate) {
    let profile = load_tool_profiles()
        .unwrap()
        .into_iter()
        .find(|profile| profile.tool_id() == ToolId::Gitleaks)
        .unwrap();
    std::fs::write(root.join("gitleaks"), b"synthetic executable").unwrap();
    let search_path = std::fs::canonicalize(root).unwrap();
    let candidate = find_tool_candidates(
        std::slice::from_ref(&profile),
        &[search_path],
        HostPlatform::Linux,
    )
    .into_iter()
    .next()
    .unwrap();
    (profile, candidate)
}

fn import_files() -> BundleFiles {
    [
        (
            "SKILL.md",
            include_bytes!("../../../docs/examples/repository-foundation/SKILL.md").as_slice(),
        ),
        (
            "jameskills.toml",
            include_bytes!("../../../docs/examples/repository-foundation/jameskills.toml")
                .as_slice(),
        ),
        (
            "policies/repository.toml",
            include_bytes!("../../../docs/examples/repository-foundation/policies/repository.toml")
                .as_slice(),
        ),
        (
            "guidance/repository.toml",
            include_bytes!("../../../docs/examples/repository-foundation/guidance/repository.toml")
                .as_slice(),
        ),
    ]
    .into_iter()
    .map(|(path, bytes)| {
        (
            jameskills_core::domain::PortablePath::new(path.to_owned()).unwrap(),
            bytes.to_vec(),
        )
    })
    .collect()
}

fn scanner(
    root: &TestRoot,
    report: Report,
    fingerprint: Option<ExecutableFingerprint>,
) -> (GitleaksImportScanner, Arc<FakeProcess>) {
    let (profile, candidate) = gitleaks_profile_and_candidate(&root.0);
    let files = import_files();
    let skill_bytes =
        files[&jameskills_core::domain::PortablePath::new("SKILL.md".to_owned()).unwrap()].clone();
    let fake = Arc::new(FakeProcess {
        version_args: profile
            .version_args()
            .iter()
            .cloned()
            .map(OsString::from)
            .collect(),
        skill_bytes,
        report,
        calls: Mutex::new(Vec::new()),
    });
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let scanner = GitleaksImportScanner::new(
        profile,
        candidate,
        fingerprint,
        &environment,
        fake.clone(),
        root.0.join("cache").join("import-scans"),
    );
    (scanner, fake)
}

#[test]
fn scanner_stages_exact_validated_bytes_redacts_findings_and_cleans_staging() {
    for (report, expected) in [
        (Report::Clean, ImportScanStatus::NoFindings),
        (Report::Findings, ImportScanStatus::Findings),
        (Report::Malformed, ImportScanStatus::Unknown),
    ] {
        let root = TestRoot::new();
        let fingerprint = ExecutableFingerprint::from_sha256([0x42; 32]);
        let (scanner, fake) = scanner(&root, report, Some(fingerprint));
        let observed = block_on(scanner.scan(&import_files())).unwrap();
        assert_eq!(
            observed,
            expected,
            "process calls: {:?}; staging root: {}",
            *fake.calls.lock().unwrap(),
            root.0.join("cache").join("import-scans").display(),
        );
        let calls = fake.calls.lock().unwrap();
        assert_eq!(calls.len(), 2);
        assert!(calls.iter().all(|args| {
            !args
                .iter()
                .any(|arg| arg.to_string_lossy().contains("REDACTED-FIXTURE"))
        }));
        assert!(
            std::fs::read_dir(root.0.join("cache").join("import-scans"))
                .unwrap()
                .next()
                .is_none(),
            "per-scan staged files must be removed before returning"
        );
    }
}

#[test]
fn scanner_without_approved_fingerprint_blocks_without_process_or_staging() {
    let root = TestRoot::new();
    let (scanner, fake) = scanner(&root, Report::Clean, None);
    assert_eq!(
        block_on(scanner.scan(&import_files())).unwrap(),
        ImportScanStatus::Blocked
    );
    assert!(fake.calls.lock().unwrap().is_empty());
    assert!(!root.0.join("cache").exists());
}

#[test]
fn imported_gitleaks_ignore_file_blocks_before_process_spawn() {
    let root = TestRoot::new();
    let fingerprint = ExecutableFingerprint::from_sha256([0x42; 32]);
    let (profile, candidate) = gitleaks_profile_and_candidate(&root.0);
    let fake = Arc::new(FakeProcess {
        version_args: profile
            .version_args()
            .iter()
            .cloned()
            .map(OsString::from)
            .collect(),
        skill_bytes: Vec::new(),
        report: Report::Clean,
        calls: Mutex::new(Vec::new()),
    });
    let environment = ApprovedEnv::new(BTreeMap::new()).unwrap();
    let scanner = GitleaksImportScanner::new(
        profile,
        candidate,
        Some(fingerprint),
        &environment,
        fake.clone(),
        root.0.join("cache").join("import-scans"),
    );
    let mut files = import_files();
    files.insert(
        jameskills_core::domain::PortablePath::new(".gitleaksignore".to_owned()).unwrap(),
        b"ignored-secret".to_vec(),
    );
    assert_eq!(
        block_on(scanner.scan(&files)).unwrap(),
        ImportScanStatus::Blocked
    );
    assert!(fake.calls.lock().unwrap().is_empty());
    assert!(
        std::fs::read_dir(root.0.join("cache").join("import-scans"))
            .unwrap()
            .next()
            .is_none()
    );
}
