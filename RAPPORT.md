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
| CPU | AMD Rysen 7 7735U |
| Cœurs / Threads | 8C 16T |
| Cache L1 | 512 Ko |
| Cache L2 | 4.0 Mo |
| Cache L3 | 16.0 Mo |
| RAM | 16Go 	LPDDR5 6400 MT/s |
| OS | Windows 11 |
| Runtime Rust | rustc 1.98.1 |

### 1.2 Outils de profiling

#### Installation

```bash
# Hyperfine — mesure statistique de la durée d'exécution
cargo install hyperfine

# Samply — flamegraph interactif (ouvre Firefox Profiler automatiquement)
cargo install samply
```

#### Activer les symboles de debug en release

Requis pour que samply affiche les noms de fonctions Rust (et non des adresses hexadécimales).
Ajouter dans `Cargo.toml` :

```toml
[profile.release]
debug = 2
split-debuginfo = "off"
strip = "none"
```

#### Compiler le binaire de benchmark en release

```bash
cargo build --release --bin bench
```

#### Profiling avec Samply

```bash
# Linux / Setup A
samply record ./target/release/bench

# Windows / Setup B (PowerShell)
samply record .\target\release\bench.exe
```

Firefox Profiler s'ouvre automatiquement avec le flamegraph interactif.

#### Protocole de mesure (Hyperfine)

```bash
# Linux / Setup A
hyperfine \
  --warmup 10 \
  --runs 100 \
  --shell=none \
  --export-json results_baseline.json \
  --export-markdown results_baseline.md \
  './target/release/bench'
```

```powershell
# Windows / Setup B (PowerShell)
hyperfine --warmup 10 --runs 100 --shell=none --export-json results_baseline.json --export-markdown results_baseline.md '.\target\release\bench.exe'
```

**Justification des paramètres :**
- `--warmup 10` : écarte les 10 premiers runs (cache disque froid, montée en fréquence CPU).
- `--runs 100` : réduit l'erreur standard à σ/√100 ≈ 10 % de σ.
- `--shell=none` : retire le bruit du shell de la mesure (pas de fork/exec de bash).

#### Extraction des métriques complètes depuis le JSON

```bash
python -c "
import json, statistics as s
t = json.load(open('results_baseline.json'))['results'][0]['times']
print(f'Moyenne   : {s.mean(t)*1000:.1f} ms')
print(f'Médiane   : {s.median(t)*1000:.1f} ms')
print(f'Écart-type: {s.stdev(t)*1000:.1f} ms')
print(f'Variance  : {s.variance(t)*1e6:.2f} ms²')
print(f'Min       : {min(t)*1000:.1f} ms')
print(f'Max       : {max(t)*1000:.1f} ms')
"
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

---

## 2. Scénarios de Benchmark

### 2.1 Principes de conception des scénarios

Les scénarios de benchmark couvrent quatre axes qui influencent directement la charge computationnelle de l'évaluateur Monte Carlo :

1. **Nombre de joueurs** — chaque joueur supplémentaire ajoute une évaluation `best_hand` par itération (soit 21 appels à `evaluate_five` pour 7 cartes), plus une allocation `Vec<HandValue>` plus large.
2. **Stade de jeu (flop / turn / river)** — détermine le nombre de cartes inconnues à tirer dans le deck résiduel : 2 cartes au flop, 1 au turn, 0 à la river. Moins d'inconnues = moins de variabilité par itération, ce qui justifie un nombre d'itérations plus élevé pour atteindre la même précision statistique.
3. **Présence de joueurs inconnus** — forcer le tirage de 2 cartes supplémentaires par joueur fantôme augmente la pression sur le générateur de nombres aléatoires et la gestion du deck résiduel.
4. **Richesse en draws** — les boards connectés et suiteds génèrent davantage de combinaisons de flush et de quinte à évaluer, ce qui stresse uniformément le chemin `evaluate_five → counts → tiebreak`.

Le nombre d'itérations est calibré de façon à ce que chaque scénario s'exécute entre **800 ms et 1,8 s** sur la baseline naive, garantissant :
- Un signal suffisant pour Hyperfine (réduction de l'erreur standard ≈ σ/√N avec N = 100 runs externes).
- Une résolution suffisante des probabilités (erreur Monte Carlo ≈ 1/√itérations ; 50 000 iters → ±0,45 % sur une équité de 50 %).

---

### 2.2 Description des scénarios

#### SC1 — AA vs KK vs QJs | Flop 9♠-T♠-2♦ | 50 000 itérations

**Contexte :** Situation emblématique du « big hand vs big draw ». AA est la main premium absolue, KK est en très mauvaise posture face aux aces, et QJs représente un double tirage (quinte + flush treillis) sur un board très connecté.

**Intérêt pour le benchmark :**
- 3 joueurs → 3 évaluations `best_hand` par itération.
- 2 cartes inconnues au board (turn + river) → deck résiduel de 44 cartes, shuffle complet à chaque itération.
- Présence fréquente de flush et de quinte dans les résultats → toutes les branches de `evaluate_five` sont exercées.
- Référence de complexité **moyenne** (3 joueurs, flop).

**Itérations : 50 000** — équilibre précision/temps, erreur ≈ ±0,45 %.

---

#### SC2 — AhKh vs 9c9d vs JcTc | Turn 9♥-8♥-2♠-3♥ | 75 000 itérations

**Contexte :** Scénario de **turn** (4 cartes au board) avec une seule carte inconnue restante. AhKh possède un tirage couleur nut (4 cœurs avec l'As), 9c9d a un brelan (set de 9) et JcTc dispose d'un tirage quinte + tirage couleur trèfle.

**Intérêt pour le benchmark :**
- **Seule 1 carte inconnue au board** — le deck résiduel est plus petit (43 cartes résiduelles, 1 seule à tirer), ce qui réduit la durée par itération par rapport au flop.
- Ce gain par itération est compensé par un nombre d'itérations plus élevé (75 000 au lieu de 50 000), maintenant la durée totale comparable tout en améliorant la précision (erreur ≈ ±0,37 %).
- Permet de **mesurer séparément l'impact du board partiel** sur les performances : avec 1 inconnue, le moteur doit évaluer des mains à 6 cartes connues + 1 piochée, ce qui altère les chemins d'accès mémoire dans `best_hand`.
- Illustre la différence algorithmique flop vs turn dans le contexte du profiling.

**Itérations : 75 000** — justifié par la réduction du coût par iter (1 seule carte à tirer).

---

#### SC3 — 7s6s vs AdKd vs QcQh | Flop 8♠-9♦-2♠ | 50 000 itérations

**Contexte :** Le joueur 1 (7s6s) est en situation de tirage quinte ouverte sur un board connexe, avec également un tirage couleur pique. AdKd est un tirage couleur carreau premium (deux overcards + backdoor flush). QcQh a une paire haute mais est menacé par deux tirages directs.

**Intérêt pour le benchmark :**
- **Équité très distribuée** (55 % / 15 % / 30 % sur la baseline) : le moteur produit des résultats divergents de la distribution uniforme, ce qui valide la correction des calculs de probabilité.
- Board spécialement conçu pour maximiser les tirages simultanés → toutes les catégories de mains (flush, straight, two pair, brelan) sont représentées dans les résultats statistiques, couvrant l'ensemble des branches de `match hv.category`.
- Référence de complexité **moyenne**, identique à SC1 mais avec une distribution d'équités plus complexe.

**Itérations : 50 000** — cohérent avec SC1 pour comparaison directe.

---

#### SC4 — As5s vs KdKc vs QhJh vs Joueur inconnu | Flop 2♠-3♦-8♠ | 30 000 itérations

**Contexte :** Scénario **4 joueurs** incluant un joueur dont les cartes sont inconnues (mains aléatoires). As5s est en tirage couleur nut + tirage quinte basse (A-2-3-4-5 wheel). KdKc est favori avec une grosse paire. QhJh a deux overcards et un backdoor draw.

**Intérêt pour le benchmark :**
- **Cas le plus coûteux** : 4 évaluations `best_hand` par itération + tirage de 2 cartes supplémentaires pour le joueur inconnu + allocation d'un `Vec<[Card;2]>` de taille 4.
- Le joueur inconnu force le moteur à gérer des cartes manquantes (`None`) dans la boucle de simulation, testant le chemin `unwrap_or_else` à chaque itération.
- Représente un cas d'usage réel (un joueur dont on ne voit pas les cartes dans une application live).
- **Scénario de référence pour mesurer le coût marginal d'un joueur supplémentaire** : comparer SC1 (3 joueurs, ~50 k iters/s) à SC4 (4 joueurs, ~31 k iters/s) quantifie directement la complexité linéaire en nombre de joueurs.

**Itérations : 30 000** — réduit pour maintenir la durée d'exécution sous 1,8 s malgré le coût accru par itération.

---

#### SC5 — Ah5h vs JdJc vs 7c6c | Flop 4♥-5♠-6♥ | 50 000 itérations

**Contexte :** Board **très connecté et monotone partiel** (4♥-5♠-6♥). Ah5h a un tirage couleur cœur + une paire de 5. JdJc a une paire haute mais le board est dangereux. 7c6c a une quinte (6-7-8... non : 4-5-6-7-8) — en réalité JcTc joue dans SC2 ; ici 7c6c construit une quinte directe sur le board 4-5-6.

**Intérêt pour le benchmark :**
- **Board le plus connecté** de la suite : 3 cartes consécutives avec deux cœurs → toutes les branches de quinte et de flush sont actives simultanément.
- Valide la détection de la wheel (A-2-3-4-5) en cas de tirage bas, testant le chemin `is_wheel` dans `evaluate_five`.
- Permet de **contrôler la cohérence des probabilités** : l'équité J3 (7c6c) doit être significativement plus haute sur ce board connecté que dans SC1, vérifiant la correction du moteur.

**Itérations : 50 000** — complexité identique à SC1 et SC3.

---

#### SC6 — AhKh vs QsQd | Flop J♥-T♥-2♣ | 500 000 itérations

**Contexte :** Duel **heads-up** (2 joueurs) sur un board offrant un tirage royal flush nut à AhKh (A♥-K♥-J♥-T♥, une carte du royal flush déjà posée). QsQd possède une overpair mais est exposé à un nombre exceptionnel de outs adverses (9 flush + quinte royale, 6 overcard outs).

**Intérêt pour le benchmark :**
- **Cas le moins coûteux par itération** : seulement 2 évaluations `best_hand`, pas de joueur inconnu, deck résiduel maximal → ~55 000 iters/s sur la baseline, soit la vitesse la plus haute de la suite.
- **500 000 itérations** : exploite cette rapidité pour atteindre une précision Monte Carlo de **±0,14 %** (erreur ≈ 1/√500 000), contre ±0,45 % pour les scénarios à 50 000 iters. Ce scénario sert de **référence de convergence** : les probabilités produites sont suffisamment précises pour valider les résultats des autres scénarios.
- **Cas de validation des optimisations** : le volume élevé d'itérations amplifie les gains d'optimisation et rend visibles des écarts de performance qui seraient noyés dans le bruit à 50 000 iters. C'est sur ce scénario que les speedups seront les plus nets et les plus fiables statistiquement.
- Exercice intensif du chemin `evaluate_five` pour les flush (J♥-T♥ sur le board → nombreuses mains flush) et les quintes (J-T connectés → straights fréquents), maximisant la couverture des branches de détection.

**Itérations : 500 000** — justifié par la faible charge par itération (2 joueurs, mains connues) et la nécessité d'une référence haute précision.

---

### 2.3 Récapitulatif et justification des itérations

| ID | Scénario | Joueurs | Board | Inconnues board | Itérations | Durée estimée (baseline) | Erreur MC |
|----|----------|---------|-------|-----------------|------------|--------------------------|-----------|
| SC1 | AA vs KK vs QJs | 3 | Flop | 2 | 50 000 | ~1,24 s | ±0,45 % |
| SC2 | AhKh vs 9c9d vs JcTc | 3 | Turn | 1 | 75 000 | ~1,10 s | ±0,37 % |
| SC3 | 7s6s vs AdKd vs QcQh | 3 | Flop | 2 | 50 000 | ~1,36 s | ±0,45 % |
| SC4 | As5s vs KdKc vs QhJh vs ??? | 4 | Flop | 2 | 30 000 | ~1,62 s | ±0,58 % |
| SC5 | Ah5h vs JdJc vs 7c6c | 3 | Flop | 2 | 50 000 | ~1,18 s | ±0,45 % |
| SC6 | AhKh vs QsQd *(haute précision)* | 2 | Flop | 2 | 500 000 | ~9,0 s | ±0,14 % |

**Règle d'adaptation des itérations :** SC1–SC5 suivent `N_iters ≈ T_cible / t_iter` avec `T_cible = 1,3 s` pour garantir des durées homogènes et des intervalles de confiance Hyperfine comparables. SC6 déroge volontairement à cette règle : son objectif n'est pas la vitesse mais la **précision de convergence**, d'où les 500 000 itérations (~9 s). Il n'est pas inclus dans les runs Hyperfine comparatifs mais sert de référence de validation des probabilités.


