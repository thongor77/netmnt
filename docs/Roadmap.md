# Roadmap — netmnt

## Phase 0 — Scaffold (fait)

- [x] Phase d'architecture (problème, utilisateurs, cas d'usage, inconnues)
- [x] Workspace Cargo + 3 crates (`netmnt`, `netmntd`, `netmnt-common`)
- [x] Contrat D-Bus (types partagés, constantes)
- [x] Fichiers d'intégration : dbus conf, polkit policy, systemd unit, servicemenu
- [x] Documentation (README, CLAUDE.md, docs/)

## Phase 1 — Prototypes des inconnues critiques

But : lever les risques avant de figer l'implémentation (cf. Architecture.md).

- [x] Prototype : parsing `smb://` + résolution du point de montage (module
      `netmnt-common::smb`, testé : 7 tests unitaires)
- [x] Prototype : montage SMB réel via `mount.cifs` depuis le daemon, session
      (module `netmntd::exec`) — mot de passe hors argv via env `PASSWD`
- [ ] Prototype : récupération du sujet polkit (uid/pid) + `CheckAuthorization`
- [ ] Prototype : lecture d'un secret KWallet via Secret Service D-Bus
- [ ] Prototype : génération + `enable --now` d'une unit systemd `.mount`

## Phase 2 — MVP « Mount » session

- [x] Implémenter `Mount` (session, guest) bout en bout : CLI → D-Bus → daemon
- [x] Convention du point de montage (`~/mnt/<share>`, confiné à la base)
- [x] Implémenter `Unmount`
- [ ] Garde polkit sur chaque méthode
- [ ] Test manuel sur un vrai partage (nécessite root + serveur SMB)
- [ ] `Mount as…` avec mot de passe (prompt sécurisé, pas encore câblé)

## Phase 3 — Credentials & persistance

- [ ] `Mount as…` : prompt username/password (KDialog) + stockage KWallet
- [ ] `Mount (persistent)` : unit systemd `.mount` + credentials persistés
- [ ] Démontage d'un mount persistant (disable de l'unit)

## Phase 4 — Packaging & UX

- [ ] Script/`Makefile` d'installation (placement dbus/polkit/systemd/servicemenu)
- [ ] Icônes et libellés du servicemenu finalisés
- [ ] Notifications de succès/échec
- [ ] Paquet Arch (`PKGBUILD`)

## Plus tard

- [ ] NFS, SSHFS
- [ ] Applet Plasma : liste des montages actifs
