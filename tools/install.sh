#!/usr/bin/env bash

#################################################
#### Ensure we are in the right path. ###########
#################################################
if [[ 0 -eq $(echo $0 | grep -c '^/') ]]; then
    # relative path
    EXEC_PATH=$(dirname "`pwd`/$0")
else
    # absolute path
    EXEC_PATH=$(dirname "$0")
fi

EXEC_PATH=$(echo ${EXEC_PATH} | sed 's@/\./@/@g' | sed 's@/\.*$@@')
cd $EXEC_PATH || exit 1
#################################################

#
# install `btm daemon` as a systemd service
#     - replace all UPPER words in btm-daemon.service to their actual instances
#     - copy `btm-daemon.service` to a right path
#     - copy `btm` binary to '/usr/local/bin/'
#     - enable `btm-daemon.service`
#
# example:
#
# ```
# install.sh \
#         --snapshot-itv=4 \
#         --snapshot-cap=100 \
#         --snapshot-mode=zfs \
#         --snapshot-algo=fair \
#         --snapshot-volume=zfs/blockchain
# ```
#

usage() {
    echo -e "\033[31;01mUsage\033[0m"
    echo
    echo -e "\tinstall.sh"
    echo -e "\t\t--snapshot-itv=<ITV>"
    echo -e "\t\t--snapshot-cap=<CAP>"
    echo -e "\t\t--snapshot-mode=<MODE>"
    echo -e "\t\t--snapshot-algo=<ALGO>"
    echo -e "\t\t--snapshot-volume=<VOLUME>"

    echo

    echo -e "\033[31;01mExample\033[0m"
    echo
    echo -e "\tinstall.sh \\"
    echo -e "\t\t--snapshot-itv=4 \\"
    echo -e "\t\t--snapshot-cap=100 \\"
    echo -e "\t\t--snapshot-mode=zfs \\"
    echo -e "\t\t--snapshot-algo=fair \\"
    echo -e "\t\t--snapshot-volume=zfs/blockchain"

    echo

    echo -e "\033[31;01mExample, short style\033[0m"
    echo
    echo -e "\tinstall.sh -i=4 -c=100 -m=zfs -a=fair -p=zfs/blockchain"
    echo -e "\tinstall.sh -i=4 -c=100 -m=btrfs -a=fair -p=/data/blockchain"
    echo
}

for i in "$@"; do
    case $i in
        -i=*|--snapshot-itv=*)
            ITV="${i#*=}"
            ;;
        -c=*|--snapshot-cap=*)
            CAP="${i#*=}"
            ;;
        -m=*|--snapshot-mode=*)
            MODE="${i#*=}"
            ;;
        -a=*|--snapshot-algo=*)
            ALGO="${i#*=}"
            ;;
        -p=*|--snapshot-volume=*)
            VOLUME="${i#*=}"
            ;;
        *)
            usage
            exit 1
            ;;
    esac
done

if [[ "" == $ITV ]]; then
    ITV=10
fi

expr $ITV + 0 2>/dev/null 1>&2
if [[ 0 -ne $? ]]; then
    echo -e "\n\tthe value of \033[31;01m--snapshot-itv\033[0m is NOT a valid number !!\n"
    exit 1
fi

if [[ "" == $CAP ]]; then
    CAP=100
fi

expr $CAP + 0 2>/dev/null 1>&2
if [[ 0 -ne $? ]]; then
    echo -e "\n\tthe value of \033[31;01m--snapshot-cap\033[0m is NOT a valid number !!\n"
    exit 1
fi

if [[ "" == $MODE ]]; then
    echo -e "\n\t\033[31;01m--snapshot-mode\033[0m is missing !!\n"
    exit 1
fi

if [[ "zfs" != $MODE && "btrfs" != $MODE ]]; then
    echo -e "\n\tthe value of \033[31;01m--snapshot-mode\033[0m can only be 'zfs' or 'btrfs' !!\n"
    exit 1
fi

if [[ "" == $ALGO ]]; then
    ALGO=fair
fi

if [[ "fair" != $ALGO && "fade" != $ALGO ]]; then
    echo -e "\n\tthe value of \033[31;01m--snapshot-algo\033[0m can only be 'fair' or 'fade' !!\n"
    exit 1
fi

if [[ "" == $VOLUME ]]; then
    echo -e "\n\t\033[31;01m--snapshot-volume\033[0m is missing !!\n"
    exit 1
fi

cp -f btm-daemon.service x.service
sed -i "s#ITV#${ITV}#g" x.service
sed -i "s#CAP#${CAP}#g" x.service
sed -i "s#MODE#${MODE}#g" x.service
sed -i "s#ALGO#${ALGO}#g" x.service
sed -i "s#VOLUME#${VOLUME}#g" x.service

target_path=$(dirname $(systemctl cat network.target | grep -o '#.*/network.target' | sed -r 's/^\s*#\s+//g'))
cp -f x.service ${target_path}/btm-daemon.service || exit 1

cp -f btm /usr/local/bin/ || exit 1

systemctl disable btm-daemon.service
systemctl enable btm-daemon.service
systemctl restart btm-daemon.service
systemctl status btm-daemon.service
