#!/bin/bash

run_hackbench() {
    local sched=$1

    local HB_DATASIZE=512
    local HB_LOOPS=2000
    local HB_GROUPS=15
    local HB_FDS=25
    local HB_MODE="process"

    print_test_banner "[Test 2]: Running hackbench..." "$sched"

    echo "┌──────────────────HACKBENCH───────────────────┐"
    printf "│ Datasize: %-4s | Loops: %-5s | Groups: %-4s │\n" "$HB_DATASIZE" "$HB_LOOPS" "$HB_GROUPS"
    printf "│ FDs: %-3s | Mode: %-21s │\n" "$HB_FDS" "$HB_MODE"
    echo "└──────────────────────────────────────────────┘"

    hackbench --datasize $HB_DATASIZE \
              --loops $HB_LOOPS \
              --groups $HB_GROUPS \
              --fds $HB_FDS \
              --$HB_MODE > "results/hackbench/${sched}_hackbench.log" 2>&1 &

    local HACK_PID=$!
    show_spinner "$HACK_PID"
    wait "$HACK_PID"
    echo ""
}
