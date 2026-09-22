# Prérequis : cargo binstall (https://github.com/cargo-bins/cargo-binstall)
# Installation : cargo install cargo-binstall

RESULTS_DIR := "results"

install-tools:
    cargo binstall hyperfine samply --no-confirm

build:
    cargo build --release --bin bench

format:
    cargo fmt

lint:
    cargo clippy -- -D warnings

_mkdir-results:
    mkdir -p {{RESULTS_DIR}}

# Compare naive vs fast sur un scénario : just bench sc6
# SC6 (500k iters) : warmup 3, runs 10 — autres : warmup 10, runs 100
bench SC: build _mkdir-results
    #!/usr/bin/env bash
    set -e
    if [ "{{SC}}" = "sc6" ]; then warmup=3; runs=10; else warmup=10; runs=100; fi
    hyperfine \
      --warmup $warmup --runs $runs --shell=none \
      --command-name "naive {{SC}}" \
      --command-name "fast  {{SC}}" \
      --export-json {{RESULTS_DIR}}/{{SC}}.json \
      --export-markdown {{RESULTS_DIR}}/{{SC}}.md \
      './target/release/bench {{SC}} naive' \
      './target/release/bench {{SC}} fast'

# Compare naive vs fast sur l'ensemble SC1–SC6
bench-all: build _mkdir-results
    hyperfine \
      --warmup 3 --runs 10 --shell=none \
      --command-name "naive" \
      --command-name "fast" \
      --export-json {{RESULTS_DIR}}/all.json \
      --export-markdown {{RESULTS_DIR}}/all.md \
      './target/release/bench naive' \
      './target/release/bench fast'

# Flamegraph interactif via Firefox Profiler
profile: build
    samply record ./target/release/bench sc6 fast

# Extraction des métriques depuis un fichier JSON (usage : just stats results/sc6.json)
stats FILE:
    #!/usr/bin/env python3
    import json, statistics as s
    data = json.load(open("{{FILE}}"))["results"]
    for r in data:
        t = r["times"]
        print(f"[{r['command']}]")
        print("  Moyenne   : %.1f ms" % (s.mean(t)*1000))
        print("  Médiane   : %.1f ms" % (s.median(t)*1000))
        print("  Écart-type: %.1f ms" % (s.stdev(t)*1000))
        print("  Min/Max   : %.1f / %.1f ms" % (min(t)*1000, max(t)*1000))
