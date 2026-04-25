#!/bin/bash

# ==========================================
# CONSTANTS & COLORS
# ==========================================
BOLD="\033[1m"
RESET="\033[0m"
RED="\033[31m"
YELLOW="\033[33m"
GREEN="\033[32m"

# ==========================================
# DEPENDENCY CHECKS
# ==========================================
check_dependencies() {
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
}

# ==========================================
# UI COMPONENTS
# ==========================================
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
    local sched_name="$2"
    local box_width=40

    local line1=" $test_name"
    local line2=" SCHEDULER: "
    local sched_len=${#sched_name}

    local pad1=$(( box_width - ${#line1} ))
    local pad2=$(( box_width - ${#line2} - sched_len ))

    echo "┌───────────────────INFO───────────────────┐"
    printf "│%s%*s  │\n" "$line1" "$pad1" ""
    printf "│%s${BOLD}%s${RESET}%*s  │\n" "$line2" "$sched_name" "$pad2" ""
    echo "└──────────────────────────────────────────┘"
}
