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
- [x] Prototype : garde polkit via `CheckAuthorization` (sujet `system-bus-name`,
      module `netmntd::polkit`) — câblé sur `Mount`/`Unmount`
- [ ] Prototype : lecture d'un secret KWallet via Secret Service D-Bus
- [ ] Prototype : génération + `enable --now` d'une unit systemd `.mount`

## Phase 2 — MVP « Mount » session

- [x] Implémenter `Mount` (session, guest) bout en bout : CLI → D-Bus → daemon
- [x] Convention du point de montage (`~/mnt/<share>`, confiné à la base)
- [x] Implémenter `Unmount`
- [x] Garde polkit sur chaque méthode mutante
- [x] Outillage d'install pour le test réel (`Makefile` : build/install/reload/
      uninstall ; fichier d'activation D-Bus ; unit corrigée `MountFlags=shared`)
- [x] Test manuel sur un vrai partage **réussi** (25/06/2026,
      `smb://lab1.local/public` invité : mount + unmount OK, polkit + mount.cifs validés)
- [x] `Mount as…` avec mot de passe (prompt sécurisé, câblé — voir Phase 3)

## Phase 3 — Credentials & persistance

- [x] `Mount as…` : prompt username/password (kdialog ou tty) + lecture/écriture
      KWallet (`netmnt mount --ask`) ; mot de passe hors argv, stocké seulement
      après un montage réussi
- [ ] Test réel sur un partage authentifié (`smb://lab1.local/Wiki`)
- [x] `Mount (persistent)` : unit systemd `.mount` générée + `enable --now` ;
      credentials dans un fichier root-only `/etc/netmnt/*.cred` (jamais dans l'unit)
- [x] Démontage d'un mount persistant : `unmount` détecte l'unit, fait
      `disable --now` + supprime unit et cred (sinon remontage au boot)
- [ ] Test réel : persistant (reboot), partage authentifié, entrée Unmount Dolphin

## Phase 4 — Packaging & UX

- [x] `Makefile` d'installation (binaires + dbus/polkit/systemd/servicemenu + unmount)
- [ ] Entrée Dolphin **Unmount** ajoutée (à affiner : visible sur tout dossier local)
- [ ] Icônes et libellés du servicemenu finalisés
- [ ] Notifications de succès/échec
- [ ] Paquet Arch (`PKGBUILD`)

## Plus tard

- [ ] NFS, SSHFS
- [ ] Applet Plasma : liste des montages actifs
