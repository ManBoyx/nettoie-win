# Nettoie-Win et Nettoie-Linux

Un seul fichier à lancer, qui retire du système ce qui ne sert à rien :

- **Applis préinstallées** : jeux promotionnels, raccourcis publicitaires, écrans de bienvenue…
- **Pubs et suggestions** : applis installées sans demander, suggestions du menu Démarrer, annonces dans le terminal…
- **Télémétrie** : services et tâches de collecte, rapports de plantage, identifiant de publicité…

| Système | Fichier |
|---|---|
| Windows 10 et Windows 11 | `Nettoie-Win.exe` |
| Linux (Debian, Ubuntu, Mint, Fedora, openSUSE, Arch, Manjaro…) | `nettoie-linux` |

![La fenêtre de Nettoie-Win](docs/fenetre.png)

*Capture prise en mode démonstration.*

## Téléchargement

[Dernière version](https://github.com/ManBoyx/nettoie-win/releases/latest) — rien à installer.

**Version jeune.** Elle est vérifiée par des tests automatiques sur des systèmes simulés et, pour Linux, par de vrais retraits dans des machines d'essai. Elle n'a pas encore tourné sur un grand nombre de vrais PC. Pour un premier essai : regarder **Aperçu**, appliquer deux ou trois éléments, puis vérifier que **Annuler les changements** les remet.

## Utilisation

1. Lancer le programme.
   - Windows : accepter la demande de droits d'administrateur.
   - Linux : `chmod +x nettoie-linux` puis `./nettoie-linux`. Le mot de passe n'est demandé qu'au moment d'appliquer.
2. Le programme analyse le PC et n'affiche que ce qui s'y trouve réellement.
3. Cocher ou décocher. Ce qui est sans risque est déjà coché ; ce qui peut manquer à quelqu'un est proposé mais décoché.
4. **Aperçu** montre le détail de ce qui sera fait, sans rien toucher.
5. **Appliquer** fait les changements.
6. Redémarrer le PC.

## Revenir en arrière

- **Annuler les changements** remet les réglages et services dans leur état d'origine. L'ancienne valeur de chacun est notée avant d'être modifiée.
- Les **applis retirées ne reviennent pas** par ce bouton. Sous Windows, elles se réinstallent depuis le Microsoft Store ; sous Linux, avec le gestionnaire de paquets (`sudo apt install nom`, par exemple).
- Sous Windows, un point de restauration est créé avant chaque application (Panneau de configuration → Récupération → Ouvrir la Restauration du système).

## Windows 10 et Windows 11

Le même fichier reconnaît la version de Windows et adapte sa liste.

- Communs aux deux : Candy Crush et autres jeux, Solitaire, raccourci Office, Hub de commentaires, pubs du menu Démarrer et de l'écran de verrouillage, recherche Bing, services et tâches de télémétrie, identifiant de publicité…
- Propres à Windows 10 : Skype, applis 3D, « Actualités et champs d'intérêt », icône « Démarrer une réunion ».
- Propres à Windows 11 : Chat Teams, Dev Home, Widgets, recommandations du menu Démarrer, Recall ; et, proposés décochés, Copilot, Clipchamp, le nouvel Outlook, To Do, Power Automate.

**Jamais touchés** : Windows Update, Microsoft Defender, le Microsoft Store, Edge, les pilotes et leurs panneaux de réglage, .NET et les bibliothèques Visual C++.

À savoir :

- Le fichier n'est pas signé : Windows affiche « éditeur inconnu » au lancement. Un antivirus peut aussi le signaler, comme tous les outils de ce genre.
- Les réglages « par compte » s'appliquent au compte qui donne les droits d'administrateur.
- Après certains réglages, l'appli Paramètres affiche « Certains paramètres sont gérés par votre organisation » : c'est normal, ils passent par les stratégies de Windows.
- Une grosse mise à jour de Windows peut réinstaller des applis ou remettre des réglages. Il suffit de relancer le programme.
- Le journal des changements est dans `%LOCALAPPDATA%\Nettoie-Win\journal.json`.

## Linux

Linux embarque bien moins de superflu que Windows : la liste est courte, et sur certaines distributions (Arch, un serveur…) elle peut être vide. C'est normal.

- Applis : jeux de GNOME et de KDE, écrans de bienvenue ; et, proposés décochés, Thunderbird, Rhythmbox, Cheese, Shotwell, Transmission, Remmina, HexChat…
- Pubs : annonces de Canonical à la connexion et publicités pour Ubuntu Pro pendant les mises à jour.
- Télémétrie : rapports de plantage d'Ubuntu (apport, whoopsie) et de Fedora (ABRT), popularity-contest, rapport d'installation d'Ubuntu, recensement de Zorin OS, rapports techniques de GNOME.

**Chaque retrait est d'abord simulé.** Si retirer un paquet devait en emporter d'autres, le retrait est refusé et le programme dit pourquoi. Il n'utilise jamais `autoremove` ni `purge`.

**Jamais touchés** : le noyau, systemd, le bureau et l'écran de connexion, le réseau, les mises à jour, le gestionnaire de paquets lui-même.

À savoir :

- Familles reconnues : `apt` (Debian, Ubuntu, Mint, Pop!_OS, Zorin…), `rpm` (Fedora, openSUSE…), `pacman` (Arch, Manjaro, EndeavourOS…).
- Les applis installées en **Snap** ou en **Flatpak** ne sont pas gérées.
- Seule la partie qui modifie le système tourne en administrateur, lancée par `pkexec`. Elle n'accepte que les opérations de la liste. Sans `pkexec`, lancer le programme avec `sudo`.
- `./nettoie-linux --liste` écrit dans le terminal ce qui serait proposé, sans fenêtre et sans rien modifier.
- Le journal des changements est dans `~/.local/state/nettoie-linux/journal.json`.

## Compilation

Rust, compilé depuis Linux avec `cargo-zigbuild` (zig doit être dans le `PATH`) :

```
cargo zigbuild --release --target x86_64-pc-windows-gnu          # Nettoie-Win.exe
cargo zigbuild --release --target x86_64-unknown-linux-gnu.2.28  # nettoie-linux
```

Les résultats sont `target/x86_64-pc-windows-gnu/release/Nettoie-Win.exe` et `target/x86_64-unknown-linux-gnu/release/Nettoie-Win` (à renommer `nettoie-linux`).

## Tests

```
cargo test --no-default-features     # la logique seule, en quelques secondes
cargo test                           # avec la fenêtre, pilotée sans écran
cargo test --test fenetre captures -- --ignored   # images de contrôle dans captures/
```

Les tests tournent sur de faux systèmes en mémoire (`src/faux.rs`). `--demonstration` ouvre la fenêtre sur un PC fictif, sans rien modifier.

Deux programmes d'essai servent aux vérifications sur de vrais systèmes : `examples/essai_windows.rs` (registre, sous Windows ou Wine) et `examples/essai_retrait.rs` (retrait d'un paquet Linux, dans une machine jetable).

## Organisation du code

| Fichier | Rôle |
|---|---|
| `src/catalogue.rs` | Ce qui peut être retiré ou réglé sous Windows, et la liste protégée. Ajouter un élément = ajouter une entrée ici. |
| `src/catalogue_linux.rs` | La même chose pour Linux. |
| `src/analyse.rs` | Compare le catalogue à l'état du PC. |
| `src/execution.rs` | Fait les changements, un par un. |
| `src/journal.rs` | Note l'état d'avant et sait le remettre. |
| `src/systeme.rs` | Ce que le programme demande au système (interface). |
| `src/windows.rs`, `src/texte.rs` | Windows : registre, `sc`, `schtasks`, PowerShell. |
| `src/linux.rs`, `src/gestionnaire.rs`, `src/fichier_conf.rs` | Linux : paquets, systemd, fichiers de configuration. |
| `src/assistant.rs` | Linux : la partie qui tourne en administrateur. |
| `src/moteur.rs` | Fil de travail, pour que la fenêtre ne se fige pas. |
| `src/interface.rs` | La fenêtre. |
