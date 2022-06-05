#!/bin/sh
/usr/local/bin/moose2 import \
    -f ~moose/.local/var/dump.json \
    -o /var/lib/moose2/moose.json || exit
systemctl reload moose2
