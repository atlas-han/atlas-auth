# Claude Code 하네스 개선 계획

작성일: 2026-08-29 · 대상: `.claude/` (agents 6, commands 4, skills 2, hooks 3, settings.json) · 기준 버전: Claude Code 2.1.251

## 요약

현재 하네스는 설계 골격이 건전하다 — planner / generator / evaluator 역할 분리, 파일 기반 핸드오프, 하드 임계치 루브릭, dual-backend 리포지토리의 Liskov 검사 강조, 인증 서버답게 security-auditor 상시 실행. 유지할 부분이다.

문제는 골격이 아니라 **현재 실제로 동작하지 않거나 스스로를 해치는 부분**과 **시간이 지나며 어긋날 구조**에 있다. 실측·문서 검증으로 확인한 핵심 4가지:

| # | 발견 | 근거 |
|---|------|------|
| 1 | `rust-postedit.sh`가 `rustfmt --edition 2024`로 포맷하는데 크레이트는 `edition = "2021"` → 63개 `.rs` 중 **35개**가 `cargo fmt --check`와 다르게 포맷됨. CI 게이트를 지키려는 훅이 오히려 게이트를 깬다 | 실측 (`rustfmt 1.9.0`, import 정렬·체인 assert 레이아웃 차이) |
| 2 | `/feature` `/qa` `/solid-check`의 `allowed-tools`에 `Task`, `TodoWrite` — `TodoWrite`는 제거됐고 `Task`는 이제 서브에이전트 도구가 아님(서브에이전트는 `Agent`). 옛 이름은 조용히 무시됨 | Claude Code tools reference (2.1.233+) |
| 3 | 코드베이스 지식(계층, dual-backend, `Option<web::Data>`, 에러 모델, 토큰 불변식, 마이그레이션 페어링)이 **8개 파일에 복제**됨. 비-fork 서브에이전트는 CLAUDE.md를 자동 상속하므로 에이전트 파일의 복제본은 전부 중복이며 drift 원천 | CLAUDE.md, README, architect/developer/evaluator/security-auditor/technical-writer, harness-rubric, solid-rust |
| 4 | `evaluator.md`는 기준 6개, `harness-rubric` 스킬은 8개(#7 build & gates, #8 docs 누락) — 이미 어긋나 있음 | `agents/evaluator.md` "The rubric" vs `skills/harness-rubric/SKILL.md` 표 |

그 외 워크트리 워크플로우와 gitignored 아티팩트 디렉터리의 충돌, 만료 없는 handoff 주입, 검증된 적 없는 WorktreeRemove 차단 semantics, 평가자 보정(calibration) 부재가 뒤따른다.

---

## 진단 상세

심각도: **P0** = 지금 동작을 해침 / **P1** = 구조적 drift·손실 위험 / **P2** = 품질·비용 최적화

### P0

**F1. fmt 훅의 edition 불일치** — `.claude/hooks/rust-postedit.sh`: `rustfmt --edition 2024 "$file_path"`. `Cargo.toml`은 `edition = "2021"`, `rustfmt.toml` 없음. 2024 style edition은 `use` 항목 정렬(대문자 우선)과 체인 메서드 줄바꿈이 달라, Edit/Write된 파일이 그 즉시 `cargo fmt --all -- --check`(CI)에 걸린다.
→ 수정: 파일 단위 `rustfmt` 대신 `$CLAUDE_PROJECT_DIR`에서 `cargo fmt --all` 실행(Cargo.toml의 edition을 자동 사용, 컴파일 없이 1초 미만). edition 인자를 손으로 맞추는 방식은 다음 edition 변경 때 같은 버그를 재현한다.

**F2. 무효한 도구명** — `commands/feature.md:4` `allowed-tools: Task, …, TodoWrite`, `commands/qa.md:4`, `commands/solid-check.md:4` `allowed-tools: Task, …`. 본문도 "Use a TodoWrite list".
→ 수정: `Task` → `Agent`, `TodoWrite` 삭제. 본문의 TodoWrite 지시는 "단계마다 한두 줄로 진행 요약"으로 대체(이미 그 지시가 있으므로 TodoWrite 문장은 삭제만 하면 됨).

**F3. 평가자–루브릭 이중 정의** — `agents/evaluator.md`가 루브릭 기준을 본문에 다시 나열(6개)하고 `harness-rubric`은 8개. 평가자는 스킬을 "use"하라는 텍스트 포인터에만 의존하므로 실제로 어느 쪽을 따를지 실행마다 달라진다.
→ 수정: `evaluator.md` frontmatter에 `skills: [harness-rubric]` 프리로드, 본문의 기준 목록 삭제. 루브릭은 스킬 한 곳에만 존재.

**F4. Bash 편집은 fmt 훅을 우회** — `settings.json` PostToolUse 매처 `Edit|Write|MultiEdit`. auto 모드·developer 에이전트가 `sed`/heredoc으로 편집하면 훅이 안 걸린다(`MultiEdit`는 이미 제거된 도구).
→ 수정: `PreToolUse` 매처 `Bash(git commit *)`에서 `cargo fmt --all -- --check` 실행, 실패 시 exit 2 + 메시지로 커밋 차단. PostToolUse 훅은 편의용으로 유지, 커밋 훅이 실제 보장.

### P1

**F5. 코드베이스 지식 8중 복제** — architect "What you must know about this codebase", developer "Codebase patterns you MUST honor", evaluator 기준 2, security-auditor 체크리스트 절반, technical-writer "Be accurate about the security model", 두 스킬의 코드베이스 절, 그리고 CLAUDE.md·README. `main.rs` 배선 하나가 바뀌면 최대 8곳을 고쳐야 하고, 실제로는 한두 곳만 고쳐져 어긋난다.
→ 수정: 각 에이전트 파일에서 코드베이스 사실 블록을 삭제하고 **역할 고유 델타**만 남긴다(architect: 블루프린트 섹션 구조; developer: 자기 리뷰 체크리스트; evaluator: 회의적 자세와 보고 형식; security-auditor: 위협 모델 중 CLAUDE.md에 없는 항목 — `alg=none`, 상수 시간 비교, 계정 존재 노출, IDOR). 필요한 곳은 "CLAUDE.md의 *Auth & token specifics* 참조"로 포인터만. 예상: agents+commands+skills 752줄 → 약 450줄.

**F6. 배선 상태 스냅샷이 프롬프트에 캐시됨** — 예: security-auditor "login lockout via `LoginAttemptRepository` (note it silently degrades if unwired)", CLAUDE.md의 "What `main.rs` currently wires" 절 전체. `main.rs`가 진실인데 프롬프트가 그 복사본을 들고 있다.
→ 수정: 사실 나열 대신 조회 절차로 교체 — "배선 상태는 `src/main.rs`의 `.app_data(` / `.configure(` 를 grep해서 확인하고, 핸들러 시그니처의 `Option<web::Data<_>>` vs `web::Data<_>`로 degrade/500을 판정". CLAUDE.md의 스냅샷 절은 "현재 미배선 목록"을 유지하되 갱신 규칙(배선 변경 시 함께 수정)을 명시.

**F7. 아티팩트 디렉터리와 워크트리 워크플로우 충돌** — `.claude/harness/`는 gitignored이고 워크트리마다 별도 사본. 워크트리 안에서 `/feature`·`/handoff`가 쓴 plan/eval/handoff는 워크트리 삭제와 함께 사라지고, 메인 체크아웃의 SessionStart 훅은 그것을 보지 못한다. (이 머신의 메인 체크아웃 `.claude/harness/`는 `.gitkeep`뿐 — 훅 로그조차 없음.)
→ 수정(권장): plan / eval / solid / security 리포트는 `docs/harness/<slug>/`에 **커밋**한다 — 리뷰 기록으로서 가치가 있고, 브랜치를 따라 main으로 병합된다. handoff는 일시적이므로 `$(git rev-parse --git-common-dir)/../.claude/harness/`(메인 체크아웃 공용 위치)에 쓰고, `session-handoff.sh`도 같은 경로를 읽는다.

**F8. handoff 만료 없음** — `session-handoff.sh`는 `ls -t handoff-*.md | head -1`로 항상 최신 1개를 주입. 완료된 작업의 handoff가 이후 모든 세션에 영구 주입된다.
→ 수정: `/handoff`가 파일 머리에 `commit: <sha>`를 기록하고, 훅은 그 sha가 `HEAD`의 조상이며 이후 커밋이 N개 이상이면(=이미 소화됨) 건너뛴다. 보조로 7일 초과 파일은 무시.

**F9. WorktreeRemove 자동 병합의 두 가지 미검증** — `hooks/worktree-premerge.sh`
- (a) `{"continue": false}`로 제거를 차단한다고 가정하지만, 이 이벤트의 차단 semantics는 문서화돼 있지 않고 이 머신에서 훅이 실행된 로그도 없다. 차단이 안 되면 rebase 충돌 시 **미커밋 변경이 워크트리와 함께 소실**될 수 있다.
- (b) 테스트·clippy 없이 `main`을 fast-forward한다. CI가 push에서 잡긴 하지만 로컬 `main`은 깨진 상태가 될 수 있다. timeout 120s.
→ 수정: 먼저 검증 — 던지기용 워크트리에서 충돌 커밋을 만들고 제거를 트리거해 브랜치·워크트리가 남는지 확인(아래 Phase 2 레시피). 이후 병합 전 게이트 추가: `cargo fmt --all -- --check` + `cargo clippy --all-targets --all-features -- -D warnings` (+ 선택 `cargo test`), timeout 600, 실패 시 차단. 병합 자체를 유지할지는 결정 D1.

**F10. 살아있는 진행 상태 아티팩트 부재** — 원문 하네스 설계의 중심은 세션을 넘어 유지되는 feature list인데, 여기엔 없다. `docs/plans/2026-06-08-prd-v1-oauth2-oidc-roadmap.md`의 "Major PRD gaps"는 `/oauth/revoke`, `/oauth/introspect`, `/userinfo`를 미구현으로 표기하지만 라우트가 존재한다(`src/routes/oauth.rs:693`, `:721`, `src/routes/oidc.rs:95`).
→ 수정: `docs/plans/status.md` 하나를 체크리스트(항목 · 상태 · 배선 여부 · 근거 테스트)로 두고, `/feature`의 technical-writer 단계와 `/handoff`가 이 파일을 갱신·참조한다. 기존 로드맵 문서는 이 파일로 포인터를 남기고 동결.

### P2

**F11. 모델·예산 미지정** — 모든 에이전트가 `model: inherit`(기본), `maxTurns`·`effort` 없음. `/feature`는 에이전트 6개를 돌리는데 모델이 자동 호출할 수 있다.
→ 수정: evaluator·security-auditor·architect `inherit` + `effort: high` + `maxTurns`(예: 80); technical-writer `model: sonnet`; `/handoff` 커맨드 `model: sonnet`; `/feature`에 `disable-model-invocation: true`(사용자 명시 호출만).

**F12. `skills:` 프리로드 미사용** — 에이전트가 "use the `solid-rust` skill" 같은 텍스트 포인터로 스킬을 가리킨다.
→ 수정: evaluator `skills: [harness-rubric]`, solid-reviewer `skills: [solid-rust]`, architect `skills: [solid-rust]`. 본문의 "use the X skill" 문장 삭제.

**F13. 평가자 보정(calibration) 없음** — 원문이 강조하는 "평가자를 회의적으로 튜닝"은 측정 없이는 불가능하다. 지금은 evaluator가 결함을 잡는지 확인할 수단이 없다.
→ 수정: `.claude/harness-fixtures/` 에 결함 패치 3개와 기대 판정을 둔다.
- `liskov-inmemory-skips-reuse.patch` — `RefreshTokenRepository::InMemory`에서 reuse-detection 생략 → 기준 6 FAIL(High)
- `unwired-repo.patch` — 새 리포지토리 구현+테스트, `main.rs` 미배선 → 기준 3 FAIL
- `migration-without-schema-test.patch` — 컬럼 rename, `*_schema_migration.rs` 미갱신 → 기준 4 FAIL
`scripts/harness-calibrate.sh <fixture>`가 임시 워크트리에 패치를 적용하면 `/qa`를 돌려 기대 판정과 비교. 모델·프롬프트를 바꿀 때마다 3/3 검출을 확인.

**F14. 평가자가 실행 바이너리를 못 본다** — 기준 3 "wiring reality"를 `main.rs` 읽기로만 판정. 원문의 평가자는 실제 앱을 조작한다.
→ 수정(코드 측): `App` 구성을 `src/app.rs`의 `configure_app(cfg: &mut ServiceConfig, deps: AppDeps)` 같은 함수로 추출해 `main.rs`는 의존성 생성만 담당. 그러면 "main이 무엇을 배선하는가"를 DB 없이 테스트로 고정할 수 있다(`tests/app_wiring.rs`). 선택: `scripts/smoke.sh`(docker compose + curl `/health/ready`, register/login/refresh)를 Docker가 있을 때 evaluator가 실행.

**F15. 프롬프트 문체** — 부정문("do not", "never", "don't be surprised")과 no-op 문장("Be precise, secure, and conventional", "You know this codebase's conventions cold")이 많다. 금지문은 금지 대상을 활성화하고, no-op은 토큰만 쓴다.
→ 수정: F5 축소 작업과 함께 긍정 목표 진술로 다시 쓴다. 이미 좋은 leading word들("skeptical", "default to FAIL", "wiring reality", "contract-style finding")은 유지·반복.

**F16. 잔재 정리** — `.claude/README.md`·매처의 `MultiEdit`, developer/evaluator의 "click through"(브라우저 앱 하네스 표현), `.claude/README.md` 훅 목록에 WorktreeRemove 누락.

---

## 실행 계획

각 Phase는 독립 브랜치(워크트리)로 진행하고 완료 조건을 실측한다.

### Phase 0 — 지금 깨진 것 고치기 (반나절)
F1, F2, F3, F16.
- 완료 조건: `.rs` 파일을 Edit한 직후 `cargo fmt --all -- --check` clean · `/qa` 실행 시 `Agent`로 evaluator 스폰 성공 · `/qa` 리포트에 기준 8개 전부 등장.

### Phase 1 — 단일 진실 원천화 (1일)
F5, F6, F12, F11, F15.
- 완료 조건: agents+commands+skills 합계 ≤ 500줄 · 코드베이스 사실이 CLAUDE.md 한 곳에만 존재(grep으로 "SHA-256 hex hash" 같은 문구가 CLAUDE.md·README 외 0건) · 각 에이전트 frontmatter에 `skills`/`model`/`maxTurns` 명시.

### Phase 2 — 아티팩트·워크트리·훅 정합 (1일)
F4, F7, F8, F9.
- F9 검증 레시피: `git worktree add .claude/worktrees/hooktest -b hooktest` → 워크트리에서 `README.md` 첫 줄 수정·커밋 → `main`에서 같은 줄을 다르게 수정·커밋 → `EnterWorktree path=…hooktest` 후 `ExitWorktree action=remove` → 기대: 제거 차단 메시지, `hooktest` 브랜치와 디렉터리 잔존, `worktree-premerge.log`에 `BLOCK`. 차단되지 않으면 훅의 `block()`을 "병합 포기 + 경고 + 브랜치 보존 확인"으로 재설계.
- 완료 조건: 워크트리에서 `/handoff` → 메인 체크아웃 새 세션 시작 시 노출 · 소화된 handoff는 주입되지 않음 · fmt 실패 상태에서 `git commit` 차단 · premerge 게이트 실패 시 `main` 불변.

### Phase 3 — 보정·관측 (1~2일)
F13, F10, F14.
- 완료 조건: fixture 3/3에서 `/qa`가 FAIL + 올바른 기준 번호 · `docs/plans/status.md`가 현재 배선 상태와 일치(`main.rs` 대조) · `tests/app_wiring.rs`가 배선된 리포지토리·라우트 집합을 고정.

---

## 결정이 필요한 항목

| ID | 결정 | 권장 |
|----|------|------|
| D1 | WorktreeRemove 시 `main` 자동 병합을 유지할지, 유지하면 게이트 강도(fmt+clippy vs 전체 test) | 유지 + fmt+clippy 게이트(timeout 600). 전체 test는 CI에 위임 |
| D2 | 하네스 아티팩트 위치 — git 추적(`docs/harness/`) vs 공용 gitignored 디렉터리 | plan/eval/security는 추적, handoff만 공용 디렉터리 |
| D3 | 모델 배정 — evaluator/security-auditor를 세션 모델 상속으로 둘지, 별도 고정할지 | 상속 + `effort: high`; writer/handoff만 `sonnet` |
| D4 | F14의 `configure_app` 추출을 하네스 작업에 포함할지(코드 변경) | 포함 — 기준 3을 테스트 가능하게 만드는 유일한 방법 |

## 측정 지표

- Edit 후 `cargo fmt --check` 통과율: 현재 사실상 0%(35/63 파일 대상) → 100%
- 하네스 문서 총 줄 수: 752 → ≤ 500
- calibration fixture 검출률: 측정 불가 → 3/3
- 배선 상태 진실 원천 수: 8 → 1 (CLAUDE.md) + 테스트 1
