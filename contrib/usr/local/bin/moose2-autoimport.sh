#!/bin/sh
# $TRIGGER_PATH is set when this service is invoked by a path unit trigger.
/usr/local/bin/moose2 \
    --config "${MOOSE2_CONFIG?}" \
    import --ignore "${TRIGGER_PATH-/home/moose/.local/var/dump.json}"
