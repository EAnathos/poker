# Prérequis : cargo binstall (https://github.com/cargo-bins/cargo-binstall)
# Installation : cargo install cargo-binstall

# ── Shell Windows (Linux garde sh par défaut) ─────────────────────────────────
set windows-shell := ["powershell.exe", "-NoLogo", "-NonInteractive", "-Command"]

RESULTS_DIR := "results"
EXE         := if os() == "windows" { "target/release/bench.exe" } else { "target/release/bench" }

# ── Outillage ─────────────────────────────────────────────────────────────────

install-tools:
    cargo binstall hyperfine samply --no-confirm

build:
    cargo build --release --bin bench

format:
    cargo fmt

lint:
    cargo clippy -- -D warnings

[unix]
_mkdir-results:
    mkdir -p {{RESULTS_DIR}}

[windows]
_mkdir-results:
    New-Item -ItemType Directory -Force -Path {{RESULTS_DIR}} | Out-Null

# ── Benchmarks ────────────────────────────────────────────────────────────────

# Benchmark d'un scénario — just bench sc1 [naive|fast]
# Sans évaluateur : compare naive vs fast. Avec : un seul.
# SC6 (500k iters) : warmup 3, runs 10 — autres : warmup 10, runs 100
[unix]
bench SC EVAL="": build _mkdir-results
    #!/usr/bin/env bash
    set -e
    if [ "{{SC}}" = "sc6" ]; then warmup=3; runs=10; else warmup=10; runs=100; fi
    if [ -z "{{EVAL}}" ]; then
      hyperfine \
        --warmup $warmup --runs $runs --shell=none \
        --command-name "naive {{SC}}" \
        --command-name "fast  {{SC}}" \
        --export-json {{RESULTS_DIR}}/{{SC}}.json \
        --export-markdown {{RESULTS_DIR}}/{{SC}}.md \
        '{{EXE}} {{SC}} naive' \
        '{{EXE}} {{SC}} fast'
    else
      hyperfine \
        --warmup $warmup --runs $runs --shell=none \
        --command-name "{{EVAL}} {{SC}}" \
        --export-json {{RESULTS_DIR}}/{{SC}}_{{EVAL}}.json \
        --export-markdown {{RESULTS_DIR}}/{{SC}}_{{EVAL}}.md \
        '{{EXE}} {{SC}} {{EVAL}}'
    fi

[windows]
bench SC EVAL="": build _mkdir-results
    hyperfine --warmup {{ if SC == "sc6" { "3" } else { "10" } }} --runs {{ if SC == "sc6" { "10" } else { "100" } }} --shell=none {{ if EVAL == "" { '--command-name "naive ' + SC + '" --command-name "fast ' + SC + '" "' + EXE + ' ' + SC + ' naive" "' + EXE + ' ' + SC + ' fast" --export-json "' + RESULTS_DIR + '/' + SC + '.json" --export-markdown "' + RESULTS_DIR + '/' + SC + '.md"' } else { '--command-name "' + EVAL + ' ' + SC + '" "' + EXE + ' ' + SC + ' ' + EVAL + '" --export-json "' + RESULTS_DIR + '/' + SC + '_' + EVAL + '.json" --export-markdown "' + RESULTS_DIR + '/' + SC + '_' + EVAL + '.md"' } }}

# Compare naive vs fast sur l'ensemble SC1–SC6
bench-all: build _mkdir-results
    hyperfine \
      --warmup 3 --runs 10 --shell=none \
      --command-name "naive" \
      --command-name "fast" \
      --export-json {{RESULTS_DIR}}/all.json \
      --export-markdown {{RESULTS_DIR}}/all.md \
      '{{EXE}} naive' \
      '{{EXE}} fast'

# Flamegraph interactif via Firefox Profiler — just profile [sc6] [fast|naive]
profile ARGS="sc6 fast": build
    samply record {{EXE}} {{ARGS}}

# ── Métriques ─────────────────────────────────────────────────────────────────

# Extraction des métriques depuis un fichier JSON (usage : just stats results/sc6.json)
[unix]
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

[windows]
stats FILE:
    python -c "import json,statistics as s; data=json.load(open('{{FILE}}'))['results']; [print('['+r['command']+']\n  Moyenne   : %.1f ms\n  Mediane   : %.1f ms\n  Ecart-type: %.1f ms\n  Min/Max   : %.1f / %.1f ms' % (s.mean(r['times'])*1000,s.median(r['times'])*1000,s.stdev(r['times'])*1000,min(r['times'])*1000,max(r['times'])*1000)) for r in data]"
