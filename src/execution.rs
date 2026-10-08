//! Fait les changements choisis, en notant chacun dans le journal avant de le faire.

use crate::analyse::{Ligne, Travail};
use crate::catalogue::est_protege;
use crate::journal::{Entree, Journal};
use crate::systeme::{Systeme, ValeurBrute, DEMARRAGE_DESACTIVE};

/// Ce qu'a donné un élément.
#[derive(Clone, Debug, PartialEq)]
pub struct Resultat {
    pub id: String,
    pub nom: String,
    /// Nombre d'opérations réussies.
    pub faits: usize,
    pub erreurs: Vec<String>,
}

impl Resultat {
    pub fn reussi(&self) -> bool {
        self.erreurs.is_empty()
    }
}

/// Applique les lignes choisies. `suivi` est appelé avant chaque ligne avec son rang et son nom.
pub fn appliquer(
    sys: &mut dyn Systeme,
    journal: &mut Journal,
    lignes: &[Ligne],
    suivi: &mut dyn FnMut(usize, &str),
) -> Vec<Resultat> {
    let mut resultats = Vec::new();
    for (rang, ligne) in lignes.iter().enumerate() {
        suivi(rang, ligne.element.nom);
        let mut resultat = Resultat {
            id: ligne.element.id.to_string(),
            nom: ligne.element.nom.to_string(),
            faits: 0,
            erreurs: Vec::new(),
        };
        for travail in &ligne.travaux {
            match executer(sys, journal, travail) {
                Ok(()) => resultat.faits += 1,
                Err(e) => resultat.erreurs.push(format!("{} : {e}", travail.decrire())),
            }
        }
        resultats.push(resultat);
    }
    resultats
}

/// Fait une seule opération.
pub fn executer(sys: &mut dyn Systeme, journal: &mut Journal, travail: &Travail) -> Result<(), String> {
    match travail {
        Travail::Paquet(nom) => {
            if est_protege(nom) {
                return Err("appli nécessaire à Windows, refusée".to_string());
            }
            sys.retirer_paquet(nom)?;
            journal.ajouter(Entree::PaquetRetire { nom: nom.clone() })
        }
        Travail::OneDrive => {
            sys.desinstaller_onedrive()?;
            journal.ajouter(Entree::OneDriveRetire)
        }
        Travail::Registre { ruche, cle, nom, valeur } => {
            let avant = sys.lire_valeur(*ruche, cle, nom)?;
            journal.ajouter(Entree::Registre { ruche: *ruche, cle: cle.clone(), nom: nom.clone(), avant })?;
            sys.ecrire_valeur(*ruche, cle, nom, &ValeurBrute::dword(*valeur))
                .map_err(|e| oublier(journal, e))
        }
        Travail::Service(nom) => {
            let avant = sys
                .demarrage_service(nom)?
                .ok_or_else(|| "service introuvable".to_string())?;
            journal.ajouter(Entree::Service { nom: nom.clone(), avant })?;
            sys.regler_service(nom, DEMARRAGE_DESACTIVE)
                .map_err(|e| oublier(journal, e))?;
            // Désactivé, il ne repartira plus : peu importe s'il refuse de s'arrêter tout de suite.
            let _ = sys.arreter_service(nom);
            Ok(())
        }
        Travail::Tache(chemin) => {
            let avant = sys
                .tache_active(chemin)?
                .ok_or_else(|| "tâche introuvable".to_string())?;
            journal.ajouter(Entree::Tache { chemin: chemin.clone(), avant })?;
            sys.activer_tache(chemin, false).map_err(|e| oublier(journal, e))
        }
    }
}

/// Le changement annoncé au journal n'a pas eu lieu : on retire sa trace.
fn oublier(journal: &mut Journal, erreur: String) -> String {
    let _ = journal.retirer_derniere();
    erreur
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyse::analyser;
    use crate::analyse::tests::{element, CLE};
    use crate::catalogue::{catalogue, Element};
    use crate::faux::FauxSysteme;
    use crate::journal::tests::dossier_temporaire;
    use crate::journal::annuler;
    use crate::systeme::*;

    fn reglage(nom: &str, valeur: u32) -> Travail {
        Travail::Registre { ruche: Ruche::Utilisateur, cle: CLE.into(), nom: nom.into(), valeur }
    }

    fn ligne<'a>(element: &'a Element, travaux: Vec<Travail>) -> Ligne<'a> {
        Ligne { element, travaux }
    }

    fn sans_suivi() -> impl FnMut(usize, &str) {
        |_, _| {}
    }

    #[test]
    fn un_reglage_est_ecrit_et_son_ancienne_valeur_notee() {
        let mut sys = FauxSysteme::vide();
        sys.poser(Ruche::Utilisateur, CLE, "A", 1);
        let mut journal = Journal::en_memoire();

        executer(&mut sys, &mut journal, &reglage("A", 0)).unwrap();

        assert_eq!(sys.valeur(Ruche::Utilisateur, CLE, "A"), Some(0));
        assert_eq!(
            journal.entrees(),
            &[Entree::Registre {
                ruche: Ruche::Utilisateur,
                cle: CLE.into(),
                nom: "A".into(),
                avant: Some(ValeurBrute::dword(1)),
            }]
        );
    }

    #[test]
    fn un_reglage_qui_n_existait_pas_est_note_comme_absent() {
        let mut sys = FauxSysteme::vide();
        let mut journal = Journal::en_memoire();

        executer(&mut sys, &mut journal, &reglage("A", 0)).unwrap();

        assert!(matches!(&journal.entrees()[0], Entree::Registre { avant: None, .. }));
    }

    #[test]
    fn un_reglage_qui_echoue_ne_laisse_pas_de_trace_dans_le_journal() {
        let mut sys = FauxSysteme::vide();
        sys.en_panne.insert("A".into());
        let mut journal = Journal::en_memoire();

        assert!(executer(&mut sys, &mut journal, &reglage("A", 0)).is_err());

        assert!(journal.est_vide());
    }

    #[test]
    fn sans_journal_enregistrable_rien_n_est_change() {
        let dossier = dossier_temporaire();
        std::fs::write(dossier.join("fichier"), "x").unwrap();
        let mut journal = Journal::ouvrir(dossier.join("fichier").join("journal.json")).unwrap();
        let mut sys = FauxSysteme::vide();
        sys.poser(Ruche::Utilisateur, CLE, "A", 1);
        sys.services.insert("DiagTrack".into(), DEMARRAGE_AUTO);
        sys.taches.insert(r"\T".into(), true);
        let avant = sys.clone();

        assert!(executer(&mut sys, &mut journal, &reglage("A", 0)).is_err());
        assert!(executer(&mut sys, &mut journal, &Travail::Service("DiagTrack".into())).is_err());
        assert!(executer(&mut sys, &mut journal, &Travail::Tache(r"\T".into())).is_err());

        assert_eq!(sys, avant);
    }

    #[test]
    fn un_service_est_desactive_et_arrete() {
        let mut sys = FauxSysteme::vide();
        sys.services.insert("DiagTrack".into(), DEMARRAGE_AUTO);
        sys.en_marche.insert("DiagTrack".into());
        let mut journal = Journal::en_memoire();

        executer(&mut sys, &mut journal, &Travail::Service("DiagTrack".into())).unwrap();

        assert_eq!(sys.services["DiagTrack"], DEMARRAGE_DESACTIVE);
        assert!(!sys.en_marche.contains("DiagTrack"));
        assert_eq!(journal.entrees(), &[Entree::Service { nom: "DiagTrack".into(), avant: DEMARRAGE_AUTO }]);
    }

    #[test]
    fn une_tache_est_desactivee() {
        let mut sys = FauxSysteme::vide();
        sys.taches.insert(r"\T".into(), true);
        let mut journal = Journal::en_memoire();

        executer(&mut sys, &mut journal, &Travail::Tache(r"\T".into())).unwrap();

        assert_eq!(sys.taches[r"\T"], false);
        assert_eq!(journal.entrees(), &[Entree::Tache { chemin: r"\T".into(), avant: true }]);
    }

    #[test]
    fn une_appli_est_retiree_et_notee_pour_memoire() {
        let mut sys = FauxSysteme::vide();
        sys.paquets = vec!["Microsoft.SkypeApp".into(), "Autre.Appli".into()];
        let mut journal = Journal::en_memoire();

        executer(&mut sys, &mut journal, &Travail::Paquet("Microsoft.SkypeApp".into())).unwrap();

        assert_eq!(sys.paquets, vec!["Autre.Appli".to_string()]);
        assert_eq!(journal.entrees(), &[Entree::PaquetRetire { nom: "Microsoft.SkypeApp".into() }]);
    }

    #[test]
    fn une_appli_qui_n_a_pas_pu_etre_retiree_n_est_pas_notee() {
        let mut sys = FauxSysteme::vide();
        sys.paquets = vec!["Microsoft.SkypeApp".into()];
        sys.en_panne.insert("Microsoft.SkypeApp".into());
        let mut journal = Journal::en_memoire();

        assert!(executer(&mut sys, &mut journal, &Travail::Paquet("Microsoft.SkypeApp".into())).is_err());

        assert!(journal.est_vide());
    }

    #[test]
    fn une_appli_protegee_est_refusee_meme_si_on_la_demande() {
        let mut sys = FauxSysteme::vide();
        sys.paquets = vec!["Microsoft.WindowsStore".into()];
        let mut journal = Journal::en_memoire();

        let reponse = executer(&mut sys, &mut journal, &Travail::Paquet("Microsoft.WindowsStore".into()));

        assert!(reponse.is_err());
        assert_eq!(sys.paquets, vec!["Microsoft.WindowsStore".to_string()]);
    }

    #[test]
    fn onedrive_est_desinstalle_et_note() {
        let mut sys = FauxSysteme::vide();
        sys.onedrive = true;
        let mut journal = Journal::en_memoire();

        executer(&mut sys, &mut journal, &Travail::OneDrive).unwrap();

        assert!(!sys.onedrive);
        assert_eq!(journal.entrees(), &[Entree::OneDriveRetire]);
    }

    #[test]
    fn une_erreur_n_empeche_pas_les_operations_suivantes() {
        let mut sys = FauxSysteme::vide();
        sys.en_panne.insert("A".into());
        let mut journal = Journal::en_memoire();
        let e = element(&[]);
        let lignes = [ligne(&e, vec![reglage("A", 0), reglage("B", 0)])];

        let resultats = appliquer(&mut sys, &mut journal, &lignes, &mut sans_suivi());

        assert_eq!(resultats.len(), 1);
        assert!(!resultats[0].reussi());
        assert_eq!(resultats[0].faits, 1);
        assert_eq!(resultats[0].erreurs.len(), 1);
        assert_eq!(sys.valeur(Ruche::Utilisateur, CLE, "B"), Some(0));
    }

    #[test]
    fn le_suivi_annonce_chaque_ligne_avant_de_la_traiter() {
        let mut sys = FauxSysteme::vide();
        let mut journal = Journal::en_memoire();
        let e = element(&[]);
        let lignes = [ligne(&e, vec![reglage("A", 0)]), ligne(&e, vec![reglage("B", 0)])];
        let mut vus = Vec::new();

        appliquer(&mut sys, &mut journal, &lignes, &mut |rang, nom| vus.push((rang, nom.to_string())));

        assert_eq!(vus, vec![(0, "Essai".to_string()), (1, "Essai".to_string())]);
    }

    #[test]
    fn apres_application_du_catalogue_entier_il_ne_reste_rien_a_faire() {
        let mut sys = FauxSysteme::windows_10_typique();
        let mut journal = Journal::en_memoire();
        let analyse = analyser(&sys, catalogue());
        assert!(analyse.lignes.len() > 30, "le PC de démonstration devrait proposer beaucoup d'éléments");

        let resultats = appliquer(&mut sys, &mut journal, &analyse.lignes, &mut sans_suivi());

        assert!(resultats.iter().all(|r| r.reussi()), "{resultats:?}");
        assert!(analyser(&sys, catalogue()).lignes.is_empty());
    }

    #[test]
    fn le_catalogue_entier_ne_retire_aucune_appli_protegee() {
        let mut sys = FauxSysteme::windows_10_typique();
        let mut journal = Journal::en_memoire();
        let analyse = analyser(&sys, catalogue());

        appliquer(&mut sys, &mut journal, &analyse.lignes, &mut sans_suivi());

        for nom in [
            "Microsoft.WindowsStore",
            "Microsoft.StorePurchaseApp",
            "Microsoft.DesktopAppInstaller",
            "Microsoft.MicrosoftEdge",
            "Microsoft.Windows.ShellExperienceHost",
            "Microsoft.VCLibs.140.00",
            "Microsoft.NET.Native.Framework.2.2",
            "Microsoft.UI.Xaml.2.8",
            "Microsoft.HEIFImageExtension",
            "Microsoft.XboxGameCallableUI",
        ] {
            assert!(sys.paquets.iter().any(|p| p == nom), "{nom} a été retirée");
        }
    }

    #[test]
    fn annuler_apres_le_catalogue_entier_remet_reglages_services_et_taches() {
        let depart = FauxSysteme::windows_10_typique();
        let mut sys = depart.clone();
        let mut journal = Journal::en_memoire();
        let analyse = analyser(&sys, catalogue());
        appliquer(&mut sys, &mut journal, &analyse.lignes, &mut sans_suivi());

        let bilan = annuler(&mut sys, &mut journal, catalogue());

        assert!(bilan.echecs.is_empty(), "{:?}", bilan.echecs);
        assert_eq!(sys.registre, depart.registre);
        assert_eq!(sys.services, depart.services);
        assert_eq!(sys.en_marche, depart.en_marche);
        assert_eq!(sys.taches, depart.taches);
        assert!(bilan.a_reinstaller.contains(&"Microsoft.SkypeApp".to_string()));
    }
}
