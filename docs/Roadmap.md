# Roadmap — netmnt

## Phase 0 — Scaffold (fait)

- [x] Phase d'architecture (problème, utilisateurs, cas d'usage, inconnues)
- [x] Workspace Cargo + 3 crates (`netmnt`, `netmntd`, `netmnt-common`)
- [x] Contrat D-Bus (types partagés, constantes)
- [x] Fichiers d'intégration : dbus conf, polkit policy, systemd unit, servicemenu
- [x] Documentation (README, CLAUDE.md, docs/)

## Phase 1 — Prototypes des inconnues critiques

But : lever les risques avant de figer l'implémentation (cf. Architecture.md).

- [ ] Prototype : montage SMB réel via `mount.cifs` depuis le daemon (session)
- [ ] Prototype : récupération du sujet polkit (uid/pid) + `CheckAuthorization`
- [ ] Prototype : lecture d'un secret KWallet via Secret Service D-Bus
- [ ] Prototype : génération + `enable --now` d'une unit systemd `.mount`

## Phase 2 — MVP « Mount » session

- [ ] Implémenter `Mount` (session, guest) bout en bout : Dolphin → CLI → daemon
- [ ] Choix/convention du point de montage (`~/mnt/<share>`) + collisions
- [ ] Implémenter `Unmount`
- [ ] Garde polkit sur chaque méthode
- [ ] Test manuel sur `smb://lab1.local/isos`

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
