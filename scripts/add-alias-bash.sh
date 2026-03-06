#!/usr/bin/env bash

binary="$(readlink -f "$(dirname "$0")/../target/release/directory-switcher")"
config_file=~/.bashrc
alias_line="alias ds='${binary} && cd \"\$(cat /tmp/directory-switcher-\$\$ 2>/dev/null)\"'"

if grep -qF "alias ds" "$config_file" 2>/dev/null; then
    echo "ds alias already present in $config_file"
else
    echo "" >> "$config_file"
    echo "$alias_line" >> "$config_file"
    echo "Added ds alias to $config_file"
fi
