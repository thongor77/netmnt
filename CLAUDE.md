# CLAUDE.md — netmnt

> Projet important. Doc : README.md + ce fichier + `docs/`.

---

## Description

Outil Linux/KDE pour monter des partages réseau (SMB/CIFS d'abord) depuis le menu
contextuel de Dolphin, avec trois actions : `Mount`, `Mount as…` (autres
credentials), `Mount (persistent)`. Le montage réel est fait par un service Rust
privilégié.

## Stack

- Rust (workspace, edition 2021)
- `zbus` (D-Bus), `tokio`, `clap`, `serde`, `tracing`
- systemd (cycle de vie du daemon + units `.mount` persistantes)
- polkit (autorisation), KDE ServiceMenus (intégration Dolphin)
- Backend : `mount.cifs` (cifs-utils), KWallet pour les credentials

## État actuel

Squelette initial (25/06/2026). Contrat D-Bus, structure du workspace et fichiers
d'intégration système (dbus/polkit/systemd/servicemenu) en place. Les méthodes du
daemon sont des stubs. Détail : `docs/Roadmap.md`.

## Lancer le projet

```sh
cargo build           # compile les 3 crates
cargo run -p netmntd  # daemon (échoue à claim org.netmnt sans la conf D-Bus installée)
cargo run -p netmnt -- mount smb://lab1.local/isos
```

Installation système (à terme) : conf D-Bus dans `/usr/share/dbus-1/system.d/`,
policy polkit dans `/usr/share/polkit-1/actions/`, unit dans
`/usr/lib/systemd/system/`, servicemenu dans `~/.local/share/kio/servicemenus/`.

## Architecture

3 crates : `netmnt` (CLI non-privilégié, appelé par le servicemenu) →
D-Bus système → `netmntd` (daemon privilégié, owner de `org.netmnt`) →
`mount.cifs` / unit systemd. Types partagés dans `netmnt-common`.
Détail : `docs/Architecture.md`. Choix techniques : `docs/Decisions-Techniques.md`.

## Décisions techniques

Voir `docs/Decisions-Techniques.md` (notamment : pourquoi un daemon D-Bus + polkit
plutôt que pkexec/setuid ou gio mount ; pourquoi systemd `.mount` plutôt que fstab ;
alternatives existantes écartées — Smb4K, kio-fuse).

## Conventions spécifiques

- Aucun secret (mot de passe, credential) ne transite par argv ni n'est loggé.
  Les credentials persistants vont dans KWallet, jamais dans un fichier versionné.
- Chaque méthode D-Bus mutante doit être gardée par une action polkit dédiée.
- Reste aligné sur `META/Standards.md` pour le reste (commits, langue, etc.).

## Inconnues critiques (à valider par prototype)

1. Récupération du mot de passe depuis KWallet côté daemon (D-Bus Secret Service).
2. Génération + activation d'une unit systemd `.mount` à chaud sans casser l'idempotence.
3. Récupération fiable du polkit subject (uid/pid appelant) via zbus.
