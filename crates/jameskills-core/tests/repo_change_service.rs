use async_trait::async_trait;
use jameskills_core::{
    AppError, AppResult, ContentHash,
    application::{RepoPolicyRequest, RepositoryChangeService},
    domain::{RepoChangePlan, RepoTemplateId, policy::RepositoryHead},
    ports::{
        RepoChangePort,
        process::{ApprovedRoot, CancellationToken},
    },
};
use std::{
    future::Future,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Wake, Waker},
};

fn block_on<F: Future>(future: F) -> F::Output {
    struct Noop;
    impl Wake for Noop {
        fn wake(self: Arc<Self>) {}
    }
    let waker = Waker::from(Arc::new(Noop));
    let mut context = Context::from_waker(&waker);
    let mut future = Box::pin(future);
    loop {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(output) => return output,
            Poll::Pending => std::thread::yield_now(),
        }
    }
}

fn hash(byte: char) -> ContentHash {
    ContentHash::parse_hex(&byte.to_string().repeat(64)).unwrap()
}

fn request(root_fingerprint: char) -> RepoPolicyRequest {
    RepoPolicyRequest::new(
        ApprovedRoot::from_absolute_path(std::env::current_dir().unwrap()).unwrap(),
        hash(root_fingerprint),
        RepositoryHead::parse(&"a".repeat(40)).unwrap(),
        RepoTemplateId::RustCi,
    )
}

#[derive(Default)]
struct FakeRepoChangePort {
    previews: AtomicUsize,
    applies: AtomicUsize,
}

#[async_trait]
impl RepoChangePort for FakeRepoChangePort {
    async fn preview(
        &self,
        _root: &ApprovedRoot,
        root_fingerprint: &ContentHash,
        expected_head: &RepositoryHead,
        template_id: RepoTemplateId,
        _cancellation: CancellationToken,
    ) -> AppResult<RepoChangePlan> {
        self.previews.fetch_add(1, Ordering::Relaxed);
        RepoChangePlan::new(
            template_id,
            root_fingerprint.clone(),
            expected_head.clone(),
            None,
            b"registered workflow",
            "+ new workflow\n".to_owned(),
        )
        .map_err(AppError::Validation)
    }

    async fn apply(
        &self,
        _root: &ApprovedRoot,
        _approval: jameskills_core::domain::ApprovedRepoChange,
        _cancellation: CancellationToken,
    ) -> AppResult<()> {
        self.applies.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[test]
fn preview_calls_only_the_read_only_port() {
    let port = Arc::new(FakeRepoChangePort::default());
    let service = RepositoryChangeService::new(port.clone());
    let preview =
        block_on(service.plan_repo_changes(&request('b'), CancellationToken::new())).unwrap();

    assert_eq!(
        preview.target().as_str(),
        ".github/workflows/jameskills-ci.yml"
    );
    assert_eq!(port.previews.load(Ordering::Relaxed), 1);
    assert_eq!(port.applies.load(Ordering::Relaxed), 0);
}

#[test]
fn apply_rejects_root_mismatch_and_wrong_digest_before_calling_write_port() {
    let port = Arc::new(FakeRepoChangePort::default());
    let service = RepositoryChangeService::new(port.clone());
    let preview =
        block_on(service.plan_repo_changes(&request('b'), CancellationToken::new())).unwrap();
    let digest = preview.confirmation_digest().clone();

    assert!(matches!(
        block_on(service.apply_repo_changes(
            request('c'),
            preview.clone(),
            &digest,
            CancellationToken::new(),
        )),
        Err(AppError::Validation(_))
    ));
    assert_eq!(port.applies.load(Ordering::Relaxed), 0);

    assert!(matches!(
        block_on(service.apply_repo_changes(
            request('b'),
            preview,
            &hash('f'),
            CancellationToken::new(),
        )),
        Err(AppError::Validation(_))
    ));
    assert_eq!(port.applies.load(Ordering::Relaxed), 0);
}

#[test]
fn exact_digest_approval_calls_the_write_port_and_returns_receipt() {
    let port = Arc::new(FakeRepoChangePort::default());
    let service = RepositoryChangeService::new(port.clone());
    let preview =
        block_on(service.plan_repo_changes(&request('b'), CancellationToken::new())).unwrap();
    let digest = preview.confirmation_digest().clone();

    let receipt = block_on(service.apply_repo_changes(
        request('b'),
        preview.clone(),
        &digest,
        CancellationToken::new(),
    ))
    .unwrap();

    assert_eq!(receipt.operation_id(), preview.operation_id());
    assert_eq!(receipt.target(), preview.target());
    assert_eq!(receipt.applied_hash(), preview.proposed_hash());
    assert_eq!(port.applies.load(Ordering::Relaxed), 1);
}
