#!/usr/bin/env bash
# Populate a running herdr server with a few colored workspaces and tabs to
# demo the --color flag. Start herdr first (e.g. `./target/debug/herdr`) in
# another terminal, then run this script. Each command prints the JSON
# response so you can see `color` round-trip through the API.

set -euo pipefail

HERDR="${HERDR:-./target/debug/herdr}"

if ! "$HERDR" status 2>/dev/null | grep -q "status: running"; then
    cat >&2 <<EOF
herdr server is not running. Start it first in another terminal, e.g.:

    $HERDR

Then re-run this script.
EOF
    exit 1
fi

run() {
    echo "$ $*"
    "$@"
    echo
}

echo "=== Creating colored workspaces ==="
run "$HERDR" workspace create --cwd /tmp --label "api"     --color "#89b4fa" --focus
run "$HERDR" workspace create --cwd /tmp --label "web"     --color "orange"
run "$HERDR" workspace create --cwd /tmp --label "infra"   --color "green"
run "$HERDR" workspace create --cwd /tmp --label "scratch" --color "pink"

echo "=== Adding colored tabs in workspace 1 (api) ==="
run "$HERDR" tab create --workspace 1 --label "server"  --color "blue"
run "$HERDR" tab create --workspace 1 --label "tests"   --color "yellow"
run "$HERDR" tab create --workspace 1 --label "logs"    --color "red"

echo "=== Recolor without renaming ==="
run "$HERDR" workspace rename 2 --color "#ff79c6"

echo "=== Final state ==="
run "$HERDR" workspace list
run "$HERDR" tab list --workspace 1

cat <<EOF
Done. Attach with:

    $HERDR

Try toggling render targets in ~/.config/herdr-dev/config.toml:

    [ui.entity_color]
    tab_label = true
    tab_background = true
    status_accent = true
    pane_border = false

Then \`prefix + shift + r\` (default reload-config binding) to re-apply.

Clean up demo state:

    for i in 4 3 2 1; do $HERDR workspace close \$i; done
EOF
