#!/usr/bin/env bash
set -euo pipefail

cargo test -p sour-verifier --test accounting_invariants

