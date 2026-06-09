const LOGIN_ATTEMPT_SCHEMA_MIGRATION: &str =
    include_str!("../migrations/202606080009_create_login_failure_counters.sql");

#[test]
fn login_attempt_schema_migration_adds_brute_force_lockout_state() {
    for required_fragment in [
        "CREATE TABLE login_failure_counters",
        "subject TEXT PRIMARY KEY",
        "failed_attempts INTEGER NOT NULL DEFAULT 0",
        "locked_until TIMESTAMPTZ NULL",
        "last_failed_at TIMESTAMPTZ NULL",
        "idx_login_failure_counters_locked_until",
    ] {
        assert!(
            LOGIN_ATTEMPT_SCHEMA_MIGRATION.contains(required_fragment),
            "migration should contain: {required_fragment}"
        );
    }
}
