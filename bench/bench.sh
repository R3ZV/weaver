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

if [ ! -x "./schbench/schbench" ]; then
    echo -e "${BOLD}${RED}[ERROR]${RESET}: schbench is not executable or doesn't exist."
    echo -e "${BOLD}Make sure pulled the submodules${RESET}: git submodule update --init --recursive"
    echo -e "${BOLD}Then build schbench${RESET}: cd schbench && make"
    exit 1
fi

if ! command -v hackbench &> /dev/null; then
    echo -e "${BOLD}${RED}[ERROR]${RESET}: Couldn't find hackbench."
    echo -e "${BOLD}Install package${RESET}: rt-tests"
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
show_spinner() {
    local pid=$1
    local delay=0.1
    local spinstr='|/-\'

    tput civis

    while kill -0 "$pid" 2>/dev/null; do
        local temp=${spinstr#?}
        printf "\r${BOLD}Running...${RESET} [ ${GREEN}%c${RESET} ]  " "$spinstr"
        local spinstr=$temp${spinstr%"$temp"}
        sleep $delay
    done

    printf "\r\033[K"
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

    echo "┌───────────────────INFO───────────────────┐"
    printf "│%s%*s  │\n" "$line1" "$pad1" ""
    printf "│%s${BOLD}%s${RESET}%*s  │\n" "$line2" "$sched" "$pad2" ""
    echo "└──────────────────────────────────────────┘"
}

clear
rm -rf results || true
mkdir -p results/latencies
mkdir -p results/hackbench

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

SCHEDULERS=("default" "weaver")
for sched in "${SCHEDULERS[@]}"; do
    if [ "$sched" != "default" ]; then
        sudo ../target/release/$sched 2> results/weaver_logs.txt &
        SCHED_PID=$!
        sleep 2
    fi

    print_test_banner "[Test 1]: Running schbench for ${RUNTIME}s..."
    echo "┌─────────────────────SCHBENCH─────────────────────┐"
    echo "│ CPU Cores: $CORES | M-Threads: $M_THREADS | W-Threads: $W_THREADS      │"
    echo "└──────────────────────────────────────────────────┘"

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
    echo ""

    print_test_banner "[Test 2]: Running hackbench..."
    HB_DATASIZE=512
    HB_LOOPS=2000
    HB_GROUPS=15
    HB_FDS=25
    HB_MODE="process"

    echo "┌──────────────────HACKBENCH───────────────────┐"
    printf "│ Datasize: %-4s | Loops: %-5s | Groups: %-4s │\n" "$HB_DATASIZE" "$HB_LOOPS" "$HB_GROUPS"
    printf "│ FDs/Group: %-3s | Mode: %-21s │\n" "$HB_FDS" "$HB_MODE"
    echo "└──────────────────────────────────────────────┘"

    hackbench --datasize $HB_DATASIZE \
              --loops $HB_LOOPS \
              --groups $HB_GROUPS \
              --fds $HB_FDS \
              --$HB_MODE > "results/hackbench/${sched}_hackbench.log" 2>&1 &
    HACK_PID=$!
    show_spinner $HACK_PID
    wait $HACK_PID

    # Detach scheduler
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
