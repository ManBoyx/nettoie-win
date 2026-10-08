//! Les gestionnaires de paquets de Linux : quelles commandes lancer, comment lire leurs réponses.
//!
//! Rien ici ne lance de commande : ce sont des fonctions sur du texte.

use crate::catalogue::est_protege;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Gestionnaire {
    /// Debian, Ubuntu, Linux Mint, Pop!_OS, Zorin…
    Apt,
    /// Fedora, openSUSE…
    Rpm,
    /// Arch Linux, Manjaro, EndeavourOS…
    Pacman,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commande {
    pub programme: &'static str,
    pub arguments: Vec<String>,
    pub environnement: Vec<(&'static str, &'static str)>,
}

/// Paquets « chapeaux » qui ne contiennent rien : les voir partir avec un jeu est sans conséquence.
pub const CHAPEAUX_TOLERES: &[&str] = &["gnome-games", "kdegames", "kde-games"];

impl Gestionnaire {
    /// `existe` dit si un programme est disponible sur ce PC.
    pub fn detecter(existe: &dyn Fn(&str) -> bool) -> Option<Gestionnaire> {
        if existe("apt-get") && existe("dpkg-query") {
            Some(Gestionnaire::Apt)
        } else if existe("rpm") {
            Some(Gestionnaire::Rpm)
        } else if existe("pacman") {
            Some(Gestionnaire::Pacman)
        } else {
            None
        }
    }

    /// Commande qui liste les paquets installés (sans droits particuliers).
    pub fn liste(self) -> Commande {
        match self {
            Gestionnaire::Apt => commande("dpkg-query", &["-W", "-f", "${Package}\t${db:Status-Abbrev}\n"]),
            Gestionnaire::Rpm => commande("rpm", &["-qa", "--qf", "%{NAME}\n"]),
            Gestionnaire::Pacman => commande("pacman", &["-Qq"]),
        }
    }

    pub fn paquets(self, sortie: &str) -> Vec<String> {
        let mut noms: Vec<String> = Vec::new();
        for ligne in sortie.lines() {
            let nom = match self {
                // « ii » : voulu et entièrement installé.
                Gestionnaire::Apt => match ligne.split_once('\t') {
                    Some((nom, etat)) if etat.starts_with("ii") => nom,
                    _ => continue,
                },
                Gestionnaire::Rpm | Gestionnaire::Pacman => ligne.trim(),
            };
            if !nom.is_empty() && !noms.iter().any(|n| n == nom) {
                noms.push(nom.to_string());
            }
        }
        noms
    }

    /// Commande qui simule le retrait de `nom` sans rien changer.
    pub fn simulation(self, nom: &str) -> Commande {
        match self {
            Gestionnaire::Apt => commande("apt-get", &["-s", "remove", nom]),
            Gestionnaire::Rpm => commande("rpm", &["-e", "--test", nom]),
            Gestionnaire::Pacman => commande("pacman", &["-R", "--print", "--print-format", "%n", nom]),
        }
    }

    /// Lit la simulation : les autres paquets qui partiraient avec `nom`,
    /// ou une erreur si le retrait est impossible.
    pub fn emportes(self, nom: &str, reussie: bool, sortie: &str) -> Result<Vec<String>, String> {
        if !reussie {
            let marque = match self {
                Gestionnaire::Rpm => "needed by (installed) ",
                Gestionnaire::Pacman => " required by ",
                Gestionnaire::Apt => return Err(resume(sortie)),
            };
            let mut dependants: Vec<&str> = Vec::new();
            for nom in sortie.lines().filter_map(|l| l.split(marque).nth(1)).map(str::trim) {
                if !dependants.contains(&nom) {
                    dependants.push(nom);
                }
            }
            return Err(if dependants.is_empty() {
                resume(sortie)
            } else {
                format!("d'autres paquets en dépendent : {}", dependants.join(", "))
            });
        }
        let retires: Vec<&str> = match self {
            Gestionnaire::Apt => {
                sortie.lines().filter_map(|l| l.strip_prefix("Remv ")).filter_map(|l| l.split_whitespace().next()).collect()
            }
            // Un essai réussi signifie que rien d'autre ne dépend du paquet.
            Gestionnaire::Rpm => Vec::new(),
            Gestionnaire::Pacman => sortie.lines().map(str::trim).filter(|l| !l.is_empty()).collect(),
        };
        Ok(retires.into_iter().filter(|n| *n != nom).map(str::to_string).collect())
    }

    /// Commande qui retire `nom`, et lui seul.
    pub fn retrait(self, nom: &str) -> Commande {
        match self {
            Gestionnaire::Apt => Commande {
                environnement: vec![("LC_ALL", "C"), ("DEBIAN_FRONTEND", "noninteractive")],
                ..commande("apt-get", &["remove", "-y", nom])
            },
            Gestionnaire::Rpm => commande("rpm", &["-e", nom]),
            Gestionnaire::Pacman => commande("pacman", &["-R", "--noconfirm", nom]),
        }
    }

    /// Commande à donner à l'utilisateur pour remettre des paquets.
    pub fn reinstallation(self, noms: &[String]) -> String {
        let noms = noms.join(" ");
        match self {
            Gestionnaire::Apt => format!("sudo apt install {noms}"),
            Gestionnaire::Rpm => format!("sudo dnf install {noms} (ou : sudo zypper install {noms})"),
            Gestionnaire::Pacman => format!("sudo pacman -S {noms}"),
        }
    }
}

fn commande(programme: &'static str, arguments: &[&str]) -> Commande {
    Commande {
        programme,
        arguments: arguments.iter().map(|a| a.to_string()).collect(),
        // Messages en anglais : ce sont eux que le programme sait lire.
        environnement: vec![("LC_ALL", "C")],
    }
}

/// Les dernières lignes utiles d'un message d'erreur, sur une seule ligne.
fn resume(sortie: &str) -> String {
    let lignes: Vec<&str> = sortie.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
    let debut = lignes.len().saturating_sub(3);
    match lignes[debut..].join(" ; ") {
        vide if vide.is_empty() => "le gestionnaire de paquets a refusé".to_string(),
        texte => texte,
    }
}

/// Un nom de paquet Linux ne contient que lettres, chiffres, `+ . _ -`, et ne commence pas par un tiret.
pub fn nom_valide(nom: &str) -> bool {
    !nom.is_empty()
        && nom.len() <= 128
        && !nom.starts_with('-')
        && nom.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '_' | '-'))
}

/// Décide si un retrait a réussi. Un gestionnaire de paquets peut signaler une
/// erreur sans rapport (un autre paquet déjà cassé sur le PC) alors que le
/// paquet demandé est bien parti : c'est sa disparition qui compte.
pub fn issue_retrait(reussie: bool, encore_installe: bool, sortie: &str) -> Result<(), String> {
    if reussie || !encore_installe {
        Ok(())
    } else {
        Err(resume(sortie))
    }
}

/// Refuse un retrait qui emporterait autre chose que le paquet demandé.
pub fn verifier_retrait(nom: &str, emportes: &[String]) -> Result<(), String> {
    if est_protege(nom) {
        return Err(format!("{nom} est nécessaire au système"));
    }
    let genants: Vec<&str> = emportes
        .iter()
        .map(String::as_str)
        .filter(|n| est_protege(n) || !CHAPEAUX_TOLERES.contains(n))
        .collect();
    if genants.is_empty() {
        Ok(())
    } else {
        Err(format!("le retirer emporterait aussi : {}", genants.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Gestionnaire::*;

    fn avec(programmes: &'static [&'static str]) -> Option<Gestionnaire> {
        Gestionnaire::detecter(&|p| programmes.contains(&p))
    }

    #[test]
    fn chaque_famille_est_reconnue_a_ses_outils() {
        assert_eq!(avec(&["apt-get", "dpkg-query"]), Some(Apt));
        assert_eq!(avec(&["rpm", "dnf"]), Some(Rpm));
        assert_eq!(avec(&["rpm", "zypper"]), Some(Rpm));
        assert_eq!(avec(&["pacman"]), Some(Pacman));
        assert_eq!(avec(&["ls"]), None);
    }

    #[test]
    fn une_debian_ou_rpm_est_aussi_installe_reste_une_debian() {
        assert_eq!(avec(&["rpm", "apt-get", "dpkg-query"]), Some(Apt));
    }

    #[test]
    fn la_liste_apt_ne_garde_que_les_paquets_vraiment_installes() {
        let sortie = "aisleriot\tii \nvieux-paquet\trc \nlibc6\tii \nlibc6\tii \ncasse\tiU \n";
        assert_eq!(Apt.paquets(sortie), vec!["aisleriot".to_string(), "libc6".to_string()]);
    }

    #[test]
    fn les_listes_rpm_et_pacman_sont_un_nom_par_ligne() {
        assert_eq!(Rpm.paquets("gnome-tour\nabrt\n\n"), vec!["gnome-tour".to_string(), "abrt".to_string()]);
        assert_eq!(Pacman.paquets("manjaro-hello\nkpat\n"), vec!["manjaro-hello".to_string(), "kpat".to_string()]);
    }

    const APT_SEUL: &str = "Reading package lists...\nBuilding dependency tree...\nThe following packages will be REMOVED:\n  aisleriot\n0 upgraded, 0 newly installed, 1 to remove and 0 not upgraded.\nRemv aisleriot [1:3.22.23-1]\n";
    const APT_ENTRAINE: &str = "The following packages will be REMOVED:\n  gnome-games gnome-mines\nRemv gnome-games [1:43+1]\nRemv gnome-mines [1:40.1-2]\n";
    const APT_BUREAU: &str = "Remv ubuntu-desktop [1.539]\nRemv ubuntu-desktop-minimal [1.539]\nRemv gnome-shell [46.0-0ubuntu5]\nRemv mutter [46.0]\n";

    #[test]
    fn apt_un_paquet_qui_part_seul_n_emporte_rien() {
        assert_eq!(Apt.emportes("aisleriot", true, APT_SEUL), Ok(vec![]));
    }

    #[test]
    fn apt_les_autres_paquets_retires_sont_releves() {
        assert_eq!(Apt.emportes("gnome-mines", true, APT_ENTRAINE), Ok(vec!["gnome-games".to_string()]));
    }

    #[test]
    fn apt_une_simulation_en_echec_est_une_erreur() {
        assert!(Apt.emportes("x", false, "E: Unable to locate package x\n").is_err());
    }

    #[test]
    fn rpm_un_paquet_dont_d_autres_dependent_est_refuse_en_les_nommant() {
        let sortie = "error: Failed dependencies:\n\tlibabrt.so.0()(64bit) is needed by (installed) gnome-abrt-1.4.2-5.fc40.x86_64\n";
        let erreur = Rpm.emportes("abrt-libs", false, sortie).unwrap_err();
        assert!(erreur.contains("gnome-abrt-1.4.2-5.fc40.x86_64"), "{erreur}");
        assert_eq!(Rpm.emportes("gnome-tour", true, ""), Ok(vec![]));
    }

    #[test]
    fn pacman_un_paquet_requis_ailleurs_est_refuse() {
        let sortie = "error: failed to prepare transaction (could not satisfy dependencies)\n:: removing kpat breaks dependency 'kpat' required by kde-games-meta\n";
        let erreur = Pacman.emportes("kpat", false, sortie).unwrap_err();
        assert_eq!(erreur, "d'autres paquets en dépendent : kde-games-meta");
        // Vu en vrai sous Arch : le même paquet est cité plusieurs fois.
        let reel = ":: removing ncurses breaks dependency 'ncurses' required by bash\n:: removing ncurses breaks dependency 'libncursesw.so=6-64' required by readline\n:: removing ncurses breaks dependency 'ncurses' required by readline\nerror: failed to prepare transaction (could not satisfy dependencies)\n";
        assert_eq!(Pacman.emportes("ncurses", false, reel).unwrap_err(), "d'autres paquets en dépendent : bash, readline");
        assert!(Pacman.emportes("x", false, "error: target not found: x\n").unwrap_err().contains("target not found"));
        assert_eq!(Pacman.emportes("kpat", true, "kpat\n"), Ok(vec![]));
    }

    #[test]
    fn un_retrait_qui_n_emporte_rien_est_accepte() {
        assert_eq!(verifier_retrait("aisleriot", &[]), Ok(()));
    }

    #[test]
    fn un_retrait_qui_n_emporte_qu_un_paquet_chapeau_est_accepte() {
        assert_eq!(verifier_retrait("gnome-mines", &["gnome-games".to_string()]), Ok(()));
    }

    #[test]
    fn un_retrait_qui_emporterait_le_bureau_est_refuse_en_disant_pourquoi() {
        let emportes = Apt.emportes("yelp", true, APT_BUREAU).unwrap();
        let erreur = verifier_retrait("yelp", &emportes).unwrap_err();
        assert!(erreur.contains("ubuntu-desktop") && erreur.contains("mutter"), "{erreur}");
    }

    #[test]
    fn un_paquet_protege_n_est_jamais_retire() {
        for nom in ["systemd", "linux-image-6.8.0-45-generic", "ubuntu-desktop", "sudo", "glibc", "NetworkManager"] {
            assert!(verifier_retrait(nom, &[]).is_err(), "{nom}");
        }
    }

    #[test]
    fn un_retrait_signale_en_erreur_mais_effectif_compte_comme_reussi() {
        // Vu en vrai sous Ubuntu : apt retire le paquet puis échoue sur un autre paquet déjà cassé.
        assert_eq!(issue_retrait(false, false, "E: Sub-process /usr/bin/dpkg returned an error code (1)\n"), Ok(()));
        assert_eq!(issue_retrait(true, false, ""), Ok(()));
    }

    #[test]
    fn un_retrait_en_erreur_dont_le_paquet_est_toujours_la_est_un_echec() {
        let erreur = issue_retrait(false, true, "bla\nE: Could not get lock /var/lib/dpkg/lock-frontend\n").unwrap_err();
        assert!(erreur.contains("Could not get lock"), "{erreur}");
    }

    #[test]
    fn les_noms_de_paquets_douteux_sont_refuses() {
        for nom in ["aisleriot", "gnome-2048", "libstdc++6", "NetworkManager", "python3.11", "a_b"] {
            assert!(nom_valide(nom), "{nom}");
        }
        for nom in ["", "-y", "--purge", "a b", "a;b", "a/b", "a$(x)", "é", "a\nb"] {
            assert!(!nom_valide(nom), "{nom:?}");
        }
    }

    #[test]
    fn les_commandes_ne_touchent_qu_au_paquet_demande() {
        for gestionnaire in [Apt, Rpm, Pacman] {
            for commande in [gestionnaire.simulation("aisleriot"), gestionnaire.retrait("aisleriot")] {
                assert_eq!(commande.arguments.last().map(String::as_str), Some("aisleriot"), "{commande:?}");
                let texte = commande.arguments.join(" ");
                for interdit in ["autoremove", "purge", "--cascade", "-Rs", "-Rc", "--recursive", "--nodeps"] {
                    assert!(!texte.contains(interdit), "{gestionnaire:?} : {texte}");
                }
            }
        }
    }

    #[test]
    fn les_simulations_ne_peuvent_rien_modifier() {
        assert!(Apt.simulation("x").arguments.contains(&"-s".to_string()));
        assert!(Rpm.simulation("x").arguments.contains(&"--test".to_string()));
        assert!(Pacman.simulation("x").arguments.contains(&"--print".to_string()));
    }

    #[test]
    fn apt_ne_pose_aucune_question_pendant_le_retrait() {
        let commande = Apt.retrait("aisleriot");
        assert!(commande.arguments.contains(&"-y".to_string()));
        assert!(commande.environnement.contains(&("DEBIAN_FRONTEND", "noninteractive")));
    }

    #[test]
    fn la_commande_de_reinstallation_est_celle_de_la_famille() {
        let noms = ["aisleriot".to_string(), "gnome-mines".to_string()];
        assert_eq!(Apt.reinstallation(&noms), "sudo apt install aisleriot gnome-mines");
        assert_eq!(Pacman.reinstallation(&noms), "sudo pacman -S aisleriot gnome-mines");
        assert!(Rpm.reinstallation(&noms).contains("dnf install aisleriot gnome-mines"));
    }
}
