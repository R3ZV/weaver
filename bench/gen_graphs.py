import pandas as pd
import seaborn as sb
import matplotlib.pyplot as plt
import glob
import os


def plot_latencies():
    csv_files = glob.glob("results/latencies/*_schbench.csv")
    dataframes = []

    for file in csv_files:
        sched_name = os.path.basename(file).replace("_schbench.csv", "")

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

        plt.title("Wakeup Latency Distribution (schbench)")
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
