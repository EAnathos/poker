# Mise en place de l'environnement

## Installation

```bash
cargo install cargo-binstall  # à faire en premier
cargo binstall just
just install-tools             # Hyperfine + Samply
just build                     # compile le binaire bench en release
```

## Activer les symboles de debug en release

Requis pour que samply affiche les noms de fonctions (et non des adresses hexadécimales). Ajouter dans `Cargo.toml` :

```toml
[profile.release]
debug = 1        # line tables seulement — suffit pour samply
strip = "none"   # garde les symboles — obligatoire
```

## Référence des commandes

→ [`docs/commandes.md`](commandes.md)
