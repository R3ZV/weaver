#!/bin/bash

run_vkmark() {
    local sched=$1
    local runtime=$2

    print_test_banner "[Test 3]: Running vkmark..." "$sched"

    export MANGOHUD_CONFIG="autostart_log=1,output_folder=$(pwd)/results/vkmark,no_display"

    sudo -u "$SUDO_USER" mangohud vkmark > "results/vkmark/${sched}.log" 2>&1 &
    local VKMARK_PID=$!
    sleep 1

    stress-ng --cpu 0 --timeout "$runtime" --quiet &
    local STRESS_PID=$!

    show_progress "$runtime" "$STRESS_PID"
    wait "$STRESS_PID"
    kill -SIGTERM "$VKMARK_PID" 2>/dev/null

    local max_wait=3
    local waited=0
    while kill -0 "$VKMARK_PID" 2>/dev/null && [ $waited -lt $max_wait ]; do
        sleep 1
        ((waited++))
    done
    kill -9 "$VKMARK_PID" 2>/dev/null

    sleep 1
    local LATEST_CSV=$(ls -t $(pwd)/results/vkmark/*.log 2>/dev/null | head -n 1)

    if [ -n "$LATEST_CSV" ]; then
        mv "$LATEST_CSV" "$(pwd)/results/vkmark/${sched}.csv"
    fi
    echo ""
}
