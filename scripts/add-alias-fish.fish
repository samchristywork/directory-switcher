#!/usr/bin/env fish

set binary (realpath (dirname (status filename))/../target/release/directory-switcher)
set config_file ~/.config/fish/config.fish

set func_text "
function ds
    set out /tmp/directory-switcher-\$fish_pid
    $binary
    if test -f \$out
        set target (cat \$out)
        rm -f \$out
        if test -n \"\$target\"
            cd \$target
        end
    end
end"

if grep -qF "function ds" $config_file 2>/dev/null
    echo "ds function already present in $config_file"
else
    echo $func_text >> $config_file
    echo "Added ds function to $config_file"
end
