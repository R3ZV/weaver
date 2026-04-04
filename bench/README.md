# Benchmarking

## Run locally

For Benchmarking we are going to use following tools to our aid:
- stress-ng
- mangohud
- glmark2
- schbench (submodule)
- uv
- awk
- git

To start and collect data use `bench.sh`.

After your benchmark is done you can run `uv run python3 gen_graphs.py`
to get a visual representation of you collected data.

All the graphs are located in `graphs` directory.

## What are we benchmarking?

`bench.sh` will run the following benchmarks:
- TODO: explain why and how is everything tested
