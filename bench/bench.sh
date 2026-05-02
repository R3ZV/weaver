#!/bin/bash

source ./utils/style.sh
source ./utils/test_schbench.sh
source ./utils/test_hackbench.sh
source ./utils/test_vkmark.sh

check_dependencies

RUNTIME=30

while getopts "t:h" opt; do
  case $opt in
    t) RUNTIME="$OPTARG" ;;
    h)
      echo -e "${BOLD}Usage:${RESET} sudo $0 [-t seconds]"
      echo "  -t    Set schbench runtime in seconds (default: 30)"
      exit 0 
      ;;
    \?) echo -e "${RED}Invalid option. Use -h for help.${RESET}"; exit 1 ;;
  esac
done

clear
rm -rf results || true
mkdir -p results/latencies
mkdir -p results/hackbench
mkdir -p results/vkmark

cat << "EOF"

    ███████╗ ██████╗██╗  ██╗    ██████╗ ███████╗███╗   ██╗ ██████╗██╗  ██╗
    ██╔════╝██╔════╝╚██╗██╔╝    ██╔══██╗██╔════╝████╗  ██║██╔════╝██║  ██║
    ███████╗██║      ╚███╔╝     ██████╔╝█████╗  ██╔██╗ ██║██║     ███████║
    ╚════██║██║      ██╔██╗     ██╔══██╗██╔══╝  ██║╚██╗██║██║     ██╔══██║
    ███████║╚██████╗██╔╝ ██╗    ██████╔╝███████╗██║ ╚████║╚██████╗██║  ██║
    ╚══════╝ ╚═════╝╚═╝  ╚═╝    ╚═════╝ ╚══════╝╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝

EOF

CORES=$(nproc)
M_THREADS=$(( CORES / 4 ))
if [ "$M_THREADS" -lt 2 ]; then M_THREADS=2; fi
W_THREADS=$(( CORES * 2 ))

# SCHEDULERS=("default" "weaver" "scx_lavd")
SCHEDULERS=("scx_lavd")
for sched in "${SCHEDULERS[@]}"; do
    if [ "$sched" == "weaver" ]; then
        ../target/release/$sched 2> results/weaver_logs.txt &
        SCHED_PID=$!
        sleep 2
    elif [ "$sched" == "scx_lavd" ]; then
        scx_lavd 2> results/scx_lavd_logs.txt &
        SCHED_PID=$!
    sleep 2
fi

    run_schbench "$sched" "$RUNTIME" "$M_THREADS" "$W_THREADS" "$CORES"
    run_hackbench "$sched"
    run_vkmark "$sched" "$RUNTIME"

    if [ "$sched" != "default" ]; then
        kill -SIGINT $SCHED_PID
        sleep 2
    fi
done

if [ -n "$SUDO_USER" ]; then
    chown -R "$SUDO_USER:$SUDO_USER" results
fi

cat << "EOF"

    ██████╗  ██████╗ ███╗   ██╗███████╗
    ██╔══██╗██╔═══██╗████╗  ██║██╔════╝
    ██║  ██║██║   ██║██╔██╗ ██║█████╗
    ██║  ██║██║   ██║██║╚██╗██║██╔══╝
    ██████╔╝╚██████╔╝██║ ╚████║███████╗
    ╚═════╝  ╚═════╝ ╚═╝  ╚═══╝╚══════╝

EOF
