#!/usr/bin/env fish

set binary (realpath (dirname (status filename))/../target/release/directory-switcher)
set config_file ~/.config/fish/config.fish
set alias_line "alias ds '$binary && cd (cat /tmp/directory-switcher-\$fish_pid 2>/dev/null)'"

if grep -qF "alias ds" $config_file 2>/dev/null
    echo "ds alias already present in $config_file"
else
    echo "" >> $config_file
    echo $alias_line >> $config_file
    echo "Added ds alias to $config_file"
end
