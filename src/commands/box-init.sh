#!/usr/bin/env bash

set -eu -o pipefail

if [[ -z "${BOX_USER}" ]]; then
    echo "Container initialization requires host user"
    exit 1
fi

HOME="/home/$BOX_USER"

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
usermod -d "$HOME" -s "${BOX_SHELL:-$shell}" "$BOX_USER"

echo "Setting up user home from /etc/skel"

if [[ ! -d "$HOME" ]]; then
    mkdir "$HOME"
fi

# make it owned by the user
chown "$BOX_USER:$BOX_USER" "$HOME"

# create all directories with same permissions and make them owned by the user
while IFS= read -r -d '' dir
do
    # skip cur dir, prev dir, and empty string for some reason..
    [[ "$dir" == "." || "$dir" == ".." || -z "$dir" ]] && continue

    perm="$(stat --format='%a' "/etc/skel/$dir")"
    mkdir --mode="$perm" "$HOME/$dir"
    chown "$BOX_USER:$BOX_USER" "$HOME/$dir"
done < <(cd /etc/skel && find . -type d -printf '%P\0')

# copy all the files (without overwriting) while keeping permissions and make
# them also owned by the user
while IFS= read -r -d '' file
do
    perm="$(stat --format='%a' "/etc/skel/$file")"
    cp --no-dereference --update=none --preserve "/etc/skel/$file" "$HOME/$file"

    # links require some special care
    if [[ -L "/etc/skel/$file" ]]; then
        # needs -h so it does not change the targeted file
        chown -h "$BOX_USER:$BOX_USER" "$HOME/$file"
    else
        chmod --reference="/etc/skel/$file" "$HOME/$file"
        chown "$BOX_USER:$BOX_USER" "$HOME/$file"
    fi
done < <(cd /etc/skel && find . \( -type f -o -type l \) -printf '%P\0')

# NOTE i preserved the old versions here which are less readable
# (cd /etc/skel && find . -type d -exec mkdir -p "/home/$BOX_USER/{}" \; -exec chown "$BOX_USER:$BOX_USER" "/home/$BOX_USER/{}" \;)
# (cd /etc/skel && find . -type f -exec cp --update=none --preserve "{}" "/home/$BOX_USER/{}" \; -exec chown "$BOX_USER:$BOX_USER" "/home/$BOX_USER/{}" \;)

# TODO is this even necessary?
[[ -d "$HOME/.ssh" ]] && chmod 0700 "$HOME/.ssh"

# only do it if there is sudo installed
if [[ -f /usr/bin/sudo ]]; then
    echo "Enabling rootless sudo for all"

    # disable hostname resolving
    echo 'Defaults !fqdn' >> /etc/sudoers

    # allow everything without a password
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

echo "Starting infinite loop (use SIGTERM to stop the container)"

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
