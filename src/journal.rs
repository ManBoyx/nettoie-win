//! Trace de chaque changement, écrite avant de le faire, pour pouvoir revenir en arrière.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::catalogue::{Action, Element};
use crate::systeme::{Ruche, Systeme, ValeurBrute, DEMARRAGE_AUTO};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Entree {
    /// `avant` vaut `None` si la valeur n'existait pas.
    Registre { ruche: Ruche, cle: String, nom: String, avant: Option<ValeurBrute> },
    Service { nom: String, avant: u32 },
    Tache { chemin: String, avant: bool },
    /// Pour mémoire : une appli retirée ne se remet pas toute seule.
    PaquetRetire { nom: String },
    OneDriveRetire,
}

#[derive(Debug)]
pub struct Journal {
    fichier: Option<PathBuf>,
    entrees: Vec<Entree>,
}

/// Ce qu'a donné une annulation.
#[derive(Debug, Default, PartialEq)]
pub struct Annulation {
    /// Nombre de réglages remis comme avant.
    pub remis: usize,
    pub echecs: Vec<String>,
    /// Applis retirées, à réinstaller à la main.
    pub a_reinstaller: Vec<String>,
}

impl Journal {
    /// Journal qui n'est écrit nulle part.
    pub fn en_memoire() -> Self {
        Journal { fichier: None, entrees: Vec::new() }
    }

    /// Ouvre le journal enregistré dans `fichier`. Un fichier absent donne un journal vide.
    pub fn ouvrir(fichier: PathBuf) -> Result<Self, String> {
        let entrees = match std::fs::read_to_string(&fichier) {
            Ok(texte) => serde_json::from_str(&texte)
                .map_err(|e| format!("journal illisible ({}) : {e}", fichier.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
            // Le dossier n'existe pas encore non plus : rien n'a jamais été noté.
            Err(_) if !fichier.exists() => Vec::new(),
            Err(e) => return Err(format!("journal illisible ({}) : {e}", fichier.display())),
        };
        Ok(Journal { fichier: Some(fichier), entrees })
    }

    fn enregistrer(&self) -> Result<(), String> {
        let Some(fichier) = &self.fichier else {
            return Ok(());
        };
        let erreur = |e: std::io::Error| format!("journal non enregistré ({}) : {e}", fichier.display());
        if let Some(dossier) = fichier.parent() {
            std::fs::create_dir_all(dossier).map_err(erreur)?;
        }
        let texte = serde_json::to_string_pretty(&self.entrees).map_err(|e| e.to_string())?;
        // Écriture à côté puis remplacement : une coupure ne laisse jamais un journal à moitié écrit.
        let provisoire = fichier.with_extension("json.tmp");
        std::fs::write(&provisoire, texte).map_err(erreur)?;
        std::fs::rename(&provisoire, fichier).map_err(erreur)
    }

    pub fn entrees(&self) -> &[Entree] {
        &self.entrees
    }

    pub fn est_vide(&self) -> bool {
        self.entrees.is_empty()
    }

    /// Ajoute une entrée et enregistre aussitôt.
    pub fn ajouter(&mut self, entree: Entree) -> Result<(), String> {
        self.entrees.push(entree);
        if let Err(e) = self.enregistrer() {
            self.entrees.pop();
            return Err(e);
        }
        Ok(())
    }

    /// Retire la dernière entrée (le changement qu'elle annonçait n'a pas eu lieu).
    pub fn retirer_derniere(&mut self) -> Result<(), String> {
        self.entrees.pop();
        self.enregistrer()
    }
}

/// Dit si l'entrée désigne bien un réglage que le catalogue sait modifier.
///
/// Le journal est un simple fichier : sans ce contrôle, quelqu'un qui le
/// modifierait ferait écrire n'importe quoi dans le registre par un programme
/// lancé en administrateur.
pub fn connue(entree: &Entree, catalogue: &[Element]) -> bool {
    let actions = || catalogue.iter().flat_map(|e| e.actions.iter());
    match entree {
        Entree::Registre { ruche, cle, nom, .. } => actions().any(|a| match a {
            Action::Registre { ruche: r, cle: c, nom: n, .. } => {
                r == ruche && c.eq_ignore_ascii_case(cle) && n.eq_ignore_ascii_case(nom)
            }
            _ => false,
        }),
        Entree::Service { nom, .. } => {
            actions().any(|a| matches!(a, Action::Service(n) if n.eq_ignore_ascii_case(nom)))
        }
        Entree::Tache { chemin, .. } => {
            actions().any(|a| matches!(a, Action::Tache(c) if c.eq_ignore_ascii_case(chemin)))
        }
        Entree::PaquetRetire { .. } | Entree::OneDriveRetire => true,
    }
}

/// Remet tout ce que le journal a noté, du plus récent au plus ancien.
/// Les entrées qui n'ont pas pu être remises restent dans le journal ;
/// celles que le catalogue ne connaît pas sont écartées sans être appliquées.
pub fn annuler(sys: &mut dyn Systeme, journal: &mut Journal, catalogue: &[Element]) -> Annulation {

    let mut bilan = Annulation::default();
    let mut restantes = Vec::new();
    for entree in journal.entrees.drain(..).rev() {
        if !connue(&entree, catalogue) {
            bilan.echecs.push("Entrée du journal ignorée : elle ne correspond à aucun réglage connu.".to_string());
            continue;
        }
        let reponse = match &entree {
            Entree::Registre { ruche, cle, nom, avant } => match avant {
                Some(valeur) => sys.ecrire_valeur(*ruche, cle, nom, valeur),
                None => sys.supprimer_valeur(*ruche, cle, nom),
            },
            Entree::Service { nom, avant } => sys.regler_service(nom, *avant).map(|()| {
                if *avant == DEMARRAGE_AUTO {
                    // Sans gravité s'il ne repart pas tout de suite : il repartira au redémarrage.
                    let _ = sys.demarrer_service(nom);
                }
            }),
            Entree::Tache { chemin, avant } => sys.activer_tache(chemin, *avant),
            Entree::PaquetRetire { nom } => {
                bilan.a_reinstaller.push(nom.clone());
                continue;
            }
            Entree::OneDriveRetire => {
                bilan.a_reinstaller.push("OneDrive".to_string());
                continue;
            }
        };
        match reponse {
            Ok(()) => bilan.remis += 1,
            Err(e) => {
                bilan.echecs.push(e);
                restantes.push(entree);
            }
        }
    }
    restantes.reverse();
    bilan.a_reinstaller.reverse();
    journal.entrees = restantes;
    if let Err(e) = journal.enregistrer() {
        bilan.echecs.push(e);
    }
    bilan
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::faux::FauxSysteme;
    use crate::systeme::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    /// Dossier temporaire neuf pour un test.
    pub(crate) fn dossier_temporaire() -> PathBuf {
        static NUMERO: AtomicU32 = AtomicU32::new(0);
        let dossier = std::env::temp_dir().join(format!(
            "nettoie-win-test-{}-{}",
            std::process::id(),
            NUMERO.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dossier).unwrap();
        dossier
    }

    const CLE: &str = r"Software\Essai";

    /// Catalogue qui connaît tout ce que ces tests remettent.
    fn essai() -> [Element; 1] {
        [crate::analyse::tests::element(&[
            Action::Registre { ruche: Ruche::Utilisateur, cle: CLE, nom: "A", valeur: Valeur::Nombre(0) },
            Action::Registre { ruche: Ruche::Utilisateur, cle: CLE, nom: "B", valeur: Valeur::Nombre(0) },
            Action::Service("DiagTrack"),
            Action::Service("dmwappushservice"),
            Action::Tache(r"\T"),
        ])]
    }

    fn registre(nom: &str, avant: Option<ValeurBrute>) -> Entree {
        Entree::Registre { ruche: Ruche::Utilisateur, cle: CLE.into(), nom: nom.into(), avant }
    }

    #[test]
    fn un_fichier_absent_donne_un_journal_vide() {
        let journal = Journal::ouvrir(dossier_temporaire().join("journal.json")).unwrap();
        assert!(journal.est_vide());
    }

    #[test]
    fn une_entree_ajoutee_se_retrouve_a_la_reouverture() {
        let fichier = dossier_temporaire().join("journal.json");
        let mut journal = Journal::ouvrir(fichier.clone()).unwrap();
        journal.ajouter(registre("A", Some(ValeurBrute::dword(1)))).unwrap();
        journal.ajouter(Entree::Service { nom: "DiagTrack".into(), avant: 2 }).unwrap();

        let relu = Journal::ouvrir(fichier).unwrap();
        assert_eq!(relu.entrees(), journal.entrees());
        assert_eq!(relu.entrees().len(), 2);
    }

    #[test]
    fn le_dossier_du_journal_est_cree_au_besoin() {
        let fichier = dossier_temporaire().join("sous").join("dossier").join("journal.json");
        let mut journal = Journal::ouvrir(fichier.clone()).unwrap();
        journal.ajouter(Entree::OneDriveRetire).unwrap();
        assert!(fichier.exists());
    }

    #[test]
    fn un_journal_illisible_est_signale_au_lieu_d_etre_ecrase() {
        let fichier = dossier_temporaire().join("journal.json");
        std::fs::write(&fichier, "ceci n'est pas du JSON").unwrap();
        assert!(Journal::ouvrir(fichier.clone()).is_err());
        assert_eq!(std::fs::read_to_string(fichier).unwrap(), "ceci n'est pas du JSON");
    }

    #[test]
    fn une_entree_qui_ne_peut_pas_etre_enregistree_n_est_pas_gardee() {
        let dossier = dossier_temporaire();
        std::fs::write(dossier.join("fichier"), "x").unwrap();
        // Le « dossier » du journal est un fichier : l'écriture ne peut pas réussir.
        let mut journal = Journal::ouvrir(dossier.join("fichier").join("journal.json")).unwrap();
        assert!(journal.ajouter(Entree::OneDriveRetire).is_err());
        assert!(journal.est_vide());
    }

    #[test]
    fn retirer_la_derniere_entree_l_enleve_aussi_du_fichier() {
        let fichier = dossier_temporaire().join("journal.json");
        let mut journal = Journal::ouvrir(fichier.clone()).unwrap();
        journal.ajouter(registre("A", None)).unwrap();
        journal.ajouter(registre("B", None)).unwrap();
        journal.retirer_derniere().unwrap();
        assert_eq!(Journal::ouvrir(fichier).unwrap().entrees(), &[registre("A", None)]);
    }

    #[test]
    fn annuler_remet_l_ancienne_valeur_du_registre() {
        let mut sys = FauxSysteme::vide();
        sys.poser(Ruche::Utilisateur, CLE, "A", 0);
        let mut journal = Journal::en_memoire();
        journal.ajouter(registre("A", Some(ValeurBrute::dword(7)))).unwrap();

        let bilan = annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys.valeur(Ruche::Utilisateur, CLE, "A"), Some(7));
        assert_eq!(bilan.remis, 1);
        assert!(journal.est_vide());
    }

    #[test]
    fn annuler_supprime_une_valeur_qui_n_existait_pas_avant() {
        let mut sys = FauxSysteme::vide();
        sys.poser(Ruche::Utilisateur, CLE, "A", 0);
        let mut journal = Journal::en_memoire();
        journal.ajouter(registre("A", None)).unwrap();

        annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys.lire_valeur(Ruche::Utilisateur, CLE, "A").unwrap(), None);
    }

    #[test]
    fn annuler_remet_une_valeur_qui_n_etait_pas_un_nombre() {
        let texte = ValeurBrute { genre: 1, octets: vec![b'o', 0, b'k', 0, 0, 0] };
        let mut sys = FauxSysteme::vide();
        sys.poser(Ruche::Utilisateur, CLE, "A", 0);
        let mut journal = Journal::en_memoire();
        journal.ajouter(registre("A", Some(texte.clone()))).unwrap();

        annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys.lire_valeur(Ruche::Utilisateur, CLE, "A").unwrap(), Some(texte));
    }

    #[test]
    fn annuler_remonte_du_plus_recent_au_plus_ancien() {
        // La même valeur a été changée deux fois : c'est la plus ancienne trace qui doit gagner.
        let mut sys = FauxSysteme::vide();
        sys.poser(Ruche::Utilisateur, CLE, "A", 0);
        let mut journal = Journal::en_memoire();
        journal.ajouter(registre("A", Some(ValeurBrute::dword(1)))).unwrap();
        journal.ajouter(registre("A", Some(ValeurBrute::dword(2)))).unwrap();

        annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys.valeur(Ruche::Utilisateur, CLE, "A"), Some(1));
    }

    #[test]
    fn annuler_reactive_et_relance_un_service_automatique() {
        let mut sys = FauxSysteme::vide();
        sys.services.insert("DiagTrack".into(), DEMARRAGE_DESACTIVE);
        let mut journal = Journal::en_memoire();
        journal.ajouter(Entree::Service { nom: "DiagTrack".into(), avant: DEMARRAGE_AUTO }).unwrap();

        annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys.services["DiagTrack"], DEMARRAGE_AUTO);
        assert!(sys.en_marche.contains("DiagTrack"));
    }

    #[test]
    fn annuler_ne_lance_pas_un_service_qui_etait_manuel() {
        let mut sys = FauxSysteme::vide();
        sys.services.insert("dmwappushservice".into(), DEMARRAGE_DESACTIVE);
        let mut journal = Journal::en_memoire();
        journal
            .ajouter(Entree::Service { nom: "dmwappushservice".into(), avant: DEMARRAGE_MANUEL })
            .unwrap();

        annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys.services["dmwappushservice"], DEMARRAGE_MANUEL);
        assert!(!sys.en_marche.contains("dmwappushservice"));
    }

    #[test]
    fn annuler_reactive_une_tache() {
        let mut sys = FauxSysteme::vide();
        sys.taches.insert(r"\T".into(), false);
        let mut journal = Journal::en_memoire();
        journal.ajouter(Entree::Tache { chemin: r"\T".into(), avant: true }).unwrap();

        annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys.taches[r"\T"], true);
    }

    #[test]
    fn les_applis_retirees_sont_listees_mais_pas_reinstallees() {
        let mut sys = FauxSysteme::vide();
        let mut journal = Journal::en_memoire();
        journal.ajouter(Entree::PaquetRetire { nom: "Microsoft.SkypeApp".into() }).unwrap();
        journal.ajouter(Entree::OneDriveRetire).unwrap();

        let bilan = annuler(&mut sys, &mut journal, &essai());

        assert_eq!(bilan.a_reinstaller, vec!["Microsoft.SkypeApp".to_string(), "OneDrive".to_string()]);
        assert_eq!(bilan.remis, 0);
        assert!(sys.paquets.is_empty());
        assert!(journal.est_vide());
    }

    #[test]
    fn un_reglage_qui_ne_peut_pas_etre_remis_reste_dans_le_journal() {
        let mut sys = FauxSysteme::vide();
        sys.en_panne.insert("B".into());
        let mut journal = Journal::en_memoire();
        journal.ajouter(registre("A", Some(ValeurBrute::dword(1)))).unwrap();
        journal.ajouter(registre("B", Some(ValeurBrute::dword(1)))).unwrap();

        let bilan = annuler(&mut sys, &mut journal, &essai());

        assert_eq!(bilan.remis, 1);
        assert_eq!(bilan.echecs.len(), 1);
        assert_eq!(journal.entrees(), &[registre("B", Some(ValeurBrute::dword(1)))]);
    }

    #[test]
    fn une_entree_inconnue_du_catalogue_n_est_jamais_appliquee() {
        let mut sys = FauxSysteme::vide();
        sys.services.insert("WinDefend".into(), DEMARRAGE_AUTO);
        sys.taches.insert(r"\Autre".into(), false);
        let avant = sys.clone();
        let mut journal = Journal::en_memoire();
        journal
            .ajouter(Entree::Registre {
                ruche: Ruche::Machine,
                cle: r"SOFTWARE\Microsoft\Windows\CurrentVersion\Run".into(),
                nom: "Intrus".into(),
                avant: Some(ValeurBrute { genre: 1, octets: vec![b'x', 0, 0, 0] }),
            })
            .unwrap();
        journal.ajouter(Entree::Service { nom: "WinDefend".into(), avant: DEMARRAGE_DESACTIVE }).unwrap();
        journal.ajouter(Entree::Tache { chemin: r"\Autre".into(), avant: true }).unwrap();

        let bilan = annuler(&mut sys, &mut journal, &essai());

        assert_eq!(sys, avant);
        assert_eq!(bilan.remis, 0);
        assert_eq!(bilan.echecs.len(), 3);
        assert!(journal.est_vide());
    }

    #[test]
    fn la_casse_du_registre_ne_fait_pas_rejeter_une_entree() {
        let entree = Entree::Registre {
            ruche: Ruche::Utilisateur,
            cle: CLE.to_uppercase(),
            nom: "a".into(),
            avant: None,
        };
        assert!(connue(&entree, &essai()));
    }

    #[test]
    fn la_meme_cle_dans_l_autre_ruche_est_inconnue() {
        let entree = Entree::Registre { ruche: Ruche::Machine, cle: CLE.into(), nom: "A".into(), avant: None };
        assert!(!connue(&entree, &essai()));
    }
}
