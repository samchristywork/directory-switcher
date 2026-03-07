#!/usr/bin/env bash
set -e

cd "$(dirname "$0")/.."

echo "Building release binary..."
cargo build --release

shell="$(basename "$SHELL")"
case "$shell" in
    bash)
        bash scripts/add-alias-bash.sh
        echo "Reload your shell or run: source ~/.bashrc"
        ;;
    fish)
        fish scripts/add-alias-fish.fish
        echo "Reload your shell or run: source ~/.config/fish/config.fish"
        ;;
    *)
        echo "Unknown shell '$shell'. Add the alias manually:"
        binary="$(pwd)/target/release/directory-switcher"
        echo "  alias ds='${binary} && cd \"\$(cat /tmp/directory-switcher-\$\$ 2>/dev/null)\" ; rm -f /tmp/directory-switcher-\$\$'"
        ;;
esac
