#!/usr/bin/env bash

set -eu -o pipefail

if [[ -z "${BOX_USER}" ]]; then
    echo "Container initialization requires host user"
    exit 1
fi

# you probably won't have fish and zsh installed and as bash is required, any
# other shell is considered as the default so
if [[ -f /bin/fish ]]; then
    shell=/bin/fish
elif [[ -f /bin/zsh ]]; then
    shell=/bin/zsh
else
    shell=/bin/bash
fi

echo "Setting the user home and shell"
usermod -d "/home/$BOX_USER" -s "${BOX_SHELL:-$shell}" "$BOX_USER"

echo "Setting up user home from /etc/skel"
/sbin/mkhomedir_helper "$BOX_USER"

# only do it if there is sudo installed
if [[ -f /usr/bin/sudo ]]; then
    echo "Enabling rootless sudo for all"
    echo 'ALL ALL = (ALL) NOPASSWD: ALL' >> /etc/sudoers
else
    # set root passwd just in case so you can use `su`
    echo "root:root" | passwd root
fi

# run user scripts
echo "Running /init.d/ scripts"
if [[ -d /init.d ]]; then
    for script in /init.d/*; do
        if [[ -x "$script" ]]; then
            # run each script as user
            sudo -u "$BOX_USER" "$script"
        fi
    done
fi

echo "Starting infinite loop (Ctrl + C to close)"

# make sure the container stays alive
sleep infinity &

# make container respond to being killed
on_sigterm() {
    echo Caught SIGTERM, exiting...
    jobs -p | xargs -r kill -TERM
    wait
}

trap "on_sigterm" TERM INT
wait
