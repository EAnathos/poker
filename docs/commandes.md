# Référence des commandes just

## Outillage

| Commande | Action |
|----------|--------|
| `just install-tools` | Installe hyperfine + samply via cargo-binstall |
| `just build` | Compile le binaire `bench` en mode release |
| `just play` | Lance l'interface graphique (GUI Texas Hold'em) |
| `just format` | Formate le code (`cargo fmt`) |
| `just lint` | Vérifie le code (`cargo clippy -D warnings`) |

## Benchmarks

| Commande | Action |
|----------|--------|
| `just bench sc1` … `just bench sc6` | Compare tous les évaluateurs sur un scénario (100 runs / 10 runs pour sc6) |
| `just bench sc1 lut_par` | Un seul évaluateur sur un scénario |
| `just bench sc1 lut lut_par` | Deux évaluateurs en comparaison directe |
| `just bench-all` | Comparatif sur l'ensemble SC1–SC6 (10 runs, warmup 3) |

**Paramètres Hyperfine :**
- `--warmup 10` : écarte les 10 premiers runs (cache froid, montée en fréquence CPU)
- `--runs 100` : réduit l'erreur standard à σ/√100
- `--shell=none` : retire le bruit du shell

## Profiling

| Commande | Action |
|----------|--------|
| `just profile` | Flamegraph samply → Firefox Profiler (défaut : sc6 sort_free) |
| `just profile sc6 lut_par` | Scénario et évaluateur explicites |
| `just stats results/sc6.json` | Extrait moyenne/médiane/σ/min/max depuis le JSON Hyperfine |

Firefox Profiler s'ouvre automatiquement. Requiert `debug = 1` et `strip = "none"` dans `Cargo.toml`.

## Mémoire

| Commande | Action |
|----------|--------|
| `just mem` | Pic de RAM (VmHWM) — tous les évaluateurs sur sc6 |
| `just mem sc1` | Tous les évaluateurs sur un scénario donné |
| `just mem sc6 lut_par` | Un seul évaluateur sur un scénario |

La mesure provient de `VmHWM` dans `/proc/self/status` (peak RSS, Linux uniquement).

## Compteurs matériels bas niveau (sans just)

```bash
perf stat -e cache-misses,cache-references,instructions,cycles \
  ./target/release/bench sc1 lut_par
```
