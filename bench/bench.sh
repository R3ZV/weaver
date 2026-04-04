#!/bin/bash

BOLD="\033[1m"
RESET="\033[0m"
RED="\033[31m"

if [ "$EUID" -ne 0 ]; then
    echo -e "${BOLD}${RED}[ERROR]${RESET}: Need sudo to load the eBPF scheduler."
    echo -e "${BOLD}Use${RESET}: sudo $0"
    exit 1
fi

clear

print_test_banner() {
    local test_name="$1"
    local box_width=40

    local line1=" $test_name"
    local line2=" SCHEDULER: "
    local sched_len=${#sched}

    local pad1=$(( box_width - ${#line1} ))
    local pad2=$(( box_width - ${#line2} - sched_len ))

    echo "┌──────────────────INFO──────────────────┐"
    printf "│%s%*s│\n" "$line1" "$pad1" ""
    printf "│%s${BOLD}%s${RESET}%*s│\n" "$line2" "$sched" "$pad2" ""
    echo "└────────────────────────────────────────┘"
}

rm -rf results || true
mkdir -p results/latencies

cat << "EOF"

    ███████╗ ██████╗██╗  ██╗    ██████╗ ███████╗███╗   ██╗ ██████╗██╗  ██╗
    ██╔════╝██╔════╝╚██╗██╔╝    ██╔══██╗██╔════╝████╗  ██║██╔════╝██║  ██║
    ███████╗██║      ╚███╔╝     ██████╔╝█████╗  ██╔██╗ ██║██║     ███████║
    ╚════██║██║      ██╔██╗     ██╔══██╗██╔══╝  ██║╚██╗██║██║     ██╔══██║
    ███████║╚██████╗██╔╝ ██╗    ██████╔╝███████╗██║ ╚████║╚██████╗██║  ██║
    ╚══════╝ ╚═════╝╚═╝  ╚═╝    ╚═════╝ ╚══════╝╚═╝  ╚═══╝ ╚═════╝╚═╝  ╚═╝
 
EOF

SCHEDULERS=("default" "weaver")
for sched in "${SCHEDULERS[@]}"; do
    if [ "$sched" != "default" ]; then
        sudo ../target/release/$sched &
        SCHED_PID=$!
        sleep 2
    fi

    print_test_banner "[Test 1]: Running schbench for 30s..."
    echo "Percentile,Latency" > results/latencies/${sched}_schbench.csv
    ./schbench/schbench --message-threads 2 --threads 4 --runtime 30 2>&1 | \
    awk '{
        gsub(/\*/, "");
        if ($1 ~ /th:/) {
            gsub(/th:/, "", $1);
            print $1","$2 
        }
    }' >> results/latencies/${sched}_schbench.csv

    if [ "$sched" != "default" ]; then
        sudo kill -SIGINT $SCHED_PID
        sleep 2
    fi
done

cat << "EOF"

    ██████╗  ██████╗ ███╗   ██╗███████╗
    ██╔══██╗██╔═══██╗████╗  ██║██╔════╝
    ██║  ██║██║   ██║██╔██╗ ██║█████╗
    ██║  ██║██║   ██║██║╚██╗██║██╔══╝
    ██████╔╝╚██████╔╝██║ ╚████║███████╗
    ╚═════╝  ╚═════╝ ╚═╝  ╚═══╝╚══════╝

EOF
