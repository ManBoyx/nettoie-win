//! Ce que le programme sait retirer ou régler sous Linux.
//!
//! Linux embarque bien moins de superflu que Windows : la liste est courte, et
//! sur certaines distributions presque vide. Les paquets sont désignés par leur
//! nom exact ; ceux qui n'existent pas sur une distribution ne sont simplement
//! pas proposés.

use crate::catalogue::{Action, Categorie, Cible, Element, Risque, Ruche, Valeur};

use Action::{Paquet, Service};
use Risque::{Attention, SansRisque};

const fn fichier(chemin: &'static str, nom: &'static str, valeur: &'static str) -> Action {
    Action::Registre { ruche: Ruche::Fichier, cle: chemin, nom, valeur: Valeur::Texte(valeur) }
}

const fn bureau(schema: &'static str, nom: &'static str, valeur: &'static str) -> Action {
    Action::Registre { ruche: Ruche::Gsettings, cle: schema, nom, valeur: Valeur::Texte(valeur) }
}

const fn element(
    id: &'static str,
    categorie: Categorie,
    nom: &'static str,
    explication: &'static str,
    risque: Risque,
    actions: &'static [Action],
) -> Element {
    Element { id, categorie, nom, explication, risque, cible: Cible::Partout, actions }
}

const fn appli(id: &'static str, nom: &'static str, explication: &'static str, paquets: &'static [Action]) -> Element {
    element(id, Categorie::Applis, nom, explication, Attention, paquets)
}

pub fn catalogue() -> &'static [Element] {
    CATALOGUE_LINUX
}

const VIE_PRIVEE: &str = "org.gnome.desktop.privacy";

pub const CATALOGUE_LINUX: &[Element] = &[
    // ----- Applis préinstallées -----
    element(
        "jeux-gnome",
        Categorie::Applis,
        "Jeux de GNOME",
        "Solitaire, Mahjongg, Démineur, Sudoku, Échecs et les autres petits jeux fournis avec le bureau.",
        SansRisque,
        &[
            Paquet("aisleriot"),
            Paquet("gnome-mahjongg"),
            Paquet("gnome-mines"),
            Paquet("gnome-sudoku"),
            Paquet("gnome-chess"),
            Paquet("five-or-more"),
            Paquet("four-in-a-row"),
            Paquet("hitori"),
            Paquet("gnome-klotski"),
            Paquet("lightsoff"),
            Paquet("gnome-nibbles"),
            Paquet("gnome-robots"),
            Paquet("quadrapassel"),
            Paquet("swell-foop"),
            Paquet("tali"),
            Paquet("gnome-taquin"),
            Paquet("gnome-tetravex"),
            Paquet("iagno"),
            Paquet("gnome-2048"),
        ],
    ),
    element(
        "jeux-kde",
        Categorie::Applis,
        "Jeux de KDE",
        "Mahjongg, Démineur, Patience et Sudoku fournis avec le bureau KDE.",
        SansRisque,
        &[Paquet("kmahjongg"), Paquet("kmines"), Paquet("kpat"), Paquet("ksudoku")],
    ),
    element(
        "bienvenue",
        Categorie::Applis,
        "Écrans de bienvenue",
        "La visite guidée qui s'ouvre après l'installation (GNOME, KDE, Linux Mint, Manjaro, openSUSE).",
        SansRisque,
        &[
            Paquet("gnome-tour"),
            Paquet("plasma-welcome"),
            Paquet("mintwelcome"),
            Paquet("manjaro-hello"),
            Paquet("opensuse-welcome"),
        ],
    ),
    appli("thunderbird", "Thunderbird", "Messagerie. Les comptes qui y sont configurés ne seront plus relevés.", &[Paquet("thunderbird")]),
    appli("rhythmbox", "Rhythmbox", "Lecteur de musique.", &[Paquet("rhythmbox")]),
    appli("cheese", "Cheese", "Appli pour la webcam.", &[Paquet("cheese")]),
    appli("shotwell", "Shotwell", "Gestionnaire de photos.", &[Paquet("shotwell")]),
    appli("transmission", "Transmission", "Téléchargement de fichiers torrent.", &[Paquet("transmission-gtk"), Paquet("transmission-qt")]),
    appli("remmina", "Remmina", "Connexion à un bureau à distance.", &[Paquet("remmina")]),
    appli("hexchat", "HexChat", "Discussion en ligne par IRC.", &[Paquet("hexchat")]),
    appli("meteo", "Météo", "L'appli météo de GNOME.", &[Paquet("gnome-weather")]),
    appli("cartes", "Cartes", "Cartes et itinéraires de GNOME.", &[Paquet("gnome-maps")]),
    appli("contacts", "Contacts", "Carnet d'adresses de GNOME.", &[Paquet("gnome-contacts")]),
    appli("hypnotix", "Hypnotix", "Télévision par internet de Linux Mint.", &[Paquet("hypnotix")]),
    // ----- Pubs et suggestions -----
    element(
        "annonces-ubuntu",
        Categorie::Pubs,
        "Annonces d'Ubuntu dans le terminal",
        "Retire les nouvelles de Canonical affichées à la connexion et les publicités pour Ubuntu Pro pendant les mises à jour.",
        SansRisque,
        &[
            fichier("/etc/default/motd-news", "ENABLED", "0"),
            Action::Registre { ruche: Ruche::Pro, cle: "config", nom: "apt_news", valeur: Valeur::Texte("False") },
        ],
    ),
    // ----- Télémétrie -----
    element(
        "plantages-ubuntu",
        Categorie::Telemetrie,
        "Rapports de plantage (Ubuntu)",
        "Ubuntu ne prépare plus et n'envoie plus de rapport à Canonical quand un programme plante.",
        SansRisque,
        &[
            fichier("/etc/default/apport", "enabled", "0"),
            Service("apport.service"),
            Service("whoopsie.service"),
            Service("kerneloops.service"),
        ],
    ),
    element(
        "plantages-fedora",
        Categorie::Telemetrie,
        "Rapports de plantage (Fedora)",
        "Désactive les services ABRT qui collectent les plantages pour les envoyer à Fedora.",
        SansRisque,
        &[
            Service("abrtd.service"),
            Service("abrt-journal-core.service"),
            Service("abrt-oops.service"),
            Service("abrt-xorg.service"),
            Service("abrt-vmcore.service"),
        ],
    ),
    element(
        "statistiques-paquets",
        Categorie::Telemetrie,
        "Statistiques d'utilisation des paquets",
        "Debian et Ubuntu n'envoient plus chaque semaine la liste des paquets installés (popularity-contest).",
        SansRisque,
        &[fichier("/etc/popularity-contest.conf", "PARTICIPATE", "\"no\"")],
    ),
    element(
        "rapport-ubuntu",
        Categorie::Telemetrie,
        "Rapport d'installation d'Ubuntu",
        "Retire l'outil qui envoie à Canonical la description du matériel et de l'installation.",
        SansRisque,
        &[Paquet("ubuntu-report")],
    ),
    element(
        "recensement-zorin",
        Categorie::Telemetrie,
        "Recensement de Zorin OS",
        "Retire l'outil qui signale chaque jour à Zorin que ce PC est en service.",
        SansRisque,
        &[Paquet("zorin-os-census")],
    ),
    element(
        "rapports-gnome",
        Categorie::Telemetrie,
        "Rapports techniques de GNOME",
        "Le bureau n'envoie plus de rapports de problèmes ni de statistiques d'utilisation. Réglage propre à votre compte.",
        SansRisque,
        &[
            bureau(VIE_PRIVEE, "report-technical-problems", "false"),
            bureau(VIE_PRIVEE, "send-software-usage-stats", "false"),
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::est_protege;
    use crate::gestionnaire::nom_valide;
    use std::collections::HashSet;

    #[test]
    fn les_identifiants_sont_uniques_et_chaque_element_est_complet() {
        let mut vus = HashSet::new();
        for e in catalogue() {
            assert!(vus.insert(e.id), "identifiant en double : {}", e.id);
            assert!(!e.nom.trim().is_empty() && e.explication.trim().len() > 15, "{}", e.id);
            assert!(!e.actions.is_empty(), "{}", e.id);
            assert_eq!(e.cible, Cible::Partout, "{}", e.id);
        }
    }

    #[test]
    fn les_trois_categories_sont_remplies() {
        for c in [Categorie::Applis, Categorie::Pubs, Categorie::Telemetrie] {
            assert!(catalogue().iter().any(|e| e.categorie == c), "{c:?} est vide");
        }
    }

    #[test]
    fn les_paquets_sont_des_noms_exacts_valides_et_jamais_proteges() {
        for e in catalogue() {
            for a in e.actions {
                if let Paquet(nom) = a {
                    assert!(nom_valide(nom), "{} : {nom}", e.id);
                    assert!(!nom.contains('*'), "{} : pas de motif sous Linux ({nom})", e.id);
                    assert!(!est_protege(nom), "{} : {nom} est protégé", e.id);
                }
            }
        }
    }

    #[test]
    fn aucun_reglage_ne_vient_du_registre_de_windows() {
        for e in catalogue() {
            for a in e.actions {
                match a {
                    Action::Registre { ruche, valeur, cle, .. } => {
                        assert!(matches!(ruche, Ruche::Fichier | Ruche::Gsettings | Ruche::Pro), "{}", e.id);
                        assert!(matches!(valeur, Valeur::Texte(_)), "{}", e.id);
                        if *ruche == Ruche::Fichier {
                            assert!(cle.starts_with("/etc/"), "{} : {cle}", e.id);
                        }
                    }
                    Action::Tache(_) | Action::OneDrive => panic!("{} : action propre à Windows", e.id),
                    _ => {}
                }
            }
        }
    }

    #[test]
    fn les_services_sont_des_unites_systemd_nommees_en_entier() {
        for e in catalogue() {
            for a in e.actions {
                if let Service(nom) = a {
                    assert!(nom.ends_with(".service"), "{} : {nom}", e.id);
                }
            }
        }
    }

    #[test]
    fn seuls_les_jeux_et_ecrans_de_bienvenue_sont_des_applis_cochees_d_office() {
        for e in catalogue().iter().filter(|e| e.categorie == Categorie::Applis) {
            let attendu = ["jeux-gnome", "jeux-kde", "bienvenue"].contains(&e.id);
            assert_eq!(e.coche_par_defaut(), attendu, "{}", e.id);
        }
    }

    #[test]
    fn rien_ne_touche_aux_mises_a_jour_au_reseau_ni_a_la_connexion() {
        for e in catalogue() {
            let texte = format!("{:?}", e.actions).to_lowercase();
            for interdit in ["unattended", "apt-daily", "packagekit", "networkmanager", "ssh", "firewall", "ufw", "systemd-"] {
                assert!(!texte.contains(interdit), "{} touche à {interdit}", e.id);
            }
        }
    }
}
