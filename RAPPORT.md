# Rapport d'Audit de Performance — Poker Monte Carlo (Rust)

Sup de Vinci - RNCP Bloc 4 - Session E42 Optimisations

---

## 1. Environnement & Métrologie (Baseline)

### 1.1 Spécification des bancs d'essai matériels

**Setup A**

| Composant | Détail |
|-----------|--------|
| CPU | AMD Ryzen 5 5600H with Radeon Graphics |
| Cœurs / Threads | 6C / 12T |
| Cache L1 | 32 KiB par cœur (données) + 32 KiB par cœur (instructions) |
| Cache L2 | 512 KiB par cœur |
| Cache L3 | 16 MiB partagé |
| RAM | 16 GiB DDR4-3200 (2×8 GiB) |
| OS | Arch Linux kernel 7.1.11 |
| Runtime Rust | rustc 1.98.1 |

**Setup B**

| Composant | Détail |
|-----------|--------|
| CPU | <!-- ex. AMD Ryzen 9 5900X --> |
| Cœurs / Threads | <!-- ex. 12C / 24T --> |
| Cache L1 | <!-- ex. 32 KB par cœur (données) --> |
| Cache L2 | <!-- ex. 512 KB par cœur --> |
| Cache L3 | <!-- ex. 64 MB partagé --> |
| RAM | <!-- ex. 64 GB DDR4-3200 --> |
| OS | <!-- ex. Ubuntu 24.04 LTS --> |
| Runtime Rust | <!-- ex. rustc 1.XX.X (stable) --> |

### 1.2 Protocole de mesure (Hyperfine)

```bash
# TODO : compléter avec la commande hyperfine exacte utilisée
hyperfine \
  --warmup 5 \
  --runs 50 \
  --export-json results_baseline.json \
  './target/release/poker'
```

**Setup A**

| Métrique | Valeur baseline |
|----------|----------------|
| Moyenne | <!-- µs ou ms --> |
| Médiane | |
| Écart-type | |
| Variance | |
| Min | |
| Max | |

> Isolation : <!-- décrire les processus parasites fermés, CPU governor fixé en performance, etc. -->

**Setup B**

| Métrique | Valeur baseline |
|----------|----------------|
| Moyenne | <!-- µs ou ms --> |
| Médiane | |
| Écart-type | |
| Variance | |
| Min | |
| Max | |

> Isolation : <!-- décrire les processus parasites fermés, CPU governor fixé en performance, etc. -->
