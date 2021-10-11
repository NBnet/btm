#!/usr/bin/env bash

#
# install `btm daemon` as a systemd service
#     - replace all UPPER words in btm-daemon.service to their actual instances
#     - copy `btm` binary to '/usr/local/bin/'
#     - copy `btm-daemon.service` to a right path
#     - enable `btm-daemon.service`
#
# example:
#
# ```
# install.sh \
#         --snapshot-itv=4 \
#         --snapshot-cap=100 \
#         --snapshot-algo=fade \
#         --snapshot-target=zfs/data
# ```
#

