#!/usr/bin/env bash
set -euo pipefail

cargo +stable llvm-cov \
  --workspace \
  --summary-only \
  --fail-under-lines 90 \
  --ignore-filename-regex '(^|/)(main|config|db|error)\.rs$|src/(auth|oauth)/.*_repository\.rs|src/routes/(auth|oauth|oidc|health)\.rs|src/oauth/authorization_code\.rs'
