use atlas_auth::auth::login_attempt_repository::LoginAttemptRepository;
use chrono::{Duration, Utc};

#[actix_rt::test]
async fn login_attempt_repository_locks_subject_after_threshold_failures() {
    let repository = LoginAttemptRepository::in_memory();
    let now = Utc::now();

    for _ in 0..4 {
        let status = repository
            .record_failure("USER@Example.TEST", 5, Duration::minutes(15), now)
            .await
            .unwrap();
        assert!(!status.locked);
    }

    let status = repository
        .record_failure("user@example.test", 5, Duration::minutes(15), now)
        .await
        .unwrap();

    assert!(status.locked);
    assert_eq!(status.failed_attempts, 5);
    assert!(status.locked_until.unwrap() > now);
}

#[actix_rt::test]
async fn login_attempt_repository_clears_failures_after_success() {
    let repository = LoginAttemptRepository::in_memory();
    let now = Utc::now();

    repository
        .record_failure("user@example.test", 5, Duration::minutes(15), now)
        .await
        .unwrap();
    repository.clear("user@example.test").await.unwrap();

    let status = repository
        .status("user@example.test", Utc::now())
        .await
        .unwrap();

    assert_eq!(status.failed_attempts, 0);
    assert!(!status.locked);
    assert!(status.locked_until.is_none());
}

#[actix_rt::test]
async fn login_attempt_repository_unlocks_after_lock_window_expires() {
    let repository = LoginAttemptRepository::in_memory();
    let now = Utc::now();

    repository
        .record_failure("user@example.test", 1, Duration::seconds(1), now)
        .await
        .unwrap();

    let status = repository
        .status("user@example.test", now + Duration::seconds(2))
        .await
        .unwrap();

    assert!(!status.locked);
    assert!(status.locked_until.is_none());
}
