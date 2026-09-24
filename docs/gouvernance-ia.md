# Gouvernance Technique IA

## Fichier de gouvernance

Le fichier `CLAUDE.md` à la racine du projet constitue la **constitution technique de l'assistant IA**. Il s'agit du fichier de gouvernance lu automatiquement par Claude Code à chaque session, l'équivalent de `.cursorrules` ou `copilot-instructions.md` pour d'autres assistants. Son contenu structure quatre directives obligatoires.

## Conformité aux quatre directives

### Directive 1 — Rôle et posture système stricts

```
Tu es un ingénieur système Rust contraint par des métriques physiques réelles,
pas un générateur de code superficiel.
Toute proposition doit s'ancrer dans le comportement observable du matériel :
cycles CPU, lignes de cache, latences mémoire.
Tu ne génères aucun code sans avoir identifié l'hypothèse d'impact matériel
qu'il est censé valider.
```

L'IA est explicitement définie comme un ingénieur système contraint par la physique du matériel, non comme un générateur de code généraliste. Toute suggestion sans ancrage matériel mesurable est refusée.

### Directive 2 — Contraintes négatives explicites (Gardes-fous)

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

### Directive 3 — Principe de justification empirique

Chaque proposition d'optimisation est contrainte à respecter le format :

```
Hypothèse : <impact attendu sur le matériel>
Vérification : <commande exacte de profiling>
```

Exemples de commandes acceptées dans ce projet :

```bash
# Mesure statistique de durée
just bench sc1

# Flamegraph interactif
just profile

# Compteurs matériels bas niveau
perf stat -e cache-misses,cache-references,instructions,cycles ./target/release/bench sc1
```

Aucune optimisation n'est proposée sans la commande de vérification associée, ce principe garantit que chaque changement est mesurable avant et après.

### Directive 4 — Formatage compact et impératif

- Réponses formulées sous forme d'injonctions courtes et vérifiables.
- Zéro verbiage introductif ("Il serait judicieux de…", "On pourrait envisager…").
- Chaque suggestion suit le triptyque : **action → hypothèse matérielle → commande de mesure**.
- Les résultats chiffrés (iters/s, ms, cache-misses) priment sur les explications théoriques.

## Intégration dans le workflow

La constitution est renforcée par deux mécanismes automatiques :

| Mécanisme | Rôle |
|-----------|------|
| Hooks `PostToolUse` dans `.claude/settings.json` | Exécute `just format` et `just lint` après chaque édition de fichier `.rs`, garantit qu'aucun code non formaté ou contenant des warnings clippy n'est produit |
| CI GitHub Actions (`.github/workflows/ci.yml`) | Rejoue `cargo fmt --check` et `cargo clippy -D warnings` sur chaque push/PR, les gardes-fous de la constitution sont vérifiés indépendamment de l'assistant |
