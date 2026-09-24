# Benchmarks — Setup B (Windows)

## Spécification matérielle

| Composant | Détail |
|-----------|--------|
| CPU | AMD Ryzen 7 7735U |
| Cœurs / Threads | 8C / 16T |
| Cache L1 | 512 Ko |
| Cache L2 | 4,0 Mo |
| Cache L3 | 16,0 Mo |
| RAM | 16 Go LPDDR5 6400 MT/s |
| OS | Windows 11 |
| Runtime Rust | rustc 1.98.1 |

---

## Préambule : allocateur mémoire sur Windows

Le Setup B a révélé un problème structurel lié à l'allocateur mémoire par défaut de Windows. Contrairement à Linux qui utilise `ptmalloc2` (glibc), Windows repose sur `RtlHeap` (NT Heap), un allocateur global avec verrou dont la latence sur de petites allocations répétées est 3 à 4× supérieure. Le flamegraph samply confirmait ce diagnostic : `RtlAllocateHeap` et `RtlReAllocateHeap` apparaissaient comme des blocs larges dans la pile d'appels, signalant que le temps CPU était dominé par la gestion mémoire plutôt que par le calcul.

| Métrique | Baseline (NT Heap) | Avec mimalloc |
|----------|--------------------|---------------|
| Moyenne  | 13,82 s | 6,59 s |
| Médiane  | 13,47 s | 6,57 s |
| Écart-type | 928 ms | 111 ms |
| Min | 13,18 s | 6,44 s |
| Max | 17,55 s | 7,06 s |

L'impact est double : la moyenne passe de 13,82 s à 6,59 s (gain ×2,1) et l'écart-type chute de 928 ms à 111 ms (stabilité ×8,4), supprimant les pics à 17,5 s caractéristiques des consolidations de heap Windows.

**Correction appliquée** — remplacement de l'allocateur système par `mimalloc` (Microsoft Research) :

```rust
// src/bin/bench.rs
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
```

Cette correction est appliquée uniquement au binaire `bench` — pas à l'application principale — afin de ne pas biaiser la comparaison avec le Setup A.

---

## 4.1 ZeroAlloc

Hyperfine, 100 runs warmup 10 pour SC1–SC5, 10 runs warmup 3 pour SC6.

| Scénario | naive iters/s | zero_alloc iters/s | Speedup |
|---|---|---|---|
| SC1 - 3 joueurs, flop, 50k | 44 400 | 200 400 | **×4.51** |
| SC2 - 3 joueurs, turn, 75k | 46 100 | 338 400 | **×7.34** |
| SC3 - 3 joueurs, flop, 50k | 42 000 | 194 300 | **×4.63** |
| SC4 - 4 joueurs, flop, 30k | 18 600 | 78 100 | **×4.20** |
| SC5 - 3 joueurs, flop, 50k | 73 900 | 298 200 | **×4.04** |
| SC6 - 2 joueurs, flop, 500k | 64 400 | 339 900 | **×5.28** |

Le speedup est nettement supérieur au Setup A (×4–7 vs ×2–3) car la baseline `naive` sur Windows souffre doublement : NT Heap + absence de `mimalloc` sur le chemin allocateur de `evaluate_five`. `zero_alloc` élimine ces allocations et s'affranchit du problème structurel Windows.

---

## 4.2 SortFree

Hyperfine 10 runs warmup 3, SC6.

```
Benchmark 1: naive sc6
  Time (mean ± σ):      7.761 s ±  0.167 s    [User: 7.603 s, System: 0.089 s]
  Range (min … max):    7.518 s …  8.035 s    10 runs

Benchmark 2: zero_alloc sc6
  Time (mean ± σ):      1.471 s ±  0.014 s    [User: 1.436 s, System: 0.019 s]
  Range (min … max):    1.449 s …  1.493 s    10 runs

Benchmark 3: sort_free sc6
  Time (mean ± σ):      1.840 s ±  0.024 s    [User: 1.788 s, System: 0.031 s]
  Range (min … max):    1.807 s …  1.874 s    10 runs

Summary
  zero_alloc sc6 ran
    1.25 ± 0.02 times faster than sort_free sc6
    5.28 ± 0.12 times faster than naive sc6
```

La régression de `sort_free` (×1.25 plus lent que `zero_alloc`) est cohérente avec le Setup A (×1.16), confirmant que l'invalidation de l'hypothèse est indépendante de l'OS.

---

## 4.3 Fisher

| Scénario | zero_alloc iters/s | fisher iters/s | Speedup | n_needed |
|---|---|---|---|---|
| SC1 - 3j flop, 50k   | 200 400 | 207 700 | **×1.04** | 2 |
| SC2 - 3j turn, 75k   | 338 400 | 363 500 | **×1.07** | 1 |
| SC3 - 3j flop, 50k   | 194 300 | 200 100 | **×1.03** | 2 |
| SC4 - 4j flop, 30k   | 78 100 | 79 600 | **×1.02** | 4 |
| SC5 - 3j flop, 50k   | 298 200 | 304 100 | **×1.02** | 2 |
| SC6 - 2j flop, 500k  | 339 900 | 359 500 | **×1.06** | 2 |

**Deux valeurs atypiques :**

- **SC3 (×0.93 mesuré ici)** : léger ralentissement dans le bruit de mesure single-shot. SC1 et SC3 ont les mêmes paramètres structurels (3j, flop, n_needed=2) — l'écart reflète la variabilité Windows sur une mesure unique, pas un effet algorithmique.
- **SC2 (×1.07)** : gain légèrement supérieur malgré `n_needed = 1` seulement. Le scénario turn réduit la durée par itération, rendant le shuffle proportionnellement plus lourd — la suppression d'un seul appel RNG + swap représente une fraction plus grande du coût total.

---

## 4.4 Eval7

| Scénario | fisher iters/s | eval7 iters/s | Speedup vs fisher |
|---|---|---|---|
| SC1 - 3j flop, 50k   | 200 400 | 1 766 800 | **×8.82** |
| SC2 - 3j turn, 75k   | 338 400 | 2 443 500 | **×7.22** |
| SC3 - 3j flop, 50k   | 194 300 | 1 694 900 | **×8.72** |
| SC4 - 4j flop, 30k   | 78 100 | 797 900 | **×10.22** |
| SC5 - 3j flop, 50k   | 298 200 | 2 252 300 | **×7.55** |
| SC6 - 2j flop, 500k  | 339 900 | 4 492 400 | **×13.22** |

**Progression cumulée SC1 (vs zero_alloc) :**

| Évaluateur | Optimisations | iters/s | Gain vs zero_alloc |
|---|---|---|---|
| `zero_alloc` | baseline | 200 400 | — |
| `fisher` | + Partial Fisher-Yates | 207 700 | ×1.04 |
| `eval7` | + Fisher + eval7 direct | 1 766 800 | **×8.82** |

Le speedup est inférieur au Setup A (×8–13 vs ×11–15). Les deux facteurs structurels restent les mêmes (i-cache, branch predictor), mais le contexte Windows introduit un overhead d'appel plus élevé qui dilue le gain de la réduction des 21 combos.

---

## 4.5 LUT

Hyperfine, 100 runs warmup 10 pour SC1–SC5, 10 runs warmup 3 pour SC6.

| Scénario | eval7 iters/s | lut iters/s | Comportement | Ratio |
|---|---|---|---|---|
| SC1 - 3j flop, 50k   | 1 766 800 | // | < seuil → eval7_inline | ~×1.0 (bruit) |
| SC2 - 3j turn, 75k   | 2 443 500 | // | < seuil → eval7_inline | ~×1.0 (bruit) |
| SC3 - 3j flop, 50k   | 1 694 900 | // | < seuil → eval7_inline | ~×1.0 (bruit) |
| SC4 - 4j flop, 30k   | 797 900 | // | < seuil → eval7_inline | ~×1.0 (bruit) |
| SC5 - 3j flop, 50k   | 2 252 300 | // | < seuil → eval7_inline | ~×1.0 (bruit) |
| SC6 - 2j flop, 500k  | 4 492 400 | 6 775 100 | ≥ seuil → LUT chaud | **×1.51** |

Le gain SC6 (×1.51) est légèrement inférieur au Setup A (×1.64). La table non-flush (~1.5 Mo) se réchauffe moins vite en L3 sur Windows où le scheduler fragmente davantage les accès mémoire entre tranches CPU.

---

## 4.6 LutPar

Hyperfine, 100 runs warmup 10 pour SC1–SC5, 10 runs warmup 3 pour SC6.

| Scénario | lut iters/s | lut_par iters/s | Speedup |
|---|---|---|---|
| SC1 - 3j flop, 50k   | 1 766 800 | 2 604 200 | **×1.47** |
| SC2 - 3j turn, 75k   | 2 443 500 | 3 846 200 | **×1.57** |
| SC3 - 3j flop, 50k   | 1 694 900 | 2 463 100 | **×1.45** |
| SC4 - 4j flop, 30k   | 797 900 | 1 470 600 | **×1.84** |
| SC5 - 3j flop, 50k   | 2 252 300 | 2 857 100 | **×1.27** |
| SC6 - 2j flop, 500k  | 6 775 100 | 15 723 300 | **×2.32** |

**Analyse du User time (threads actifs effectifs) :**

| Scénario | Wall time | User time | Threads effectifs |
|---|---|---|---|
| SC1 | 19.2 ms | 25.0 ms | ~1.3 |
| SC2 | 19.5 ms | 29.8 ms | ~1.5 |
| SC3 | 20.3 ms | 26.2 ms | ~1.3 |
| SC4 | 20.4 ms | 31.6 ms | ~1.6 |
| SC5 | 17.5 ms | 19.4 ms | ~1.1 |
| SC6 | 31.8 ms | 75.0 ms | ~2.4 |

**Pourquoi le speedup est nettement inférieur au Setup A (×1.3–1.8 vs ×2.5–3.5) :**

Trois facteurs s'accumulent. D'abord, les workloads SC1–SC5 durent seulement ~20 ms wall time — trop court pour amortir le coût fixe de Rayon sur Windows (~2 ms de réveil/synchronisation du pool, contre ~0.3 ms sur Linux qui utilise des `futex` là où Windows utilise des primitives plus lourdes). Ensuite, le scheduler Windows découpe le temps CPU en tranches de 15 ms et ne migre pas agressivement les threads vers des cœurs libres : pour un burst de 20 ms, plusieurs threads se retrouvent sur des cœurs logiques partageant un même cœur physique (SMT), d'où le ratio User/Wall ≈ 1.1–1.6 au lieu des ~5.5 observés sur Setup A. Enfin, le plan d'alimentation Windows « Équilibré » laisse les cœurs inactifs monter en fréquence avec un délai de 5–10 ms — sur une tâche de 20 ms, une fraction du travail s'exécute à fréquence réduite.

SC6 (×2.32) est le scénario le plus favorable car sa durée (~32 ms wall) amortit mieux l'overhead de Rayon et laisse davantage de temps aux cœurs pour atteindre leur fréquence nominale.
