#!/bin/bash

run_schbench() {
    local sched=$1
    local runtime=$2
    local m_threads=$3
    local w_threads=$4
    local cores=$5

    print_test_banner "[Test 1]: Running schbench for ${runtime}s..." "$sched"

    echo "┌─────────────────────SCHBENCH─────────────────────┐"
    echo "│ CPU Cores: $cores | M-Threads: $m_threads | W-Threads: $w_threads       │"
    echo "└──────────────────────────────────────────────────┘"

    echo "Percentile,Latency" > "results/latencies/${sched}-${runtime}-${m_threads}-${w_threads}.csv"

    ./schbench/schbench --message-threads $m_threads --threads $w_threads --runtime $runtime 2>&1 | \
    awk '{
        gsub(/\*/, "");
        if ($1 ~ /th:/) {
            gsub(/th:/, "", $1);
            print $1","$2 
        }
    }' >> "results/latencies/${sched}-${runtime}-${m_threads}-${w_threads}.csv" &

    local SCHBENCH_PID=$!
    show_progress "$runtime" "$SCHBENCH_PID"
    wait "$SCHBENCH_PID"
    echo ""
}
