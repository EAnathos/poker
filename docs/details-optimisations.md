# Détails des optimisations — Setup A

> Données complètes retirées du RAPPORT principal pour respecter la limite de 5 pages.
> Toutes les mesures sont sur **Setup A** (AMD Ryzen 5 5600H, Arch Linux, rustc 1.98.1).
> Pour Setup B (Windows), voir [`benchmarks-setup-b.md`](benchmarks-setup-b.md).

---

## Scénarios

### SC1 - AA vs KK vs QJs | Flop 9♠-T♠-2♦ | 50 000 itérations

3 joueurs, 2 cartes inconnues au board (turn + river). Présence fréquente de flush et quinte → toutes les branches d'`evaluate_five` exercées. Référence de complexité **moyenne**.

### SC2 - AhKh vs 9c9d vs JcTc | Turn 9♥-8♥-2♠-3♥ | 75 000 itérations

3 joueurs, **1 seule carte inconnue**. AhKh : tirage couleur nut ; 9c9d : brelan ; JcTc : quinte + flush trèfle. 75 000 iters justifiées par la réduction du coût par itération (erreur ±0,37 %).

### SC3 - 7s6s vs AdKd vs QcQh | Flop 8♠-9♦-2♠ | 50 000 itérations

3 joueurs. Équité très distribuée (55 % / 15 % / 30 %) : board maximisant les tirages simultanés, toutes les catégories de mains représentées.

### SC4 - As5s vs KdKc vs QhJh vs Joueur inconnu | Flop 2♠-3♦-8♠ | 30 000 itérations

4 joueurs, **1 joueur inconnu** (cartes `None`). Cas le plus coûteux : 4 évaluations + tirage de 2 cartes supplémentaires. 30 000 iters pour rester sous 1,8 s.

### SC5 - Ah5h vs JdJc vs 7c6c | Flop 4♥-5♠-6♥ | 50 000 itérations

3 joueurs. Board le plus connecté (3 cartes consécutives, 2 cœurs). Valide la détection de wheel (A-2-3-4-5) via le chemin `is_wheel`.

### SC6 - AhKh vs QsQd | Flop J♥-T♥-2♣ | 500 000 itérations

2 joueurs, mains connues, tirage royal flush nut. Cas le moins coûteux par itération → exploité pour haute précision (±0,14 %) et comme amplificateur d'optimisation. Référence principale du rapport.

---

## 4.1 ZeroAllocEvaluator — Profil samply et résultats complets

### Profil samply — zero_alloc SC6 (1 072 samples)

```
just profile sc6 zero_alloc
```

| Symbole | Self time |
|---|---|
| `poker::eval::zero_alloc::eval5` (corps) | **50,7 %** |
| `<Ordering as PartialEq>::eq` | 9,0 % |
| `insert_tail::<(u8,u8)>` (tri fréquences) | 7,9 % |
| `insert_tail::<u8>` (tri rangs) | 6,3 % |
| `<u8 as PartialOrd>::lt` | 5,4 % |
| `ptr::copy::<Card>` (shuffle) | 3,5 % |
| `best7` | 2,0 % |
| `Rng::next` | 1,1 % |
| reste | 14,1 % |

*Analyse :* Le shuffle (`Rng::next` + `ptr::copy` = ~4,6 %) est négligeable. Le goulot est `eval5` à ~95 %, dont ~30 % pour les deux `sort_unstable_by` sur 5 éléments. L'insertion sort LLVM-inliné sur 5 octets est pourtant quasi-gratuit — voir §4.2.

**Note profiling Windows :** samply affiche des adresses hexadécimales sur Windows (incompatibilité DWARF / PDB). Les hotspots sont structurels à l'algorithme, indépendants de l'OS.

### Résultats SC1–SC6

| Scénario | naive iters/s | zero_alloc iters/s | Speedup |
|---|---|---|---|
| SC1 - 3j flop, 50k | 133 400 | 287 900 | **×2,16** |
| SC2 - 3j turn, 75k | 165 100 | 482 600 | **×2,92** |
| SC3 - 3j flop, 50k | 125 900 | 274 400 | **×2,18** |
| SC4 - 4j flop, 30k | 90 000 | 184 500 | **×2,05** |
| SC5 - 3j flop, 50k | 133 200 | 269 800 | **×2,03** |
| SC6 - 2j flop, 500k | 200 200 | 461 700 | **×2,31** |

SC2 (×2,92) : turn = 1 seule carte inconnue → le shuffle pèse proportionnellement moins, `eval5` domine davantage. SC4 (×2,05) : tirage de cartes joueur inconnu non couvert par l'optimisation.

### Hyperfine SC6 détaillé

```
Benchmark 1: naive      sc6 — Time (mean ± σ): 2.497 s ±  0.014 s
Benchmark 2: zero_alloc sc6 — Time (mean ± σ): 1.083 s ±  0.017 s
Summary: zero_alloc sc6 ran 2.31 ± 0.04 times faster than naive sc6
```

---

## 4.2 SortFreeEvaluator — Analyse de la régression

### Comparaison des opérations

| Étape | zero_alloc | sort_free |
|---|---|---|
| Construction `[u8;5]` | O(5) | — |
| Tri rangs (insertion sort) | ~12 cmp inlinées | — |
| Détection quinte | 4 comparaisons | O(9) × 5 accès freq = O(45) |
| Construction `[(u8,u8)]` | O(13) | O(13) |
| Tri fréquences | ~12 cmp inlinées | — |
| Remplissage buckets | — | O(13) + match 4 branches |
| **Total** | **~54 ops** | **~71 ops** |

La loi d'Amdahl suppose que la portion à optimiser disparaît sans coût de remplacement. Ici le remplacement ajoute ~17 opérations là où le tri n'en coûtait que ~12 chacun. Pour n ≤ 5 et des données tenant dans une ligne de cache, le tri est quasi-gratuit.

### Recadrage rétrospectif

`eval5` à 50 % du runtime était trompeur : c'est parce qu'elle était appelée **21 fois** (C(7,5)) que son coût total était élevé. Optimiser l'implémentation d'`eval5` revenait à rendre plus rapide chacun des 21 aller-retours — sans jamais questionner si les 21 voyages étaient nécessaires. Le vrai levier était de supprimer l'énumération combinatoire (→ eval7, §4.4).

---

## 4.3 FisherEvaluator — Résultats SC1–SC6

| Scénario | zero_alloc iters/s | fisher iters/s | Speedup | n_needed |
|---|---|---|---|---|
| SC1 - 3j flop, 50k | 287 900 | 309 600 | **×1,08** | 2 |
| SC2 - 3j turn, 75k | 482 600 | 543 500 | **×1,13** | 1 |
| SC3 - 3j flop, 50k | 274 400 | 300 800 | **×1,10** | 2 |
| SC4 - 4j flop, 30k | 184 500 | 190 200 | **×1,03** | 4 |
| SC5 - 3j flop, 50k | 269 800 | 286 000 | **×1,06** | 2 |
| SC6 - 2j flop, 500k | 461 700 | 508 800 | **×1,10** | 2 |

SC4 (×1,03) : n_needed = 4 (2 board + 2 mains joueur inconnu), mais le coût absolu par iter est plus élevé → overhead proportionnellement moindre du shuffle. SC2 (×1,13) : n_needed = 1, le gain Lemire est amplifié.

### Implémentation partial_shuffle + Lemire

```rust
fn partial_shuffle(&mut self, v: &mut [Card], k: usize) {
    let n = v.len();
    for i in 0..k {
        let range = (n - i) as u64;
        // Lemire 2019 : (u64 × range) >> 64 — uniforme sans division
        let j = i + ((self.next() as u128 * range as u128) >> 64) as usize;
        v.swap(i, j);
    }
}
```

---

## 4.4 Eval7Evaluator — Résultats SC1–SC6

| Scénario | fisher iters/s | eval7 iters/s | Speedup vs fisher |
|---|---|---|---|
| SC1 - 3j flop, 50k | 309 600 | 4 761 900 | **×15,38** |
| SC2 - 3j turn, 75k | 543 500 | 6 250 000 | **×11,50** |
| SC3 - 3j flop, 50k | 300 800 | 4 464 300 | **×14,84** |
| SC4 - 4j flop, 30k | 190 200 | 2 907 000 | **×15,28** |
| SC5 - 3j flop, 50k | 286 000 | 4 347 800 | **×15,20** |
| SC6 - 2j flop, 500k | 508 800 | 7 042 300 | **×13,84** |

### Progression cumulée SC1

| Évaluateur | iters/s | Gain cumulé vs zero_alloc |
|---|---|---|
| `zero_alloc` | 287 900 | — |
| `fisher` | 309 600 | ×1,08 |
| `eval7` | 4 761 900 | **×16,54** |

### Structure eval7

```
1. Boucle 7 cartes → freq[2..=14] + suit_cnt[0..3]
2. Flush (suit_cnt[s] ≥ 5) → sort flush ranks, detect straight flush → ROYAL/STR_FLUSH/FLUSH
3. cnt array [(rank, freq)] trié (freq desc, rank desc) → cnt[0] = groupe dominant
4. Scan quinte descendant freq[h..h-4] > 0 (O(9) max)
5. Lecture directe cnt[0].1 → FOUR_KIND / FULL_HOUSE / ... / HIGH_CARD
```

---

## 4.5 LutEvaluator — Détails d'implémentation

### Table flush (32 Ko, L1)

Indexée par masque 13 bits (`flush_mask`, bit k = présence du rang k+2). Énumération des ~1 378 masques valides (popcount ∈ {5,6,7}). Tient entièrement en L1 (32 KiB) → hits L1 après la première main flush.

### Table non-flush (1,5 Mo, L3)

Indexée par clé base-5 bijective : `freq_key = Σ freq[r] × 5^(r-2)` pour r ∈ [2..14]. Open addressing (131 072 slots, ~38 % de charge → < 1,3 sondages en moyenne). ~50 000 patterns distincts construits récursivement. Build : < 1 ms.

### Seuil adaptatif

```rust
const LUT_THRESHOLD: u32 = 200_000;

let lut_opt: Option<&'static LutData> = if iterations >= LUT_THRESHOLD {
    Some(get_lut())   // OnceLock : build uniquement au premier appel
} else {
    None              // get_lut() jamais appelé, tables non allouées
};

ranks[i] = match lut_opt {
    Some(lut) => eval_lut(&seven, lut),
    None      => eval7_inline(&seven),
};
```

LLVM hisse la branche hors de la boucle (loop unswitching) — aucun coût dans le hot path.

### Résultats SC1–SC6

| Scénario | eval7 iters/s | lut iters/s | Comportement | Ratio |
|---|---|---|---|---|
| SC1 - 3j flop, 50k | 4 761 900 | // | < seuil → eval7_inline | ~×1,0 (bruit) |
| SC2 - 3j turn, 75k | 6 250 000 | // | < seuil → eval7_inline | ~×1,0 (bruit) |
| SC3 - 3j flop, 50k | 4 464 300 | // | < seuil → eval7_inline | ~×1,0 (bruit) |
| SC4 - 4j flop, 30k | 2 907 000 | // | < seuil → eval7_inline | ~×1,0 (bruit) |
| SC5 - 3j flop, 50k | 4 347 800 | // | < seuil → eval7_inline | ~×1,0 (bruit) |
| SC6 - 2j flop, 500k | 7 042 300 | 11 520 700 | ≥ seuil → LUT chaud | **×1,64** |

### Analyse gain SC6

1. **Flush (32 Ko en L1)** : accès L1 (~4 cycles) vs sort + scan (~20 instr.). Gain local élevé, mais poids faible (~3 % des mains).
2. **Non-flush (1,5 Mo en L3)** : accès L3 ~40 cycles vs eval7 ~32 cycles. Le gain global vient de la suppression de la variation (chemin unique vs branches multiples), pas du ratio brut cycles.

Limite : la chaîne de 13 multiply-add de `freq_key` (~39 cycles de latence séquentielle) compense une partie du gain. Un encodage par contributions pré-calculées (`CONTRIB[r][f] = f × 5^r`, somme parallélisable) pourrait améliorer ce point.

---

## 4.6 LutParEvaluator — Résultats complets et User time

### Résultats SC1–SC6

| Scénario | lut iters/s | lut_par iters/s | Speedup |
|---|---|---|---|
| SC1 - 3j flop, 50k | 4 717 000 | 14 706 000 | **×3,12** |
| SC2 - 3j turn, 75k | 6 303 000 | 20 270 000 | **×3,23** |
| SC3 - 3j flop, 50k | 4 386 000 | 14 286 000 | **×3,28** |
| SC4 - 4j flop, 30k | 2 924 000 | 10 204 000 | **×3,50** |
| SC5 - 3j flop, 50k | 4 286 000 | 11 111 000 | **×2,55** |
| SC6 - 2j flop, 500k | 11 682 000 | 28 902 000 | **×2,47** |

### Analyse User time (threads actifs effectifs)

| Scénario | Wall time | User time | Threads effectifs |
|---|---|---|---|
| SC1 | 3,4 ms | 18,2 ms | ~5,4 |
| SC2 | 3,7 ms | 20,6 ms | ~5,6 |
| SC3 | 3,5 ms | 19,8 ms | ~5,7 |
| SC4 | 4,9 ms | 27,3 ms | ~5,6 |
| SC5 | 2,7 ms | 11,6 ms | ~4,3 |
| SC6 | 17,3 ms | 62,8 ms | ~3,6 |

SC6 : ratio User/Wall = 3,6 threads effectifs (vs 5,6 pour SC4) → les stalls L3 laissent les cœurs en attente. SC5 (×2,55, le plus faible hors SC6) : l'overhead rayon (~0,3 ms) représente ~11 % du wall time à 2,7 ms total.

### Seeding RNG par thread

```rust
fn new_seeded(tid: usize) -> Self {
    let base = SystemTime::now()...as u64;
    // Fibonacci doré 64-bit : seeds distincts sur tous les bits entre tid consécutifs
    let seed = base ^ (tid as u64).wrapping_mul(0x9e3779b97f4a7c15);
    Self(if seed == 0 { 1 } else { seed })
}
```

La LUT (`&'static LutData`) est `Send + Sync` par construction : les 6 threads lisent simultanément sans traffic de cohérence MESI.
