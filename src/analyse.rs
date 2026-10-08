//! Regarde l'état du PC et en déduit ce qu'il reste à faire pour chaque élément du catalogue.

use crate::catalogue::{est_protege, Action, Element, Ruche};
use crate::motif::correspond;
use crate::systeme::{Systeme, DEMARRAGE_DESACTIVE};

/// Une opération précise à faire sur ce PC.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Travail {
    /// Retirer l'appli qui porte exactement ce nom.
    Paquet(String),
    Registre { ruche: Ruche, cle: String, nom: String, valeur: u32 },
    Service(String),
    Tache(String),
    OneDrive,
}

impl Travail {
    /// Phrase affichée dans l'aperçu.
    pub fn decrire(&self) -> String {
        match self {
            Travail::Paquet(nom) => format!("Retirer l'appli {nom}"),
            Travail::Registre { ruche, cle, nom, valeur } => {
                let racine = match ruche {
                    Ruche::Utilisateur => "HKCU",
                    Ruche::Machine => "HKLM",
                };
                format!(r"Régler {racine}\{cle}\{nom} sur {valeur}")
            }
            Travail::Service(nom) => format!("Désactiver le service {nom}"),
            Travail::Tache(chemin) => format!("Désactiver la tâche planifiée {chemin}"),
            Travail::OneDrive => "Désinstaller OneDrive".to_string(),
        }
    }
}

/// Un élément du catalogue pour lequel il reste quelque chose à faire.
#[derive(Clone, Debug)]
pub struct Ligne<'a> {
    pub element: &'a Element,
    pub travaux: Vec<Travail>,
}

#[derive(Debug, Default)]
pub struct Analyse<'a> {
    pub lignes: Vec<Ligne<'a>>,
    /// Éléments déjà réglés ou absents de ce PC.
    pub masques: usize,
    pub avertissements: Vec<String>,
}

pub fn analyser<'a>(sys: &dyn Systeme, catalogue: &'a [Element]) -> Analyse<'a> {
    let mut analyse = Analyse::default();
    let paquets = sys.paquets().unwrap_or_else(|e| {
        analyse.avertissements.push(format!("Liste des applis illisible : {e}"));
        Vec::new()
    });

    for element in catalogue {
        let mut travaux = Vec::new();
        let mut retire_une_appli = false;
        let mut appli_presente = false;
        for action in element.actions {
            match *action {
                Action::Paquet(motif) => {
                    retire_une_appli = true;
                    for nom in &paquets {
                        let travail = Travail::Paquet(nom.clone());
                        if correspond(motif, nom) && !est_protege(nom) && !travaux.contains(&travail) {
                            travaux.push(travail);
                            appli_presente = true;
                        }
                    }
                }
                Action::OneDrive => {
                    retire_une_appli = true;
                    match sys.onedrive_present() {
                        Ok(true) => {
                            travaux.push(Travail::OneDrive);
                            appli_presente = true;
                        }
                        Ok(false) => {}
                        Err(e) => analyse.avertissements.push(format!("OneDrive : {e}")),
                    }
                }
                Action::Registre { ruche, cle, nom, valeur } => match sys.lire_valeur(ruche, cle, nom) {
                    Ok(actuelle) => {
                        if actuelle.and_then(|v| v.en_dword()) != Some(valeur) {
                            travaux.push(Travail::Registre { ruche, cle: cle.into(), nom: nom.into(), valeur });
                        }
                    }
                    Err(e) => analyse.avertissements.push(format!("{} : {e}", element.nom)),
                },
                Action::Service(nom) => match sys.demarrage_service(nom) {
                    Ok(Some(demarrage)) if demarrage != DEMARRAGE_DESACTIVE => {
                        travaux.push(Travail::Service(nom.into()));
                    }
                    Ok(_) => {}
                    Err(e) => analyse.avertissements.push(format!("{} : {e}", element.nom)),
                },
                Action::Tache(chemin) => match sys.tache_active(chemin) {
                    Ok(Some(true)) => travaux.push(Travail::Tache(chemin.into())),
                    Ok(_) => {}
                    Err(e) => analyse.avertissements.push(format!("{} : {e}", element.nom)),
                },
            }
        }
        // Les réglages qui accompagnent une appli n'ont pas de sens sans elle.
        if travaux.is_empty() || (retire_une_appli && !appli_presente) {
            analyse.masques += 1;
        } else {
            analyse.lignes.push(Ligne { element, travaux });
        }
    }
    analyse
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::catalogue::{Categorie, Risque};
    use crate::faux::FauxSysteme;
    use crate::systeme::*;

    pub(crate) const CLE: &str = r"Software\Essai";

    pub(crate) fn element(actions: &'static [Action]) -> Element {
        Element {
            id: "essai",
            categorie: Categorie::Applis,
            nom: "Essai",
            explication: "Élément d'essai.",
            risque: Risque::SansRisque,
            actions,
        }
    }

    const REGLAGE: Action = Action::Registre { ruche: Ruche::Utilisateur, cle: CLE, nom: "A", valeur: 0 };

    fn travaux(sys: &FauxSysteme, actions: &'static [Action]) -> Vec<Travail> {
        let catalogue = [element(actions)];
        let analyse = analyser(sys, &catalogue);
        analyse.lignes.into_iter().flat_map(|l| l.travaux).collect()
    }

    fn avec_paquets(noms: &[&str]) -> FauxSysteme {
        let mut sys = FauxSysteme::vide();
        sys.paquets = noms.iter().map(|s| s.to_string()).collect();
        sys
    }

    #[test]
    fn une_appli_installee_est_a_retirer() {
        let sys = avec_paquets(&["Microsoft.SkypeApp", "Autre.Appli"]);
        assert_eq!(
            travaux(&sys, &[Action::Paquet("Microsoft.SkypeApp")]),
            vec![Travail::Paquet("Microsoft.SkypeApp".into())]
        );
    }

    #[test]
    fn une_appli_absente_ne_donne_aucune_ligne() {
        let sys = avec_paquets(&["Autre.Appli"]);
        let catalogue = [element(&[Action::Paquet("Microsoft.SkypeApp")])];
        let analyse = analyser(&sys, &catalogue);
        assert!(analyse.lignes.is_empty());
        assert_eq!(analyse.masques, 1);
    }

    #[test]
    fn un_motif_designe_toutes_les_applis_qui_lui_correspondent() {
        let sys = avec_paquets(&["king.com.CandyCrushSaga", "Microsoft.SkypeApp", "king.com.FarmHeroesSaga"]);
        assert_eq!(
            travaux(&sys, &[Action::Paquet("king.com.*")]),
            vec![
                Travail::Paquet("king.com.CandyCrushSaga".into()),
                Travail::Paquet("king.com.FarmHeroesSaga".into()),
            ]
        );
    }

    #[test]
    fn une_appli_n_est_proposee_qu_une_fois_meme_si_deux_motifs_la_designent() {
        let sys = avec_paquets(&["king.com.CandyCrushSaga"]);
        assert_eq!(
            travaux(&sys, &[Action::Paquet("king.com.*"), Action::Paquet("*CandyCrush*")]),
            vec![Travail::Paquet("king.com.CandyCrushSaga".into())]
        );
    }

    #[test]
    fn une_appli_protegee_n_est_jamais_proposee() {
        let sys = avec_paquets(&["Microsoft.WindowsStore", "Microsoft.SkypeApp"]);
        assert_eq!(
            travaux(&sys, &[Action::Paquet("Microsoft.*")]),
            vec![Travail::Paquet("Microsoft.SkypeApp".into())]
        );
    }

    #[test]
    fn un_reglage_absent_ou_different_est_a_faire() {
        let attendu = vec![Travail::Registre {
            ruche: Ruche::Utilisateur,
            cle: CLE.into(),
            nom: "A".into(),
            valeur: 0,
        }];
        let mut sys = FauxSysteme::vide();
        assert_eq!(travaux(&sys, &[REGLAGE]), attendu);
        sys.poser(Ruche::Utilisateur, CLE, "A", 1);
        assert_eq!(travaux(&sys, &[REGLAGE]), attendu);
    }

    #[test]
    fn un_reglage_deja_en_place_n_est_pas_propose() {
        let mut sys = FauxSysteme::vide();
        sys.poser(Ruche::Utilisateur, CLE, "A", 0);
        assert!(travaux(&sys, &[REGLAGE]).is_empty());
    }

    #[test]
    fn un_service_actif_est_a_desactiver() {
        let mut sys = FauxSysteme::vide();
        sys.services.insert("DiagTrack".into(), DEMARRAGE_AUTO);
        assert_eq!(travaux(&sys, &[Action::Service("DiagTrack")]), vec![Travail::Service("DiagTrack".into())]);
    }

    #[test]
    fn un_service_deja_desactive_ou_absent_n_est_pas_propose() {
        let mut sys = FauxSysteme::vide();
        assert!(travaux(&sys, &[Action::Service("DiagTrack")]).is_empty());
        sys.services.insert("DiagTrack".into(), DEMARRAGE_DESACTIVE);
        assert!(travaux(&sys, &[Action::Service("DiagTrack")]).is_empty());
    }

    #[test]
    fn une_tache_active_est_a_desactiver() {
        let mut sys = FauxSysteme::vide();
        sys.taches.insert(r"\T".into(), true);
        assert_eq!(travaux(&sys, &[Action::Tache(r"\T")]), vec![Travail::Tache(r"\T".into())]);
    }

    #[test]
    fn une_tache_deja_desactivee_ou_absente_n_est_pas_proposee() {
        let mut sys = FauxSysteme::vide();
        assert!(travaux(&sys, &[Action::Tache(r"\T")]).is_empty());
        sys.taches.insert(r"\T".into(), false);
        assert!(travaux(&sys, &[Action::Tache(r"\T")]).is_empty());
    }

    #[test]
    fn onedrive_n_est_propose_que_s_il_est_installe() {
        let mut sys = FauxSysteme::vide();
        assert!(travaux(&sys, &[Action::OneDrive]).is_empty());
        sys.onedrive = true;
        assert_eq!(travaux(&sys, &[Action::OneDrive]), vec![Travail::OneDrive]);
    }

    #[test]
    fn les_reglages_d_une_appli_absente_ne_sont_pas_proposes() {
        // Comme Xbox : sans l'appli, ses réglages d'accompagnement n'ont pas lieu d'être.
        let sys = FauxSysteme::vide();
        assert!(travaux(&sys, &[Action::Paquet("Microsoft.XboxApp"), REGLAGE]).is_empty());
    }

    #[test]
    fn les_reglages_d_une_appli_presente_l_accompagnent() {
        let sys = avec_paquets(&["Microsoft.XboxApp"]);
        assert_eq!(travaux(&sys, &[Action::Paquet("Microsoft.XboxApp"), REGLAGE]).len(), 2);
    }

    #[test]
    fn si_la_liste_des_applis_est_illisible_les_reglages_restent_analyses() {
        let mut sys = avec_paquets(&["Microsoft.SkypeApp"]);
        sys.paquets_illisibles = true;
        let catalogue = [element(&[Action::Paquet("Microsoft.SkypeApp")]), element(&[REGLAGE])];
        let analyse = analyser(&sys, &catalogue);
        assert_eq!(analyse.lignes.len(), 1);
        assert_eq!(analyse.avertissements.len(), 1);
    }

    #[test]
    fn la_description_d_un_reglage_donne_son_emplacement_complet() {
        let travail = Travail::Registre { ruche: Ruche::Machine, cle: CLE.into(), nom: "A".into(), valeur: 1 };
        assert_eq!(travail.decrire(), r"Régler HKLM\Software\Essai\A sur 1");
    }

    #[test]
    fn la_description_d_une_appli_donne_son_nom() {
        assert_eq!(Travail::Paquet("Microsoft.SkypeApp".into()).decrire(), "Retirer l'appli Microsoft.SkypeApp");
    }
}
