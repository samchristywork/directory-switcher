#!/usr/bin/env bash

binary="$(readlink -f "$(dirname "$0")/../target/release/directory-switcher")"
config_file=~/.bashrc

read -r -d '' func_text <<EOF

ds() {
    local out="/tmp/directory-switcher-\$\$"
    "${binary}"
    if [ -f "\$out" ]; then
        local target
        target="\$(cat "\$out")"
        rm -f "\$out"
        [ -n "\$target" ] && cd "\$target"
    fi
}
EOF

if grep -qF "function ds\|^ds()" "$config_file" 2>/dev/null || grep -qF "alias ds" "$config_file" 2>/dev/null; then
    echo "ds function/alias already present in $config_file"
else
    printf '%s\n' "$func_text" >> "$config_file"
    echo "Added ds function to $config_file"
fi
