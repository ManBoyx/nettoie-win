//! L'assistant : la petite partie du programme qui tourne en administrateur sous Linux.
//!
//! La fenêtre tourne avec le compte ordinaire et lui envoie des demandes, une
//! par ligne. L'assistant n'exécute que ce que le catalogue connaît : même si
//! on lui écrivait autre chose, il refuserait.

use std::io::{BufRead, Write};

use serde::{Deserialize, Serialize};

use crate::catalogue::{est_protege, Action, Element, Ruche};
use crate::gestionnaire::nom_valide;
use crate::motif::correspond;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Ordre {
    Activer,
    Desactiver,
    Demarrer,
    Arreter,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Demande {
    RetirerPaquet { nom: String },
    Service { nom: String, ordre: Ordre },
    /// `valeur` à `None` : retirer la ligne.
    Fichier { chemin: String, nom: String, valeur: Option<String> },
    Pro { nom: String, valeur: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Reponse {
    Fait,
    Erreur(String),
}

/// Dit si le catalogue prévoit cette opération, avec des valeurs sans danger.
pub fn autorisee(demande: &Demande, catalogue: &[Element]) -> bool {
    let actions = || catalogue.iter().flat_map(|e| e.actions.iter());
    let reglage_connu = |ruche: Ruche, cle: &str, nom: &str| {
        actions().any(|a| matches!(a, Action::Registre { ruche: r, cle: c, nom: n, .. } if *r == ruche && *c == cle && *n == nom))
    };
    match demande {
        Demande::RetirerPaquet { nom } => {
            nom_valide(nom)
                && !est_protege(nom)
                && actions().any(|a| matches!(a, Action::Paquet(motif) if correspond(motif, nom)))
        }
        Demande::Service { nom, .. } => actions().any(|a| matches!(a, Action::Service(n) if n == nom)),
        Demande::Fichier { chemin, nom, valeur } => {
            reglage_connu(Ruche::Fichier, chemin, nom) && valeur.as_deref().is_none_or(valeur_sans_danger)
        }
        Demande::Pro { nom, valeur } => {
            reglage_connu(Ruche::Pro, "config", nom) && matches!(valeur.as_str(), "True" | "False")
        }
    }
}

/// Une valeur de fichier de configuration : courte, sur une ligne, sans rien
/// qu'un interpréteur de commandes pourrait exécuter.
fn valeur_sans_danger(valeur: &str) -> bool {
    valeur.len() <= 64 && valeur.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '"' | '.' | '_' | '-'))
}

/// Lit les demandes ligne par ligne, exécute celles qui sont autorisées et
/// répond à chacune. S'arrête quand l'entrée se ferme.
pub fn servir(
    entree: impl BufRead,
    mut sortie: impl Write,
    catalogue: &[Element],
    executer: &mut dyn FnMut(&Demande) -> Result<(), String>,
) {
    for ligne in entree.lines() {
        let Ok(ligne) = ligne else {
            break;
        };
        if ligne.trim().is_empty() {
            continue;
        }
        let reponse = match serde_json::from_str::<Demande>(&ligne) {
            Err(e) => Reponse::Erreur(format!("demande illisible : {e}")),
            Ok(demande) if !autorisee(&demande, catalogue) => {
                Reponse::Erreur("opération refusée : elle ne fait pas partie de ce que le programme sait faire".to_string())
            }
            Ok(demande) => match executer(&demande) {
                Ok(()) => Reponse::Fait,
                Err(e) => Reponse::Erreur(e),
            },
        };
        let texte = serde_json::to_string(&reponse).unwrap_or_else(|_| "{\"Erreur\":\"réponse illisible\"}".to_string());
        if writeln!(sortie, "{texte}").and_then(|()| sortie.flush()).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue_linux::catalogue;

    fn paquet(nom: &str) -> Demande {
        Demande::RetirerPaquet { nom: nom.into() }
    }

    fn fichier(chemin: &str, nom: &str, valeur: Option<&str>) -> Demande {
        Demande::Fichier { chemin: chemin.into(), nom: nom.into(), valeur: valeur.map(String::from) }
    }

    #[test]
    fn un_paquet_du_catalogue_peut_etre_retire() {
        assert!(autorisee(&paquet("aisleriot"), catalogue()));
        assert!(autorisee(&paquet("ubuntu-report"), catalogue()));
    }

    #[test]
    fn un_paquet_hors_catalogue_est_refuse() {
        for nom in ["systemd", "bash", "firefox", "openssh-server", "aisleriot extra", "-y", "aisleriot*", ""] {
            assert!(!autorisee(&paquet(nom), catalogue()), "{nom:?}");
        }
    }

    #[test]
    fn seuls_les_services_du_catalogue_sont_acceptes() {
        let service = |nom: &str| Demande::Service { nom: nom.into(), ordre: Ordre::Desactiver };
        assert!(autorisee(&service("apport.service"), catalogue()));
        assert!(autorisee(&service("abrtd.service"), catalogue()));
        for nom in ["ssh.service", "NetworkManager.service", "apport.service; reboot", "apport"] {
            assert!(!autorisee(&service(nom), catalogue()), "{nom:?}");
        }
    }

    #[test]
    fn seuls_les_fichiers_et_cles_du_catalogue_sont_acceptes() {
        assert!(autorisee(&fichier("/etc/default/apport", "enabled", Some("0")), catalogue()));
        assert!(autorisee(&fichier("/etc/default/apport", "enabled", None), catalogue()));
        assert!(autorisee(&fichier("/etc/popularity-contest.conf", "PARTICIPATE", Some("\"yes\"")), catalogue()));
        assert!(!autorisee(&fichier("/etc/passwd", "root", Some("x")), catalogue()));
        assert!(!autorisee(&fichier("/etc/default/apport", "autre", Some("0")), catalogue()));
        assert!(!autorisee(&fichier("/etc/default/../shadow", "enabled", Some("0")), catalogue()));
    }

    #[test]
    fn une_valeur_qui_ajouterait_des_lignes_ou_des_commandes_est_refusee() {
        for valeur in ["0\nPermitRootLogin=yes", "$(reboot)", "`id`", "0; rm -rf /", "a b", &"x".repeat(100)] {
            assert!(!autorisee(&fichier("/etc/default/apport", "enabled", Some(valeur)), catalogue()), "{valeur:?}");
        }
    }

    #[test]
    fn ubuntu_pro_n_accepte_que_le_reglage_prevu_et_vrai_ou_faux() {
        let pro = |nom: &str, valeur: &str| Demande::Pro { nom: nom.into(), valeur: valeur.into() };
        assert!(autorisee(&pro("apt_news", "False"), catalogue()));
        assert!(autorisee(&pro("apt_news", "True"), catalogue()));
        assert!(!autorisee(&pro("apt_news", "False --assume-yes"), catalogue()));
        assert!(!autorisee(&pro("http_proxy", "False"), catalogue()));
    }

    /// Fait tourner l'assistant sur des lignes données et rend ses réponses et ce qu'il a exécuté.
    fn dialogue(lignes: &[String], echec: Option<&str>) -> (Vec<Reponse>, Vec<Demande>) {
        let entree = lignes.join("\n") + "\n";
        let mut sortie = Vec::new();
        let mut executees = Vec::new();
        servir(entree.as_bytes(), &mut sortie, catalogue(), &mut |demande| {
            executees.push(demande.clone());
            match echec {
                Some(message) => Err(message.to_string()),
                None => Ok(()),
            }
        });
        let reponses = String::from_utf8(sortie)
            .unwrap()
            .lines()
            .map(|ligne| serde_json::from_str(ligne).expect("réponse lisible"))
            .collect();
        (reponses, executees)
    }

    fn ligne(demande: &Demande) -> String {
        serde_json::to_string(demande).unwrap()
    }

    #[test]
    fn une_demande_autorisee_est_executee_et_confirmee() {
        let (reponses, executees) = dialogue(&[ligne(&paquet("aisleriot"))], None);
        assert_eq!(reponses, vec![Reponse::Fait]);
        assert_eq!(executees, vec![paquet("aisleriot")]);
    }

    #[test]
    fn une_demande_refusee_n_est_jamais_executee() {
        let (reponses, executees) = dialogue(&[ligne(&paquet("systemd"))], None);
        assert!(matches!(reponses[0], Reponse::Erreur(_)));
        assert!(executees.is_empty());
    }

    #[test]
    fn une_ligne_illisible_donne_une_erreur_sans_arreter_l_assistant() {
        let (reponses, executees) = dialogue(&["n'importe quoi".to_string(), ligne(&paquet("aisleriot"))], None);
        assert!(matches!(reponses[0], Reponse::Erreur(_)));
        assert_eq!(reponses[1], Reponse::Fait);
        assert_eq!(executees.len(), 1);
    }

    #[test]
    fn l_echec_d_une_operation_est_rapporte_avec_son_message() {
        let (reponses, _) = dialogue(&[ligne(&paquet("aisleriot"))], Some("verrou de dpkg occupé"));
        assert_eq!(reponses, vec![Reponse::Erreur("verrou de dpkg occupé".to_string())]);
    }

    #[test]
    fn chaque_demande_recoit_exactement_une_reponse() {
        let lignes = vec![ligne(&paquet("aisleriot")), ligne(&paquet("bash")), ligne(&paquet("gnome-mines"))];
        let (reponses, executees) = dialogue(&lignes, None);
        assert_eq!(reponses.len(), 3);
        assert_eq!(executees.len(), 2);
    }
}
