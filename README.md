[![Review Assignment Due Date](https://classroom.github.com/assets/deadline-readme-button-22041afd0340ce965d47ae6ef1cefeee28c7c493a6346c4f15d667ab976d596c.svg)](https://classroom.github.com/a/b5MRUqco)

# SysWatch — Moniteur Systeme en Reseau

TP Integral Rust — Genie Logiciel L4 — 2025-2026

## Description

SysWatch est un serveur TCP interactif qui collecte les metriques systeme reelles (CPU, RAM, processus) et repond aux commandes de n'importe quel client connecte. Le projet comprend un client maitre (`master.rs`) pour piloter et controler plusieurs machines a distance sur le meme reseau.

## Prerequis

- Rust installe (`rustc --version`, `cargo --version`)
- Windows (les commandes shutdown/reboot/exec utilisent la syntaxe Windows)
- Executer l'agent en **administrateur** pour que la regle firewall TCP soit creee automatiquement

## Installation

```bash
cargo build
```

## Lancement

### Serveur (agent sur chaque machine)

```bash
cargo run --bin syswatch
```

Le serveur demarre sur le port **7878** (TCP) et attend les connexions. Il est automatiquement detectable par le master via un scan du sous-reseau.

### Client maitre (PC administrateur)

```bash
cargo run --bin syswatch-master
```

Interface de controle pour decouvrir et piloter les agents SysWatch sur le reseau local.

## Authentification

Token d'authentification : `ENSPD2026`

L'authentification est geree automatiquement par le master lors de la connexion a un agent.

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

Le client maitre (`syswatch-master`) decouvre automatiquement les agents presents sur le reseau local via un **scan TCP du sous-reseau** — aucune IP a configurer manuellement.

### Workflow

1. **Decouvrir les agents** : `scan` — scanne les 254 adresses du sous-reseau local sur le port 7878 et identifie les agents SysWatch
2. **Lister les agents connus** : `list` — affiche toutes les machines decouvertes
3. **Ajouter manuellement** : `add <nom> <ip>` — ajouter une machine par IP si elle est hors du sous-reseau
4. **Cibler une machine** : `select <nom>` — ouvre une session persistante vers l'agent choisi
5. **Envoyer des commandes** : `cpu`, `mem`, `ps`, `exec <cmd>`, `kill <pid>`, `lock`, etc.
6. **Commander toutes les machines** : `all <cmd>` — envoie une commande a tous les agents decouverts
7. **Fermer la session** : `disconnect` — deconnecte de l'agent actif

### Commandes du master

| Commande | Description |
|----------|-------------|
| `scan` | Scanner le sous-reseau pour trouver les agents |
| `list` | Afficher les agents connus |
| `add <nom> <ip>` | Ajouter une machine manuellement |
| `select <nom>` | Se connecter a un agent (session persistante) |
| `all <cmd>` | Envoyer une commande a tous les agents |
| `disconnect` | Fermer la session active |
| `help` | Afficher le menu |
| `quit` | Quitter le master |

### Protocole de decouverte

- Le master detecte l'IP locale et scanne tout le sous-reseau /24 (254 adresses) en parallele
- Chaque IP est testee sur le port **7878** avec un timeout de 400ms
- Si un serveur repond avec le prompt `TOKEN:`, il est identifie comme agent SysWatch
- Le hostname est recupere automatiquement via la commande `exec hostname`
- Les deux machines doivent etre sur le **meme sous-reseau local** pour la decouverte automatique
- Pour les machines hors du sous-reseau, utiliser `add <nom> <ip>`

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
- Decouverte automatique des agents via scan TCP du sous-reseau /24 (254 IPs en parallele)
- Aucune IP a configurer manuellement — le master scanne et identifie les agents par leur prompt `TOKEN:`
- Ajout manuel possible via `add <nom> <ip>` pour les machines hors du sous-reseau
- Sessions persistantes avec reconnexion automatique
- Commandes : `scan`, `list`, `add`, `select`, `all`, `disconnect`
- Execution de commandes shell a distance (`exec`)
- Gestion de processus a distance (`kill`)
- Verrouillage de poste a distance (`lock`)

