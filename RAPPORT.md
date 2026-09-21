# Rapport d'Audit de Performance — Poker Monte Carlo (Rust)

Sup de Vinci - RNCP Bloc 4 - Session E42 Optimisations

---

## 1. Environnement & Métrologie (Baseline)

### 1.1 Spécification des bancs d'essai matériels

**Setup A**

| Composant | Détail |
|-----------|--------|
| CPU | <!-- ex. Intel Core i7-12700H --> |
| Cœurs / Threads | <!-- ex. 14C / 20T --> |
| Cache L1 | <!-- ex. 48 KB par cœur (données) --> |
| Cache L2 | <!-- ex. 1.25 MB par cœur --> |
| Cache L3 | <!-- ex. 24 MB partagé --> |
| RAM | <!-- ex. 32 GB DDR5-4800 --> |
| OS | <!-- ex. Arch Linux kernel 7.1.11-arch1-1 --> |
| Runtime Rust | <!-- ex. rustc 1.XX.X (stable) --> |

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
