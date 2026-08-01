# CLAUDE.md — netmnt

> Projet important. Doc : README.md + ce fichier + `docs/`.

---

## Description

Outil Linux/KDE pour monter des partages réseau (SMB/CIFS et NFS) depuis le menu
contextuel de Dolphin, avec trois actions : `Mount`, `Mount as…` (autres
credentials, SMB seulement), `Mount (persistent)`. Le montage réel est fait par
un service Rust privilégié.

## Stack

- Rust (workspace, edition 2021)
- `zbus` (D-Bus), `tokio`, `clap`, `serde`, `tracing`
- systemd (cycle de vie du daemon + units `.mount` persistantes)
- polkit (autorisation), KDE ServiceMenus (intégration Dolphin)
- Backend : `mount.cifs` (cifs-utils) et `mount.nfs` (nfs-utils) ; KWallet pour
  les credentials SMB (NFS n'en a pas — accès géré par l'ACL d'export serveur)

## État actuel

**Fonctionnel (01/08/2026)** — SMB testé en réel sur le NAS `lab1.local`,
validé après reboot ; NFS testé en réel sur un export Synology
(`192.168.1.64:/volume1/testing`), validation post-reboot en attente :

- **Mount** (session, invité) : OK, sans prompt (polkit `allow_active=yes`).
- **Mount as…** (`--ask`) : prompt kdialog/tty + KWallet (lecture/écriture), mot
  de passe hors argv. OK sur partage authentifié.
- **Mount persistant** (`--persistent`) : génère une unit systemd `.mount` +
  `enable --now` ; credentials dans `/etc/netmnt/*.cred` (root, 0600). Garde
  l'auth admin polkit. **Validé : survit au reboot** (remonté au boot par systemd).
- **Unmount** : par point de montage ; démantèle l'unit systemd si persistant.
  Entrée Dolphin **netmnt → Unmount**. Accepte chemin nu ou URL `file://`.
- **Ownership** : montages SMB possédés par l'utilisateur (`uid=`/`gid=` envoyés
  par le client). **Validé après reboot** : montage possédé par l'utilisateur
  appelant, lecture/écriture OK.
- **NFS** (`nfs://host/export`) : `Mount` et `Mount (persistent)` fonctionnent
  comme pour SMB, mais sans credentials (`--ask`/`--username` silencieusement
  ignorés — pas d'équivalent KWallet, pas d'option `uid=`/`gid=`). L'accès et
  l'ownership sont entièrement gérés côté serveur (ACL d'export + mapping UID).
  Testé en réel (01/08/2026) : mount session, écriture/lecture/suppression,
  mount persistant (unit `Type=nfs`), démontage — tous OK.

Build/clippy clean, ~20 tests unitaires. Détail et suite : `docs/Roadmap.md`.

### Points ouverts / gotchas
- **Démonter depuis la vue principale, pas depuis la sidebar.** L'entrée
  **netmnt → Unmount** n'apparaît que sur le clic-droit d'un **dossier dans la
  vue fichiers** (`~/mnt/<share>`) : c'est là que KDE applique les ServiceMenus.
  Le **panneau Emplacements** (sidebar, section « Remote ») n'expose que l'« Unmount »
  natif de Solid (umount en tant qu'utilisateur → « must be superuser »), et son
  menu **n'est pas extensible** par ServiceMenu — impossible d'y ajouter netmnt.
  Workaround : ne pas s'en servir (au besoin *Hide* l'entrée auto), démonter via
  la vue fichiers. Validé en réel (25/06/2026) sur un mount authentifié.
- Au démontage, le daemon supprime le **dossier de point de montage** désormais
  vide (`remove_dir` non récursif : laissé en place s'il contient quoi que ce soit).
- Le daemon **fait confiance** à l'uid/gid envoyé par le client (TODO : vérifier
  via `GetConnectionUnixUser` du sujet D-Bus). Sans risque en usage perso.
- Résolution mDNS `.local` lente côté kio-smb (Dolphin), **indépendant de netmnt**
  (mount.cifs résout via le système). Workaround : `/etc/samba/smb.conf` avec
  `name resolve order = host bcast`, ou utiliser l'IP.
- Notifications succès/échec : fait (`notify-send` côté CLI, mount + unmount).
- **Gotcha NFS/Synology** : une ACL POSIX sur le dossier exporté (`ls -ld`
  affiche un `+`, gérée via `synoacltool` sur DSM) prime sur les bits Unix
  classiques et peut bloquer tout accès malgré un `chmod` correct — même sans
  "Windows ACL" visible dans l'UI DSM. La supprimer (`synoacltool -del` /
  `setfacl -b`) en plus d'un `chown`/`chmod` adapté. Voir README Troubleshooting.
- Prochaine piste : SSHFS.

## Lancer le projet

```sh
cargo build           # compile les 3 crates
cargo run -p netmntd  # daemon (échoue à claim org.netmnt sans la conf D-Bus installée)
cargo run -p netmnt -- mount smb://lab1.local/isos
cargo run -p netmnt -- mount nfs://192.168.1.64/volume1/testing
```

Installation système (à terme) : conf D-Bus dans `/usr/share/dbus-1/system.d/`,
policy polkit dans `/usr/share/polkit-1/actions/`, unit dans
`/usr/lib/systemd/system/`, servicemenu dans `~/.local/share/kio/servicemenus/`.

## Architecture

3 crates : `netmnt` (CLI non-privilégié, appelé par le servicemenu) →
D-Bus système → `netmntd` (daemon privilégié, owner de `org.netmnt`) →
`mount.cifs`/`mount.nfs` / unit systemd. Types partagés dans `netmnt-common`
(`smb.rs`, `nfs.rs`). Détail : `docs/Architecture.md`. Choix techniques :
`docs/Decisions-Techniques.md`.

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
3. ~~Récupération fiable du polkit subject (uid/pid appelant) via zbus.~~ **Résolu** :
   sujet `system-bus-name` + `CheckAuthorization` (module `netmntd::polkit`).
