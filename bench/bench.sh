#!/bin/bash

BOLD="\033[1m"
RESET="\033[0m"
RED="\033[31m"
YELLOW="\033[33m"
GREEN="\033[32m"

RUNTIME=30

if [ "$EUID" -ne 0 ]; then
    echo -e "${BOLD}${RED}[ERROR]${RESET}: Need sudo to load the eBPF scheduler."
    echo -e "${BOLD}Use${RESET}: sudo $0"
    exit 1
fi


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

show_progress() {
    local duration=$1
    local pid=$2
    local elapsed=0
    local bar_length=40

    tput civis

    while [ $elapsed -lt $duration ] && kill -0 $pid 2>/dev/null; do
        local percent=$(( (elapsed * 100) / duration ))
        local filled=$(( (elapsed * bar_length) / duration ))
        local empty=$(( bar_length - filled ))

        local bar_spaces=$(printf "%${filled}s" "")
        local empty_spaces=$(printf "%${empty}s" "")
        local bar="${bar_spaces// /█}"
        local space="${empty_spaces// /░}"

        printf "\r${BOLD}Progress:${RESET} [ ${GREEN}%s%s${RESET} ] %3d%% (%ds / %ds)" "$bar" "$space" "$percent" "$elapsed" "$duration"

        sleep 1
        elapsed=$(( elapsed + 1 ))
    done

    local full_spaces=$(printf "%${bar_length}s" "")
    local full_bar="${full_spaces// /█}"
    printf "\r${BOLD}Progress:${RESET} [${GREEN}%s${RESET}] 100%% (%ds / %ds)\n" "$full_bar" "$duration" "$duration"

    tput cnorm 
}

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

clear
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
CORES=$(nproc)

M_THREADS=$(( CORES / 4 ))
if [ "$M_THREADS" -lt 2 ]; then M_THREADS=2; fi

W_THREADS=$(( CORES * 2 ))

echo "┌────────────────────INFO────────────────────┐"
echo "│ CPU Cores: $CORES | M-Threads: $M_THREADS | W-Threads: $W_THREADS │"
echo "└────────────────────────────────────────────┘"

SCHEDULERS=("default" "weaver")
for sched in "${SCHEDULERS[@]}"; do
    if [ "$sched" != "default" ]; then
        sudo ../target/release/$sched 2> results/weaver_logs.txt &
        SCHED_PID=$!
        sleep 2
    fi

    print_test_banner "[Test 1]: Running schbench for ${RUNTIME}s..."
    echo "Percentile,Latency" > results/latencies/${sched}-${RUNTIME}-${M_THREADS}-${W_THREADS}_schbench.csv
    ./schbench/schbench --message-threads $M_THREADS --threads $W_THREADS --runtime $RUNTIME 2>&1 | \
    awk '{
        gsub(/\*/, "");
        if ($1 ~ /th:/) {
            gsub(/th:/, "", $1);
            print $1","$2 
        }
    }' >> results/latencies/${sched}-${RUNTIME}-${M_THREADS}-${W_THREADS}_schbench.csv &

    SCHBENCH_PID=$!
    show_progress $RUNTIME $SCHBENCH_PID
    wait $SCHBENCH_PID

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
