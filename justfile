# Prérequis : cargo binstall (https://github.com/cargo-bins/cargo-binstall)
# Installation : cargo install cargo-binstall

install-tools:
    cargo binstall hyperfine samply --no-confirm

build:
    cargo build --release --bin bench

# SC1 — AA vs KK vs QJs | Flop 9♠-T♠-2♦ | 50 000 iters
bench-sc1: build
    hyperfine \
      --warmup 10 --runs 100 --shell=none \
      --export-json results_sc1.json \
      --export-markdown results_sc1.md \
      './target/release/bench sc1'

# SC2 — AhKh vs 9c9d vs JcTc | Turn | 75 000 iters
bench-sc2: build
    hyperfine \
      --warmup 10 --runs 100 --shell=none \
      --export-json results_sc2.json \
      --export-markdown results_sc2.md \
      './target/release/bench sc2'

# SC3 — 7s6s vs AdKd vs QcQh | Flop | 50 000 iters
bench-sc3: build
    hyperfine \
      --warmup 10 --runs 100 --shell=none \
      --export-json results_sc3.json \
      --export-markdown results_sc3.md \
      './target/release/bench sc3'

# SC4 — 4 joueurs dont 1 inconnu | Flop | 30 000 iters
bench-sc4: build
    hyperfine \
      --warmup 10 --runs 100 --shell=none \
      --export-json results_sc4.json \
      --export-markdown results_sc4.md \
      './target/release/bench sc4'

# SC5 — Ah5h vs JdJc vs 7c6c | Flop connecté | 50 000 iters
bench-sc5: build
    hyperfine \
      --warmup 10 --runs 100 --shell=none \
      --export-json results_sc5.json \
      --export-markdown results_sc5.md \
      './target/release/bench sc5'

# SC6 — AhKh vs QsQd | haute précision 500 000 iters (référence convergence)
bench-sc6: build
    hyperfine \
      --warmup 3 --runs 10 --shell=none \
      --export-json results_sc6.json \
      --export-markdown results_sc6.md \
      './target/release/bench sc6'

# Lance SC1–SC5 en comparatif (exclut SC6 par sa durée)
bench-all: build
    hyperfine \
      --warmup 10 --runs 100 --shell=none \
      --export-json results_all.json \
      --export-markdown results_all.md \
      './target/release/bench sc1' \
      './target/release/bench sc2' \
      './target/release/bench sc3' \
      './target/release/bench sc4' \
      './target/release/bench sc5'

# Baseline complète (protocole RAPPORT.md §1.2)
bench-baseline: build
    hyperfine \
      --warmup 10 --runs 100 --shell=none \
      --export-json results_baseline.json \
      --export-markdown results_baseline.md \
      './target/release/bench'

# Flamegraph interactif via Firefox Profiler
profile: build
    samply record ./target/release/bench

# Extraction des métriques depuis un fichier JSON (usage : just stats results_baseline.json)
stats FILE:
    #!/usr/bin/env python3
    import json, statistics as s
    t = json.load(open("{{FILE}}"))["results"][0]["times"]
    print("Moyenne   : %.1f ms" % (s.mean(t)*1000))
    print("Mediane   : %.1f ms" % (s.median(t)*1000))
    print("Ecart-type: %.1f ms" % (s.stdev(t)*1000))
    print("Variance  : %.2f ms2" % (s.variance(t)*1e6))
    print("Min       : %.1f ms" % (min(t)*1000))
    print("Max       : %.1f ms" % (max(t)*1000))
