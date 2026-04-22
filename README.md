[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/b5MRUqco)

# SysWatch — Moniteur Systeme en Reseau

TP Integral Rust — Genie Logiciel L4 — 2025-2026

## Description

SysWatch est un serveur TCP interactif qui collecte les metriques systeme reelles (CPU, RAM, processus) et repond aux commandes de n'importe quel client connecte. Le projet comprend un client maitre (`master.rs`) pour piloter et controler plusieurs machines a distance sur le meme reseau.

## Prerequis

- Rust installe (`rustc --version`, `cargo --version`)
- Windows (les commandes shutdown/reboot/exec utilisent la syntaxe Windows)

## Installation

```bash
cargo build
```

## Lancement

### Serveur (agent sur chaque machine)

```bash
cargo run --bin syswatch
```

Le serveur demarre sur le port **7878** et attend les connexions.

### Client maitre (PC administrateur)

```bash
cargo run --bin syswatch-master
```

Interface de controle pour gerer et controler plusieurs agents SysWatch sur le reseau.

## Connexion au serveur

```bash
telnet localhost 7878
```

Token d'authentification : `ENSPD2026`

## Commandes disponibles

| Commande | Description |
|----------|-------------|
| `cpu` | Affiche l'utilisation CPU avec barre ASCII |
| `mem` | Affiche la memoire RAM avec barre de progression |
| `ps` | Liste le top 5 des processus par CPU |
| `all` | Vue complete (CPU + RAM + processus) |
| `exec <cmd>` | Executer une commande shell sur la machine distante |
| `kill <pid>` | Tuer un processus par son PID |
| `lock` | Verrouiller le poste de travail |
| `users` | Lister les utilisateurs connectes |
| `msg <texte>` | Afficher un message sur la machine cible |
| `install <pkg>` | Installer un logiciel via winget |
| `shutdown` | Eteindre la machine (5s delai) |
| `reboot` | Redemarrer la machine (5s delai) |
| `abort` | Annuler un shutdown/reboot en cours |
| `help` | Affiche l'aide |
| `quit` | Fermer la connexion |

## Controle a distance (Master)

Le client maitre (`syswatch-master`) permet a un administrateur de controler plusieurs machines sur le meme reseau :

1. **Scanner le reseau** : `scan` pour lister les machines configurees et leur statut
2. **Cibler une machine** : `select <nom>` pour choisir une machine specifique
3. **Commander toutes les machines** : `all <cmd>` pour envoyer une commande a toutes les machines en ligne
4. **Executer des commandes** : `exec <cmd>` pour lancer une commande shell a distance
5. **Gerer les processus** : `kill <pid>` pour arreter un processus
6. **Verrouiller** : `lock` pour verrouiller le poste a distance

## Structure du projet

```
syswatch/
+-- Cargo.toml          # Dependances : sysinfo 0.30, chrono 0.4
+-- src/
|   +-- main.rs         # Serveur TCP (agent) + tests unitaires et integration
|   +-- master.rs       # Client maitre (administrateur)
+-- syswatch.log        # Journal des connexions (cree au runtime)
```

## Tests

Lancer tous les tests :

```bash
cargo test --bin syswatch
```

Les tests couvrent :
- Affichage des structures (CpuInfo, MemInfo, ProcessInfo, SystemSnapshot)
- Formatage de toutes les commandes (cpu, mem, ps, all, help, quit, exec, kill)
- Collecte systeme reelle (verifie CPU > 0, RAM > 0, top 5 processus)
- Tri des processus par utilisation CPU
- Erreur custom SysWatchError
- Integration TCP : authentification + commandes + deconnexion
- Integration TCP : rejet de mauvais token

## Etapes du TP

### Etape 1 — Modelisation des donnees
- Structures : `CpuInfo`, `MemInfo`, `ProcessInfo`, `SystemSnapshot`
- Implementation du trait `fmt::Display` pour chaque structure
- Utilisation de `derive(Debug, Clone)` et `Vec<T>`

### Etape 2 — Collecte reelle et gestion d'erreurs
- Utilisation de la crate `sysinfo` pour lire CPU, RAM et processus
- Enum d'erreur personnalisee `SysWatchError`
- Fonction `collect_snapshot()` retournant `Result<SystemSnapshot, SysWatchError>`
- Tri des processus par CPU avec `.sort_by()` et troncature au top 5

### Etape 3 — Formatage des reponses reseau
- Pattern matching exhaustif sur les commandes
- Barres de progression ASCII avec iterateurs
- Commandes de controle : `exec`, `kill`, `lock`, `users`, `shutdown`, `reboot`, `abort`, `msg`, `install`

### Etape 4 — Serveur TCP multi-threade
- `TcpListener` sur le port 7878
- Chaque client gere dans un `thread::spawn` separe
- Donnees partagees via `Arc<Mutex<SystemSnapshot>>`
- Thread dedie au rafraichissement automatique toutes les 5 secondes
- Authentification par token avant l'acces aux commandes

### Etape 5 — Journalisation fichier (Bonus)
- Fonction `log_event()` utilisant `OpenOptions` en mode append
- Enregistrement de toutes les connexions et commandes avec horodatage via `chrono`
- Fichier de log : `syswatch.log`

### Etape 6 — Controle a distance (Master)
- Client maitre pour piloter plusieurs machines
- Scan du reseau, selection de machines, commandes broadcast
- Execution de commandes shell a distance (`exec`)
- Gestion de processus a distance (`kill`)
- Verrouillage de poste a distance (`lock`)

