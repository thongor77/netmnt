# Décisions techniques — netmnt

## D1 — Daemon D-Bus + polkit plutôt que pkexec/setuid ou gio

**Décision.** Le montage est fait par un daemon privilégié activé par systemd,
exposé sur le system bus, et chaque méthode mutante est autorisée par polkit. Le
client CLI reste non-privilégié.

**Alternatives écartées.**
- *pkexec / helper setuid* : plus rapide à prototyper mais autorisation grossière
  (tout ou rien), moins auditable, mauvaise séparation des privilèges.
- *gio mount (gvfs)* : aucun root requis, mais montage sous `/run/user/UID/gvfs/`,
  chemin instable mal géré par les outils CLI → contredit l'objectif « chemin stable ».

**Raisons.** Vraie séparation client/daemon, autorisation fine et révocable,
cohérent avec « tourne comme un service Linux » et avec l'écosystème (udisks2,
NetworkManager fonctionnent ainsi).

**Conséquences.** Plus de boilerplate initial (conf D-Bus, policy polkit, unit
systemd). Récupération du sujet polkit à implémenter.

## D2 — systemd `.mount` plutôt que fstab pour le persistant

**Décision.** Un montage persistant génère une unit systemd `.mount`.

**Raisons.** Plus moderne, gestion des dépendances (`network-online.target`),
`enable`/`disable` propres, pas d'édition d'un fichier partagé fragile (`/etc/fstab`).

**Conséquences.** Nécessite `daemon-reload` + `enable --now` ; idempotence à soigner.

## D3 — Rust + zbus

**Décision.** Rust pour le client et le daemon ; `zbus` (pur Rust, async) pour D-Bus.

**Raisons.** Cohérence avec les développements Linux modernes du workspace,
sûreté mémoire pour un process privilégié, `zbus` mûr et sans dépendance C.

## D4 — Backend mount.cifs (cifs-utils)

**Décision.** SMB monté via `mount.cifs`. NFS/SSHFS plus tard via le même contrat.

**Raisons.** Montage noyau réel (vs FUSE), performant, point de montage standard.

---

## Alternatives existantes évaluées (avant de démarrer)

| Outil          | Couvre quoi                                   | Pourquoi insuffisant ici                          |
| -------------- | --------------------------------------------- | ------------------------------------------------- |
| **Smb4K**      | App KDE : browse + mount SMB, credentials, signets | App autonome, pas une action clic-droit dans Dolphin |
| **kio-fuse**   | Expose `smb://` comme montage FUSE transitoire | Chemin sous `/run/user`, pas stable/prévisible    |
| **gio mount**  | Montage userspace gvfs                          | `/run/user/UID/gvfs`, mal géré en CLI             |
| **fstab cifs** | Montage persistant classique                    | Manuel, fichier partagé fragile                   |

Conclusion : la niche de netmnt est le workflow « clic-droit → chemin stable »
piloté par un service Rust. Smb4K reste l'alternative la plus proche à recommander
si ce workflow précis n'est pas requis.
