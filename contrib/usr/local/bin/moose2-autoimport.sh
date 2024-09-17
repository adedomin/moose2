#!/bin/bash
trap 'rm -f -- "$tmp"' EXIT
tmp="$(mktemp)" || exit
# $TRIGGER_PATH is set when this service is invoked by a path unit trigger.
/usr/local/bin/moose2 --config "${MOOSE2_CONFIG?}" \
    convert \
        "${TRIGGER_PATH-/home/moose/.local/var/dump.json}" \
        "$tmp" || exit
/usr/local/bin/moose2 --config "${MOOSE2_CONFIG?}" \
    import --merge "$tmp" || exit
# TODO: not necessary, for now.
# systemctl reload moose2
