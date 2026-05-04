#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "The Kani expected-failure lane has been retired; running required verification checks."
exec "$script_dir/kani.sh"
