# Architecture — netmnt

## Problème

Naviguer `smb://host/share` dans Dolphin n'expose qu'une vue KIO/FUSE transitoire,
pas un point de montage réel et prévisible utilisable par le terminal et toutes
les applications. Configurer `mount.cifs`, fstab ou des units systemd à la main est
fastidieux. netmnt comble ce trou avec une action « clic-droit → montage à un
chemin stable » pilotée par un service.

## Utilisateurs

- Utilisateur Linux/KDE qui navigue sur un NAS SMB depuis Dolphin.
- Cas typique : monter `smb://lab1.local/isos` à `~/mnt/isos` en un clic, parfois
  avec d'autres credentials, parfois de façon persistante.

## Cas d'usage

| Action                | Déclencheur Dolphin   | Résultat                                            |
| --------------------- | --------------------- | --------------------------------------------------- |
| Mount                 | clic-droit → Mount    | montage session à un chemin stable                  |
| Mount as…             | clic-droit → Mount as | idem avec username/password explicites              |
| Mount (persistent)    | clic-droit → Mount…   | unit systemd `.mount` générée, survit au reboot     |
| Unmount               | (futur)               | démontage par point de montage                      |

## Vue d'ensemble

```
Dolphin (clic-droit smb://)
   └─ ServiceMenu .desktop  →  netmnt (CLI, non-privilégié)
                                   │  D-Bus (system bus, org.netmnt.Manager1)
                              netmntd (daemon, privilégié, owner org.netmnt)
                                   │  autorisé par polkit (org.netmnt.*)
                                   ├─ mount.cifs           → ~/mnt/<share>
                                   ├─ KWallet (Secret Service) pour credentials
                                   └─ unit systemd .mount  (si persistant)
```

## Composants

| Crate            | Rôle                                                                 |
| ---------------- | ------------------------------------------------------------------- |
| `netmnt`         | Client CLI non-privilégié, appelé par le servicemenu Dolphin.       |
| `netmntd`        | Daemon privilégié, owner de `org.netmnt`, exécute le montage réel.  |
| `netmnt-common`  | Types et constantes partagés (contrat D-Bus).                       |

Fichiers d'intégration dans `data/` :
`dbus/org.netmnt.conf`, `polkit/org.netmnt.policy`, `systemd/netmntd.service`,
`servicemenus/netmnt.desktop`.

## Contrat D-Bus

- Bus : **system bus**
- Nom : `org.netmnt`
- Objet : `/org/netmnt/Manager`
- Interface : `org.netmnt.Manager1`
- Méthodes : `Mount(MountRequest) -> MountResult`, `Unmount(mount_point)`

Types : voir `crates/netmnt-common/src/lib.rs` (`MountRequest`, `MountResult`).
Les types « wire » restent primitifs (D-Bus n'a pas d'union taguée native) :
credentials et persistance sont encodés en champs plats plutôt qu'en enums Rust.

## Inconnues critiques

1. **KWallet côté daemon** — lire un secret via le Secret Service D-Bus depuis un
   process root pour le compte de l'utilisateur appelant. À prototyper.
2. **Unit systemd `.mount` à chaud** — génération idempotente + `daemon-reload` +
   `enable --now` sans état incohérent.
3. **Sujet polkit** — récupérer uid/pid de l'appelant via zbus pour construire le
   `PolkitSubject` et appeler `CheckAuthorization`.
4. **Choix du point de montage** — convention (`~/mnt/<share>` ? `/run/media` ?),
   gestion des collisions, droits.

Ces inconnues doivent être levées par prototype avant de figer l'architecture.

## Extensions futures

- NFS, SSHFS (le `CredentialSource`/`MountRequest` est déjà générique).
- Applet Plasma / notifications.
- Liste des montages actifs + démontage depuis le menu.
