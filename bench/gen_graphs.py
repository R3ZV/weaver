# pyright: reportUnknownMemberType=false
# pyright: reportUnknownVariableType=false
# pyright: reportUnknownArgumentType=false

from matplotlib.container import BarContainer
import pandas as pd
import seaborn as sb
import matplotlib.pyplot as plt
import glob
import os
import re
from dataclasses import dataclass


@dataclass
class PlotMetrics:
    scheduler: str
    time_s: float


@dataclass
class HackbenchMetrics:
    groups: int
    file_descriptors: int
    messages: int
    message_bytes: int
    time: float


def extract_metadata(filename: str, test_suffix: str):
    """
    In this case the metadata is part of the file name instead of the file
    content compared to hackbench.
    """
    base_name = os.path.basename(filename).replace(test_suffix, "")
    sched_name, runtime, m_threads, w_threads = base_name.rsplit("-", 3)

    subtitle = (
        f"(Runtime: {runtime}s | M-Threads: {m_threads} | W-Threads: {w_threads})"
    )
    return sched_name, subtitle


def plot_latencies():
    csv_files = glob.glob("results/latencies/*_schbench.csv")
    dataframes = []
    graph_subtitle = ""

    for file in csv_files:
        sched_name, subtitle = extract_metadata(file, "_schbench.csv")
        graph_subtitle = subtitle

        df = pd.read_csv(file)
        df["Scheduler"] = sched_name
        dataframes.append(df)

    if dataframes:
        all_data = pd.concat(dataframes)

        _ = plt.figure(figsize=(10, 6))

        _ = sb.barplot(
            x="Percentile",
            y="Latency",
            hue="Scheduler",
            data=all_data,
            errorbar=None,
        )

        _ = plt.title(
            f"Wakeup Latency Distribution (schbench)\n{graph_subtitle}", fontsize=12
        )
        _ = plt.ylabel("Latency (ms)")
        _ = plt.xlabel("Percentile")

        plt.gca().set_axisbelow(True)
        plt.grid(True, linestyle="--", alpha=0.7)

        plt.savefig("graphs/schbench_latencies.pdf")
        print("[INFO] Created graphs/schbench_latencies.pdf")
    else:
        print("[WARN] No latency CSVs found in results/latencies/")


def hackbench_metadata(file: str) -> HackbenchMetrics:
    """
    Format of hackbench log file:
    Running in process mode with 15 groups using 50 file descriptors each (== 750 tasks)
    Each sender will pass 2000 messages of 512 bytes
    Time: 15.43

    This funtion returns the metadata to be used as a title for plot title
    """

    with open(file, "r") as f:
        content = f.read()
        groups_match = re.search(r"with \s*([\d]+) groups", content)
        assert groups_match

        fd_match = re.search(r"using \s*([\d]+) file", content)
        assert fd_match

        messages_match = re.search(r"will pass \s*([\d]+) messages", content)
        assert messages_match

        message_len_match = re.search(r"of \s*([\d]+) bytes", content)
        assert message_len_match

        time_match = re.search(r"Time:\s*([\d\.]+)", content)
        assert time_match

        groups = int(groups_match.group(1))
        fd = int(fd_match.group(1))
        messages = int(messages_match.group(1))
        message_len = int(message_len_match.group(1))
        time = float(time_match.group(1))

        return HackbenchMetrics(groups, fd, messages, message_len, time)


def plot_context_switch_overhead():
    log_files = glob.glob("results/hackbench/*_hackbench.log")
    results: list[PlotMetrics] = []

    title = ""
    for file in log_files:
        sched_name = os.path.basename(file).replace("_hackbench.log", "")
        metadata = hackbench_metadata(file)
        curr_title = (
            "Groups={}, FileDescriptors={}, MessagesSent={}, MessageBytes={}".format(
                metadata.groups,
                metadata.file_descriptors,
                metadata.messages,
                metadata.message_bytes,
            )
        )
        if len(title) == 0:
            title = curr_title
        else:
            assert title == curr_title

        results.append(PlotMetrics(sched_name, metadata.time))

    if results:
        df = pd.DataFrame(results)
        _ = plt.figure(figsize=(8, 5))

        ax = sb.barplot(
            x="scheduler",
            y="time_s",
            data=df,
            hue="scheduler",
            legend=False,
            palette="viridis",
        )

        _ = plt.title(
            f"Context Switch Overhead (hackbench)\n{title}",
            fontsize=12,
        )
        _ = plt.xlabel("Scheduler")
        _ = plt.ylabel("Execution Time (Seconds)")
        _ = plt.grid(True, axis="y", linestyle="--", alpha=0.7)

        # Add exact values on top of bars
        for container in ax.containers:
            if isinstance(container, BarContainer):
                _ = ax.bar_label(container, fmt="%.3fs", padding=3)

        plt.savefig("graphs/hackbench_results.pdf")
        print("[INFO] Created graphs/hackbench_results.pdf")
        plt.close()
    else:
        print("[WARN] No valid hackbench logs found.")


def main():
    os.makedirs("graphs", exist_ok=True)

    print("Generating...")
    plot_latencies()
    plot_context_switch_overhead()
    print("Done!")


if __name__ == "__main__":
    main()
