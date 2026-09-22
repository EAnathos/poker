# Rapport d'Audit de Performance - Poker Monte Carlo (Rust)

Sup de Vinci - RNCP Bloc 4 - Session E42 Optimisations

---

## 1. Présentation du projet

### 1.1 Concept

Ce projet implémente un **moteur de calcul de probabilités Monte Carlo** pour le Texas Hold'em en Rust. Étant donné un ensemble de mains connues et un board partiel, le moteur estime l'équité de chaque joueur par simulation : il complète le board manquant et les cartes inconnues de façon aléatoire, évalue la meilleure main à 7 cartes pour chaque joueur, et répète l'opération des dizaines ou centaines de milliers de fois pour converger vers une probabilité de victoire.

Deux objectifs distincts gouvernent le projet :
1. **Correction algorithmique** - produire des équités fiables (erreur Monte Carlo ≤ ±0,45 % sur 50 000 itérations).
2. **Performance** - atteindre le minimum physique d'exécution en supprimant les allocations inutiles et en exploitant la localité mémoire.

### 1.2 Architecture

Deux évaluateurs coexistent dans `src/eval/` :

| Évaluateur | Fichier | Approche | Rôle |
|---|---|---|---|
| `naive` | `src/eval/naive.rs` | Allocations heap (`Vec`) à chaque appel | Référence de correction |
| `fast` | `src/eval/fast.rs` | Buffers stack, encodage `u32` | Cible d'optimisation |

Le binaire `bench` (`src/bin/bench.rs`) orchestre les simulations et sert de point de mesure Hyperfine. Chaque exécution prend un scénario (`sc1`–`sc6`) et un évaluateur (`naive` ou `fast`) en argument de ligne de commande.

---

## 2. Environnement & Métrologie (Baseline)

### 2.1 Spécification des bancs d'essai matériels

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
| CPU | AMD Rysen 7 7735U |
| Cœurs / Threads | 8C 16T |
| Cache L1 | 512 Ko |
| Cache L2 | 4.0 Mo |
| Cache L3 | 16.0 Mo |
| RAM | 16Go 	LPDDR5 6400 MT/s |
| OS | Windows 11 |
| Runtime Rust | rustc 1.98.1 |

### 2.2 Outils de profiling

#### Installation

```bash
# cargo-binstall — installe les binaires Rust sans recompilation (à faire en premier)
cargo install cargo-binstall

# just — exécuteur de recettes (via binstall, pas de compilation)
cargo binstall just

# Hyperfine et Samply via binstall (rapide, pas de compilation)
just install-tools
```

#### Activer les symboles de debug en release

Requis pour que samply affiche les noms de fonctions Rust (et non des adresses hexadécimales).
Ajouter dans `Cargo.toml` :

```toml
[profile.release]
debug = 1        # line tables seulement — suffit pour samply
strip = "none"   # garde les symboles — obligatoire
```

#### Compiler le binaire de benchmark en release

```bash
just build
```

#### Profiling avec Samply

```bash
just profile

# Scénario et évaluateur explicites
just profile sc6 fast
just profile sc1 naive
```

Firefox Profiler s'ouvre automatiquement avec le flamegraph interactif.

#### Protocole de mesure (Hyperfine)

```bash
# Scénario individuel — compare naive vs fast sur un scénario
just bench sc1
just bench sc2
just bench sc3
just bench sc4
just bench sc5

# SC6 haute précision (500k iters — ~2.5 s/run, 10 runs)
just bench sc6

# Comparatif naive vs fast sur l'ensemble SC1–SC6 en un seul appel
just bench-all
```

**Justification des paramètres :**
- `--warmup 10` : écarte les 10 premiers runs (cache disque froid, montée en fréquence CPU).
- `--runs 100` : réduit l'erreur standard à σ/√100 ≈ 10 % de σ.
- `--shell=none` : retire le bruit du shell de la mesure (pas de fork/exec de bash).

#### Extraction des métriques complètes depuis le JSON

```bash
just stats results/baseline.json
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

| Métrique | Valeur baseline (NT Heap) | Avec mimalloc |
|----------|--------------------------|---------------|
| Moyenne  | 13,82 s | 6,59 s |
| Médiane  | 13,47 s | 6,57 s |
| Écart-type | 928 ms | 111 ms |
| Min | 13,18 s | 6,44 s |
| Max | 17,55 s | 7,06 s |

> **Isolation - problèmes spécifiques à Windows (Setup B)**
>
> Le Setup B a révélé un problème structurel lié à l'allocateur mémoire par défaut de Windows. Contrairement à Linux qui utilise `ptmalloc2` (glibc), Windows repose sur `RtlHeap` (NT Heap), un allocateur global avec verrou dont la latence sur de petites allocations répétées est 3 à 4× supérieure. Le flamegraph samply confirmait ce diagnostic : `RtlAllocateHeap` et `RtlReAllocateHeap` apparaissaient comme des blocs larges dans la pile d'appels, signalant que le temps CPU était dominé par la gestion mémoire plutôt que par le calcul.
>
> L'impact est double : la **moyenne** passe de 13,82 s à 6,59 s (gain ×2,1) et l'**écart-type** chute de 928 ms à 111 ms (stabilité ×8,4), supprimant les pics à 17,5 s observés en baseline. Ces pics sont caractéristiques des consolidations de heap que Windows effectue périodiquement sous charge.
>
> **Correction appliquée** - remplacement de l'allocateur système par `mimalloc` (Microsoft Research) via une seule ligne dans le binaire de benchmark :
>
> ```rust
> // src/bin/bench.rs
> #[global_allocator]
> static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;
> ```
>
> Cette ligne redirige tous les appels `malloc`/`free` de Rust vers mimalloc, qui utilise des arènes par thread et évite le verrou global du NT Heap. Aucune modification algorithmique n'est nécessaire, le gain est purement infrastructurel.
>
> **Note :** cette correction est appliquée uniquement au binaire `bench` et non à l'application principale, afin de ne pas biaiser la comparaison avec le Setup A (Linux) qui ne souffre pas de ce problème.

---

### 2.3 Automatisation (justfile)

| Commande | Action |
|----------|--------|
| `just install-tools` | Installe hyperfine + samply via cargo-binstall |
| `just build` | Compile le binaire `bench` en mode release |
| `just format` | Formate le code (`cargo fmt`) |
| `just lint` | Vérifie le code (`cargo clippy -D warnings`) |
| `just bench sc1` … `just bench sc6` | Compare naive vs fast sur le scénario (100 runs / 10 runs pour sc6) |
| `just bench-all` | Compare naive vs fast sur l'ensemble SC1–SC6 (10 runs, warmup 3) |
| `just profile [sc] [eval]` | Flamegraph samply → Firefox Profiler (défaut : sc6 fast) |
| `just stats results/sc6.json` | Extrait moyenne/médiane/σ/min/max du JSON pour chaque évaluateur |

La recette `bench` prend le scénario en argument (`just bench sc6`) et passe les arguments `scN naive` puis `scN fast` à deux commandes Hyperfine distinctes, le comparatif est affiché nativement avec le ratio de vitesse.

---

## 3. Scénarios de Benchmark

### 3.1 Principes de conception des scénarios

Les scénarios de benchmark couvrent quatre axes qui influencent directement la charge computationnelle de l'évaluateur Monte Carlo :

1. **Nombre de joueurs** - chaque joueur supplémentaire ajoute une évaluation `best_hand` par itération (soit 21 appels à `evaluate_five` pour 7 cartes), plus une allocation `Vec<HandValue>` plus large.
2. **Stade de jeu (flop / turn / river)** - détermine le nombre de cartes inconnues à tirer dans le deck résiduel : 2 cartes au flop, 1 au turn, 0 à la river. Moins d'inconnues = moins de variabilité par itération, ce qui justifie un nombre d'itérations plus élevé pour atteindre la même précision statistique.
3. **Présence de joueurs inconnus** - forcer le tirage de 2 cartes supplémentaires par joueur fantôme augmente la pression sur le générateur de nombres aléatoires et la gestion du deck résiduel.
4. **Richesse en draws** - les boards connectés et suiteds génèrent davantage de combinaisons de flush et de quinte à évaluer, ce qui stresse uniformément le chemin `evaluate_five → counts → tiebreak`.

Le nombre d'itérations est calibré de façon à ce que chaque scénario s'exécute entre **800 ms et 1,8 s** sur la baseline naive, garantissant :
- Un signal suffisant pour Hyperfine (réduction de l'erreur standard ≈ σ/√N avec N = 100 runs externes).
- Une résolution suffisante des probabilités (erreur Monte Carlo ≈ 1/√itérations ; 50 000 iters → ±0,45 % sur une équité de 50 %).

---

### 3.2 Description des scénarios

#### SC1 - AA vs KK vs QJs | Flop 9♠-T♠-2♦ | 50 000 itérations

**Contexte :** Situation emblématique du « big hand vs big draw ». AA est la main premium absolue, KK est en très mauvaise posture face aux aces, et QJs représente un double tirage (quinte + flush treillis) sur un board très connecté.

**Intérêt pour le benchmark :**
- 3 joueurs → 3 évaluations `best_hand` par itération.
- 2 cartes inconnues au board (turn + river) → deck résiduel de 44 cartes, shuffle complet à chaque itération.
- Présence fréquente de flush et de quinte dans les résultats → toutes les branches de `evaluate_five` sont exercées.
- Référence de complexité **moyenne** (3 joueurs, flop).

**Itérations : 50 000**, équilibre précision/temps, erreur ≈ ±0,45 %.

---

#### SC2 - AhKh vs 9c9d vs JcTc | Turn 9♥-8♥-2♠-3♥ | 75 000 itérations

**Contexte :** Scénario de **turn** (4 cartes au board) avec une seule carte inconnue restante. AhKh possède un tirage couleur nut (4 cœurs avec l'As), 9c9d a un brelan (set de 9) et JcTc dispose d'un tirage quinte + tirage couleur trèfle.

**Intérêt pour le benchmark :**
- **Seule 1 carte inconnue au board**, le deck résiduel est plus petit (43 cartes résiduelles, 1 seule à tirer), ce qui réduit la durée par itération par rapport au flop.
- Ce gain par itération est compensé par un nombre d'itérations plus élevé (75 000 au lieu de 50 000), maintenant la durée totale comparable tout en améliorant la précision (erreur ≈ ±0,37 %).
- Permet de **mesurer séparément l'impact du board partiel** sur les performances : avec 1 inconnue, le moteur doit évaluer des mains à 6 cartes connues + 1 piochée, ce qui altère les chemins d'accès mémoire dans `best_hand`.
- Illustre la différence algorithmique flop vs turn dans le contexte du profiling.

**Itérations : 75 000**, justifié par la réduction du coût par iter (1 seule carte à tirer).

---

#### SC3 - 7s6s vs AdKd vs QcQh | Flop 8♠-9♦-2♠ | 50 000 itérations

**Contexte :** Le joueur 1 (7s6s) est en situation de tirage quinte ouverte sur un board connexe, avec également un tirage couleur pique. AdKd est un tirage couleur carreau premium (deux overcards + backdoor flush). QcQh a une paire haute mais est menacé par deux tirages directs.

**Intérêt pour le benchmark :**
- **Équité très distribuée** (55 % / 15 % / 30 % sur la baseline) : le moteur produit des résultats divergents de la distribution uniforme, ce qui valide la correction des calculs de probabilité.
- Board spécialement conçu pour maximiser les tirages simultanés → toutes les catégories de mains (flush, straight, two pair, brelan) sont représentées dans les résultats statistiques, couvrant l'ensemble des branches de `match hv.category`.
- Référence de complexité **moyenne**, identique à SC1 mais avec une distribution d'équités plus complexe.

**Itérations : 50 000**, cohérent avec SC1 pour comparaison directe.

---

#### SC4 - As5s vs KdKc vs QhJh vs Joueur inconnu | Flop 2♠-3♦-8♠ | 30 000 itérations

**Contexte :** Scénario **4 joueurs** incluant un joueur dont les cartes sont inconnues (mains aléatoires). As5s est en tirage couleur nut + tirage quinte basse (A-2-3-4-5 wheel). KdKc est favori avec une grosse paire. QhJh a deux overcards et un backdoor draw.

**Intérêt pour le benchmark :**
- **Cas le plus coûteux** : 4 évaluations `best_hand` par itération + tirage de 2 cartes supplémentaires pour le joueur inconnu + allocation d'un `Vec<[Card;2]>` de taille 4.
- Le joueur inconnu force le moteur à gérer des cartes manquantes (`None`) dans la boucle de simulation, testant le chemin `unwrap_or_else` à chaque itération.
- Représente un cas d'usage réel (un joueur dont on ne voit pas les cartes dans une application live).
- **Scénario de référence pour mesurer le coût marginal d'un joueur supplémentaire** : comparer SC1 (3 joueurs, ~50 k iters/s) à SC4 (4 joueurs, ~31 k iters/s) quantifie directement la complexité linéaire en nombre de joueurs.

**Itérations : 30 000**, réduit pour maintenir la durée d'exécution sous 1,8 s malgré le coût accru par itération.

---

#### SC5 - Ah5h vs JdJc vs 7c6c | Flop 4♥-5♠-6♥ | 50 000 itérations

**Contexte :** Board **très connecté et monotone partiel** (4♥-5♠-6♥). Ah5h a un tirage couleur cœur + une paire de 5. JdJc a une paire haute mais le board est dangereux. 7c6c a une quinte (6-7-8... non : 4-5-6-7-8), en réalité JcTc joue dans SC2 ; ici 7c6c construit une quinte directe sur le board 4-5-6.

**Intérêt pour le benchmark :**
- **Board le plus connecté** de la suite : 3 cartes consécutives avec deux cœurs → toutes les branches de quinte et de flush sont actives simultanément.
- Valide la détection de la wheel (A-2-3-4-5) en cas de tirage bas, testant le chemin `is_wheel` dans `evaluate_five`.
- Permet de **contrôler la cohérence des probabilités** : l'équité J3 (7c6c) doit être significativement plus haute sur ce board connecté que dans SC1, vérifiant la correction du moteur.

**Itérations : 50 000**, complexité identique à SC1 et SC3.

---

#### SC6 - AhKh vs QsQd | Flop J♥-T♥-2♣ | 500 000 itérations

**Contexte :** Duel **heads-up** (2 joueurs) sur un board offrant un tirage royal flush nut à AhKh (A♥-K♥-J♥-T♥, une carte du royal flush déjà posée). QsQd possède une overpair mais est exposé à un nombre exceptionnel de outs adverses (9 flush + quinte royale, 6 overcard outs).

**Intérêt pour le benchmark :**
- **Cas le moins coûteux par itération** : seulement 2 évaluations `best_hand`, pas de joueur inconnu, deck résiduel maximal → ~55 000 iters/s sur la baseline, soit la vitesse la plus haute de la suite.
- **500 000 itérations** : exploite cette rapidité pour atteindre une précision Monte Carlo de **±0,14 %** (erreur ≈ 1/√500 000), contre ±0,45 % pour les scénarios à 50 000 iters. Ce scénario sert de **référence de convergence** : les probabilités produites sont suffisamment précises pour valider les résultats des autres scénarios.
- **Cas de validation des optimisations** : le volume élevé d'itérations amplifie les gains d'optimisation et rend visibles des écarts de performance qui seraient noyés dans le bruit à 50 000 iters. C'est sur ce scénario que les speedups seront les plus nets et les plus fiables statistiquement.
- Exercice intensif du chemin `evaluate_five` pour les flush (J♥-T♥ sur le board → nombreuses mains flush) et les quintes (J-T connectés → straights fréquents), maximisant la couverture des branches de détection.

**Itérations : 500 000**, justifié par la faible charge par itération (2 joueurs, mains connues) et la nécessité d'une référence haute précision.

---

### 3.3 Récapitulatif et justification des itérations

| ID | Scénario | Joueurs | Board | Inconnues board | Itérations | Durée estimée (baseline) | Erreur MC |
|----|----------|---------|-------|-----------------|------------|--------------------------|-----------|
| SC1 | AA vs KK vs QJs | 3 | Flop | 2 | 50 000 | ~1,24 s | ±0,45 % |
| SC2 | AhKh vs 9c9d vs JcTc | 3 | Turn | 1 | 75 000 | ~1,10 s | ±0,37 % |
| SC3 | 7s6s vs AdKd vs QcQh | 3 | Flop | 2 | 50 000 | ~1,36 s | ±0,45 % |
| SC4 | As5s vs KdKc vs QhJh vs ??? | 4 | Flop | 2 | 30 000 | ~1,62 s | ±0,58 % |
| SC5 | Ah5h vs JdJc vs 7c6c | 3 | Flop | 2 | 50 000 | ~1,18 s | ±0,45 % |
| SC6 | AhKh vs QsQd *(haute précision)* | 2 | Flop | 2 | 500 000 | ~9,0 s | ±0,14 % |

**Règle d'adaptation des itérations :** SC1–SC5 suivent `N_iters ≈ T_cible / t_iter` avec `T_cible = 1,3 s` pour garantir des durées homogènes et des intervalles de confiance Hyperfine comparables. SC6 déroge volontairement à cette règle : son objectif n'est pas la vitesse mais la **précision de convergence**, d'où les 500 000 itérations (~9 s). Il n'est pas inclus dans les runs Hyperfine comparatifs mais sert de référence de validation des probabilités.

---

## 4. Optimisations

### 4.1 Optimisation 1 - FastEvaluator : Zéro Allocation sur le Hot Path

#### Diagnostic

Le profiling de la version naïve révèle une pression allocateur omniprésente sur le hot path. Pour chaque simulation, la fonction `evaluate_five` est appelée **21 fois** (C(7,5) combinaisons), et chaque appel effectue plusieurs allocations heap :

| Allocation | Type | Fréquence |
|---|---|---|
| `vals: Vec<u8>` | 5 rangs triés | 21× par itération |
| `counts: Vec<(u8, u8)>` | table de fréquences | 21× par itération |
| `tb_ranks: Vec<u8>` | tiebreak ordonné | 21× par itération |
| `HandValue { tiebreak: Vec<u8> }` | résultat d'évaluation | 21× par itération |
| `Vec<[Card; 2]> hole_cards` | mains des joueurs | 1× par itération |
| `Vec<Card> filled_board` | board complété | 1× par itération |
| `Vec<HandValue> hand_values` | classement par joueur | 1× par itération |
| `Vec<usize> winners` | liste des gagnants | 1× par itération |

Sur SC6 (500 000 itérations, 2 joueurs) : **≈ 11 millions d'allocations heap** par run, dont ~87 % proviennent d'`evaluate_five`.

**Hypothèse :** supprimer ces allocations en remplaçant `HandValue` par un `u32` encodé et les `Vec` par des tableaux stack → réduire la pression allocateur de ~87 % et éliminer les cache misses liés aux pointeurs indirects.

**Vérification :**
```bash
just bench sc6
```

#### Implémentation

**Fichier :** `src/eval/fast.rs`

##### Encodage u32 de la main

`HandValue { category: HandCategory, tiebreak: Vec<u8> }` est remplacé par un unique `u32` directement comparable (plus grand = meilleure main) :

```
bits [23:20]  catégorie (0 = HighCard … 9 = RoyalFlush)
bits [19:16]  tiebreak[0]  (rang le plus discriminant)
bits [15:12]  tiebreak[1]
bits [11:8]   tiebreak[2]
bits [7:4]    tiebreak[3]
bits [3:0]    tiebreak[4]
```

Chaque rang (2–14) tient dans 4 bits. La comparaison `u32 > u32` remplace l'`Ord` dérivé sur la struct, sans aucune indirection.

##### Buffers stack réutilisés

| Avant (naive) | Après (fast) |
|---|---|
| `Vec<u8>` de 5 éléments | `[u8; 5]` |
| `Vec<(u8, u8)>` de 13 éléments max | `[(u8, u8); 5]` |
| `Vec<HandValue>` par itération | `[u32; MAX_PLAYERS]` |
| `Vec<Card> filled_board` | `[Card; 7]` (board en positions 2–6) |
| `Vec<usize> winners` | deux passes O(n) sans allocation |

Le board est écrit une seule fois dans `seven[2..7]` par itération ; seules les positions `seven[0..2]` (cartes privées) changent entre joueurs.

#### Résultats mesurés

Mesures sur **Setup A** (AMD Ryzen 5 5600H, Arch Linux, rustc 1.98.1), hyperfine, 10 runs, warmup 3 pour SC6, 100 runs warmup 10 pour SC1–SC5.

| Scénario | naive iters/s | fast iters/s | Speedup |
|---|---|---|---|
| SC1 - 3 joueurs, flop, 50k | 138 300 | 309 900 | **×2.24** |
| SC2 - 3 joueurs, turn, 75k | 169 200 | 520 700 | **×3.08** |
| SC3 - 3 joueurs, flop, 50k | 130 800 | 300 600 | **×2.30** |
| SC4 - 4 joueurs, flop, 50k | 91 600 | 190 300 | **×2.08** |
| SC5 - 3 joueurs, flop, 30k | 134 700 | 282 600 | **×2.10** |
| SC6 - 2 joueurs, flop, 500k | 196 900 | 489 100 | **×2.42** |

##### SC6 - mesure hyperfine détaillée

```
Benchmark 1: naive sc6
  Time (mean ± σ):   2.546 s ±  0.023 s   [min: 2.523 s … max: 2.594 s]

Benchmark 2: fast  sc6
  Time (mean ± σ):   1.103 s ±  0.011 s   [min: 1.091 s … max: 1.121 s]

Summary: fast sc6 ran 2.31 ± 0.03 times faster than naive sc6
```

##### Analyse

- Le gain sur SC2 (×3.08) est supérieur aux autres scénarios de même taille car c'est le seul scénario **turn** : une seule carte inconnue au board signifie que le shuffle du deck (O(deck) = O(44)) pèse proportionnellement moins, laissant `evaluate_five` dominer la durée, et c'est précisément la fonction qu'on a optimisée.
- SC4 (×2.08) est le gain le plus modeste : avec 4 joueurs et un joueur inconnu, le tirage de cartes supplémentaires et la gestion des `Option<Card>` représentent une fraction non négligeable du temps, non couverte par l'optimisation courante.
- La réduction de la variance (σ passe de 23 ms à 11 ms sur SC6) confirme l'élimination de la pression GC : les pics de latence liés aux consolidations d'allocateur ont disparu.

##### Profil samply - fast sc6 (1 072 samples)

```
just profile sc6 fast
```

| Symbole | Self time |
|---|---|
| `poker::eval::fast::eval5` (corps) | **50.7 %** |
| `<Ordering as PartialEq>::eq` | 9.0 % |
| `insert_tail::<(u8,u8)>` (sort counts) | 7.9 % |
| `insert_tail::<u8>` (sort vals) | 6.3 % |
| `<u8 as PartialOrd>::lt` | 5.4 % |
| `ptr::copy::<Card>` (shuffle) | 3.5 % |
| `best7` | 2.0 % |
| `Rng::next` | 1.1 % |
| reste (`pack`, `simulate`, …) | 14.1 % |

**Réponse à l'hypothèse :** le shuffle (`Rng::next` + `ptr::copy` = ~4.6 %) est négligeable. Le goulot est `eval5` à 95 %, et au sein d'`eval5`, les deux `sort_unstable_by` (rangs `[u8; 5]` et fréquences `[(u8,u8)]`) représentent **~30 % du runtime total** (tris 14.2 % + comparaisons Ordering/`u8::lt` 14.4 %). L'insertion sort sur 5 éléments reste le principal vecteur de cycles perdus.

**Prochaine hypothèse :** remplacer les deux tris par des structures sans comparaison générique.
- `[u8; 5]` rangs → sorting network 5 éléments (9 compare-and-swap hardcodés, zéro closure, zéro branchement).
- `[(u8,u8)]` fréquences → tableau de buckets `[[u8;4]; 5]` indexé par fréquence, rempli en O(n) sans tri.

**Vérification :**
```bash
just bench sc6 fast
```

---

## 5. Gouvernance Technique IA

### 5.1 Fichier de gouvernance

Le fichier `CLAUDE.md` à la racine du projet constitue la **constitution technique de l'assistant IA**. Il s'agit du fichier de gouvernance lu automatiquement par Claude Code à chaque session, l'équivalent de `.cursorrules` ou `copilot-instructions.md` pour d'autres assistants. Son contenu structure quatre directives obligatoires.

### 5.2 Conformité aux quatre directives

#### Directive 1 - Rôle et posture système stricts

```
Tu es un ingénieur système Rust contraint par des métriques physiques réelles,
pas un générateur de code superficiel.
Toute proposition doit s'ancrer dans le comportement observable du matériel :
cycles CPU, lignes de cache, latences mémoire.
Tu ne génères aucun code sans avoir identifié l'hypothèse d'impact matériel
qu'il est censé valider.
```

L'IA est explicitement définie comme un ingénieur système contraint par la physique du matériel, non comme un générateur de code généraliste. Toute suggestion sans ancrage matériel mesurable est refusée.

#### Directive 2 - Contraintes négatives explicites (Gardes-fous)

Sept interdictions formelles couvrent les patterns non performants spécifiques à Rust sur le Hot Path :

| Interdit | Justification |
|----------|---------------|
| `Box`, `Vec::new`, `String::new` sans justification chiffrée | Allocation tas = pression sur l'allocateur, latence imprévisible |
| `clone()` là où une référence suffit | Copie mémoire inutile |
| `collect()` dans une boucle critique | Allocation répétée par itération |
| `dyn Trait` sur le Hot Path | Vtable dispatch = branch imprévisible, inhibe l'inlining |
| `unwrap()` / `expect()` sur chemin critique | Masque une panique latente |
| Conversions `String ↔ &str ↔ Vec<u8>` superflues | Copies + réallocations invisibles |
| `Mutex` non borné là où `RwLock` ou atomiques suffisent | Contention inutile, cache-line ping-pong |

#### Directive 3 - Principe de justification empirique

Chaque proposition d'optimisation est contrainte à respecter le format :

```
Hypothèse : <impact attendu sur le matériel>
Vérification : <commande exacte de profiling>
```

Exemples de commandes acceptées dans ce projet :

```bash
# Mesure statistique de durée
just bench-sc1

# Flamegraph interactif
just profile

# Compteurs matériels bas niveau
perf stat -e cache-misses,cache-references,instructions,cycles ./target/release/bench sc1
```

Aucune optimisation n'est proposée sans la commande de vérification associée, ce principe garantit que chaque changement est mesurable avant et après.

#### Directive 4 - Formatage compact et impératif

- Réponses formulées sous forme d'injonctions courtes et vérifiables.
- Zéro verbiage introductif ("Il serait judicieux de…", "On pourrait envisager…").
- Chaque suggestion suit le triptyque : **action → hypothèse matérielle → commande de mesure**.
- Les résultats chiffrés (iters/s, ms, cache-misses) priment sur les explications théoriques.

### 5.3 Intégration dans le workflow

La constitution est renforcée par deux mécanismes automatiques :

| Mécanisme | Rôle |
|-----------|------|
| Hooks `PostToolUse` dans `.claude/settings.json` | Exécute `just format` et `just lint` après chaque édition de fichier `.rs`, garantit qu'aucun code non formaté ou contenant des warnings clippy n'est produit |
| CI GitHub Actions (`.github/workflows/ci.yml`) | Rejoue `cargo fmt --check` et `cargo clippy -D warnings` sur chaque push/PR, les gardes-fous de la constitution sont vérifiés indépendamment de l'assistant |


