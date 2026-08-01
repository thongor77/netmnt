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
- [x] Prototype : lecture/écriture d'un secret KWallet via `kwallet-query`
      (module `netmnt::creds`) — voir Phase 3
- [x] Prototype : génération + `enable --now` d'une unit systemd `.mount`
      (module `netmntd::exec`) — voir Phase 3

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
- [x] Test réel sur un partage authentifié (`smb://lab1.local/Wiki`) **réussi**
      (25/06/2026, kdialog + KWallet, identifiant réel, ownership = utilisateur
      appelant, write OK, mot de passe hors argv)
- [x] `Mount (persistent)` : unit systemd `.mount` générée + `enable --now` ;
      credentials dans un fichier root-only `/etc/netmnt/*.cred` (jamais dans l'unit)
- [x] Démontage d'un mount persistant : `unmount` détecte l'unit, fait
      `disable --now` + supprime unit et cred (sinon remontage au boot)
- [x] Test réel : persistant **survit au reboot** (25/06/2026, unit
      `home-<user>-mnt-Movies.mount` enabled + remonté au boot ; ownership =
      utilisateur appelant, write OK) ; partage authentifié validé (cf. ci-dessus).
- [x] Entrée Dolphin **Unmount** validée en réel (25/06/2026, depuis la vue
      fichiers sur un mount authentifié). Limite documentée : indisponible depuis
      le panneau Emplacements (menu non extensible). Le daemon nettoie aussi le
      dossier de point de montage vide après démontage.

## Phase 4 — Packaging & UX

- [x] Ownership : le client envoie son uid/gid, passés en `uid=`/`gid=` à
      `mount.cifs` (session + persistant) ⇒ fichiers possédés par l'utilisateur
- [x] `Makefile` d'installation (binaires + dbus/polkit/systemd/servicemenu + unmount)
- [x] Entrée Dolphin **Unmount** (vue fichiers ; no-op clair sur un dossier non
      monté). Pas dispo dans la sidebar Emplacements (menu non extensible KDE).
- [x] Icônes et libellés du servicemenu finalisés — `drive-network` n'existait
      dans aucun thème installé (fallback générique identique partout, visible
      sur `Screenshot/icones.png`) ; remplacé par `folder-network` (sous-menu),
      `media-mount` (Mount), `dialog-password` (Mount as…), `pin` (persistent).
      Testé en réel dans Dolphin (29/07/2026).
- [x] Notifications de succès/échec (`notify-send` côté CLI, mount + unmount,
      succès normal / échec critical avec icône `folder-network`)
- [x] Paquet Arch (`PKGBUILD`) — **publié sur l'AUR** (`netmnt`, v0.1.0-1) ;
      sources versionnées dans `packaging/aur/` (PKGBUILD + .SRCINFO + flux de release)

## Phase 5 — NFS

- [x] Module `netmnt-common::nfs` : parsing `nfs://host/export[/subpath]`,
      source `host:/export` pour `mount.nfs`, point de montage par défaut
      (dernier segment du chemin d'export) — 6 tests unitaires
- [x] Dispatch protocole dans `netmntd::exec` (`resolve_target`) : `mount.cifs`
      vs `mount.nfs`, `Type=cifs` vs `Type=nfs` dans l'unit systemd. Pas de
      `uid=`/`gid=`/`username=`/KWallet pour NFS — l'accès est contrôlé par
      l'ACL d'export du serveur (host/réseau), pas par des credentials
      utilisateur ; l'ownership suit le mapping UID côté serveur.
- [x] Client CLI : `--ask`/`--username` silencieusement ignorés sur `nfs://`
      (décision utilisateur ; pas d'erreur, `Mount as…`/`Mount (persistent)`
      restent utilisables tels quels depuis le servicemenu Dolphin existant,
      qui déclare déjà `X-KDE-Protocols=smb,nfs,ftp,sftp`)
- [x] Test réel (01/08/2026) sur un export Synology NFS
      (`192.168.1.64:/volume1/testing`) : mount session (guest, écriture/lecture/
      suppression OK), mount persistant (unit `home-<user>-mnt-testing.mount`,
      `Type=nfs`, `Options=rw,_netdev`, pas de fichier credentials généré),
      démontage (unit + dossier de point de montage nettoyés). Validation post-
      reboot **à faire** (prévue plus tard dans la journée par l'utilisateur).
- [x] Gotcha découvert et documenté (README Troubleshooting) : une ACL POSIX
      sur le dossier exporté (Synology `synoacltool`, visible via `ls -ld` avec
      un `+`) prime sur les bits Unix classiques côté NFS et peut bloquer tout
      accès malgré un `chmod 777` — il faut la supprimer (`synoacltool -del` /
      `setfacl -b`) en plus d'un `chown`/`chmod` correct.

## Plus tard

- [ ] SSHFS
- [ ] Validation post-reboot du montage persistant NFS
- [ ] Applet Plasma : liste des montages actifs
