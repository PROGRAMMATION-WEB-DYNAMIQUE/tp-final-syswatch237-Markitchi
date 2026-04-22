[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/b5MRUqco)

# SysWatch — Moniteur Système en Réseau

TP Intégral Rust — Génie Logiciel L4 — 2025-2026

## Description

SysWatch est un serveur TCP interactif qui collecte les métriques système réelles (CPU, RAM, processus) et répond aux commandes de n'importe quel client connecté. Le projet comprend aussi un client maître (`master.rs`) pour piloter plusieurs machines à distance.

## Prérequis

- Rust installé (`rustc --version`, `cargo --version`)
- Windows (les commandes shutdown/reboot utilisent la syntaxe Windows)

## Installation

```bash
cargo build
```

## Lancement

### Serveur (agent sur chaque machine)

```bash
cargo run --bin syswatch
```

Le serveur démarre sur le port **7878** et attend les connexions.

### Client maître (optionnel)

```bash
cargo run --bin syswatch-master
```

Interface de contrôle pour gérer plusieurs agents SysWatch sur le réseau.

## Connexion au serveur

```bash
telnet localhost 7878
```

Token d'authentification : `ENSPD2026`

## Commandes disponibles

| Commande | Description |
|----------|-------------|
| `cpu` | Affiche l'utilisation CPU avec barre ASCII |
| `mem` | Affiche la mémoire RAM avec barre de progression |
| `ps` | Liste le top 5 des processus par CPU |
| `all` | Vue complète (CPU + RAM + processus) |
| `help` | Affiche l'aide |
| `quit` | Fermer la connexion |
| `shutdown` | Éteindre la machine (5s délai) |
| `reboot` | Redémarrer la machine (5s délai) |
| `abort` | Annuler un shutdown/reboot en cours |
| `msg <texte>` | Afficher un message sur la machine cible |
| `install <pkg>` | Installer un logiciel via winget |

## Structure du projet

```
syswatch/
├── Cargo.toml          # Dépendances : sysinfo 0.30, chrono 0.4
├── src/
│   ├── main.rs         # Serveur TCP (agent)
│   └── master.rs       # Client maître (professeur)
└── syswatch.log        # Journal des connexions (créé au runtime)
```

## Étapes du TP

### Étape 1 — Modélisation des données
- Structures : `CpuInfo`, `MemInfo`, `ProcessInfo`, `SystemSnapshot`
- Implémentation du trait `fmt::Display` pour chaque structure
- Utilisation de `derive(Debug, Clone)` et `Vec<T>`

### Étape 2 — Collecte réelle et gestion d'erreurs
- Utilisation de la crate `sysinfo` pour lire CPU, RAM et processus
- Enum d'erreur personnalisée `SysWatchError`
- Fonction `collect_snapshot()` retournant `Result<SystemSnapshot, SysWatchError>`
- Tri des processus par CPU avec `.sort_by()` et troncature au top 5

### Étape 3 — Formatage des réponses réseau
- Pattern matching exhaustif sur les commandes (`cpu`, `mem`, `ps`, `all`, `help`, `quit`)
- Barres de progression ASCII avec itérateurs
- Commandes supplémentaires : `shutdown`, `reboot`, `abort`, `msg`, `install`

### Étape 4 — Serveur TCP multi-threadé
- `TcpListener` sur le port 7878
- Chaque client géré dans un `thread::spawn` séparé
- Données partagées via `Arc<Mutex<SystemSnapshot>>`
- Thread dédié au rafraîchissement automatique toutes les 5 secondes
- Authentification par token avant l'accès aux commandes

### Étape 5 — Journalisation fichier (Bonus)
- Fonction `log_event()` utilisant `OpenOptions` en mode append
- Enregistrement de toutes les connexions et commandes avec horodatage via `chrono`
- Fichier de log : `syswatch.log`
