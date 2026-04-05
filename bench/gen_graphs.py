import pandas as pd
import seaborn as sb
import matplotlib.pyplot as plt
import glob
import os


def extract_metadata(filename, test_suffix):
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

        plt.figure(figsize=(10, 6))

        sb.barplot(
            x="Percentile",
            y="Latency",
            hue="Scheduler",
            data=all_data,
            errorbar=None,
        )

        plt.title(
            f"Wakeup Latency Distribution (schbench)\n{graph_subtitle}", fontsize=12
        )
        plt.ylabel("Latency (ms)")
        plt.xlabel("Percentile")

        plt.gca().set_axisbelow(True)
        plt.grid(True, linestyle="--", alpha=0.7)

        plt.savefig("graphs/schbench_latencies.pdf")
        print("[INFO] Created graphs/schbench_latencies.pdf")
    else:
        print("[WARN] No latency CSVs found in results/latencies/")


def main():
    os.makedirs("graphs", exist_ok=True)

    print("Generating...")
    plot_latencies()
    print("Done!")


if __name__ == "__main__":
    main()
