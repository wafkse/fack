format := "human"

[no-exit-message]
check:
    #!/usr/bin/env bash
    set -euo pipefail

    message_format() {
        case "$1" in
            human|json) printf '%s' "--message-format=$1" ;;
            *) printf 'unsupported message format `%s`\n' "$1" >&2; return 2 ;;
        esac
    }

    flag="$(message_format '{{format}}')"
    cargo clippy --workspace --all-targets "$flag"
    cargo dylint --all -- --all-targets "$flag"
