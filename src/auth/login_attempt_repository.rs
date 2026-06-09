use chrono::{DateTime, Duration, Utc};
use sqlx::{FromRow, PgPool};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoginAttemptStatus {
    pub failed_attempts: i32,
    pub locked: bool,
    pub locked_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
struct LoginFailureCounterRow {
    failed_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
pub struct LoginFailureCounter {
    subject: String,
    failed_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
    last_failed_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub enum LoginAttemptRepository {
    Postgres(PgPool),
    InMemory(Arc<Mutex<Vec<LoginFailureCounter>>>),
}

impl LoginAttemptRepository {
    pub fn postgres(pool: PgPool) -> Self {
        Self::Postgres(pool)
    }

    pub fn in_memory() -> Self {
        Self::InMemory(Arc::new(Mutex::new(Vec::new())))
    }

    pub async fn status(
        &self,
        subject: &str,
        now: DateTime<Utc>,
    ) -> Result<LoginAttemptStatus, sqlx::Error> {
        let normalized = normalize_subject(subject);
        match self {
            Self::Postgres(pool) => {
                let row = sqlx::query_as::<_, LoginFailureCounterRow>(
                    r#"
                        SELECT failed_attempts, locked_until
                        FROM login_failure_counters
                        WHERE subject = $1
                    "#,
                )
                .bind(&normalized)
                .fetch_optional(pool)
                .await?;
                Ok(row_to_status(row, now))
            }
            Self::InMemory(counters) => Ok(counters
                .lock()
                .expect("login failure counter store poisoned")
                .iter()
                .find(|counter| counter.subject == normalized)
                .map(|counter| {
                    status_from_parts(counter.failed_attempts, counter.locked_until, now)
                })
                .unwrap_or_else(empty_status)),
        }
    }

    pub async fn record_failure(
        &self,
        subject: &str,
        threshold: i32,
        lock_window: Duration,
        now: DateTime<Utc>,
    ) -> Result<LoginAttemptStatus, sqlx::Error> {
        let normalized = normalize_subject(subject);
        match self {
            Self::Postgres(pool) => {
                let locked_until_expression = if threshold <= 1 {
                    "locked_until = $4"
                } else {
                    "locked_until = CASE WHEN login_failure_counters.failed_attempts + 1 >= $3 THEN $4 ELSE NULL END"
                };
                let sql = format!(
                    r#"
                        INSERT INTO login_failure_counters (
                            subject,
                            failed_attempts,
                            locked_until,
                            last_failed_at,
                            updated_at
                        ) VALUES ($1, 1, CASE WHEN $3 <= 1 THEN $4 ELSE NULL END, $2, $2)
                        ON CONFLICT (subject) DO UPDATE
                        SET failed_attempts = login_failure_counters.failed_attempts + 1,
                            {locked_until_expression},
                            last_failed_at = $2,
                            updated_at = $2
                        RETURNING failed_attempts, locked_until
                    "#
                );
                let row = sqlx::query_as::<_, LoginFailureCounterRow>(&sql)
                    .bind(&normalized)
                    .bind(now)
                    .bind(threshold)
                    .bind(now + lock_window)
                    .fetch_one(pool)
                    .await?;
                Ok(row_to_status(Some(row), now))
            }
            Self::InMemory(counters) => {
                let mut counters = counters
                    .lock()
                    .expect("login failure counter store poisoned");
                let counter = if let Some(counter) = counters
                    .iter_mut()
                    .find(|counter| counter.subject == normalized)
                {
                    counter
                } else {
                    counters.push(LoginFailureCounter {
                        subject: normalized,
                        failed_attempts: 0,
                        locked_until: None,
                        last_failed_at: None,
                    });
                    counters.last_mut().expect("counter just inserted")
                };
                counter.failed_attempts += 1;
                counter.last_failed_at = Some(now);
                if counter.failed_attempts >= threshold {
                    counter.locked_until = Some(now + lock_window);
                }
                Ok(status_from_parts(
                    counter.failed_attempts,
                    counter.locked_until,
                    now,
                ))
            }
        }
    }

    pub async fn clear(&self, subject: &str) -> Result<(), sqlx::Error> {
        let normalized = normalize_subject(subject);
        match self {
            Self::Postgres(pool) => {
                sqlx::query("DELETE FROM login_failure_counters WHERE subject = $1")
                    .bind(normalized)
                    .execute(pool)
                    .await?;
                Ok(())
            }
            Self::InMemory(counters) => {
                counters
                    .lock()
                    .expect("login failure counter store poisoned")
                    .retain(|counter| counter.subject != normalized);
                Ok(())
            }
        }
    }
}

fn normalize_subject(subject: &str) -> String {
    subject.trim().to_lowercase()
}

fn empty_status() -> LoginAttemptStatus {
    LoginAttemptStatus {
        failed_attempts: 0,
        locked: false,
        locked_until: None,
    }
}

fn row_to_status(row: Option<LoginFailureCounterRow>, now: DateTime<Utc>) -> LoginAttemptStatus {
    row.map(|row| status_from_parts(row.failed_attempts, row.locked_until, now))
        .unwrap_or_else(empty_status)
}

fn status_from_parts(
    failed_attempts: i32,
    locked_until: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> LoginAttemptStatus {
    let active_lock = locked_until.filter(|locked_until| *locked_until > now);
    LoginAttemptStatus {
        failed_attempts,
        locked: active_lock.is_some(),
        locked_until: active_lock,
    }
}
