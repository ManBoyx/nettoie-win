//! Pilote la fenêtre comme le ferait quelqu'un : clics sur les boutons, lecture de ce qui s'affiche.
#![cfg(feature = "fenetre")]

use std::time::Duration;

use egui_kittest::kittest::{NodeT, Queryable};
use egui_kittest::Harness;
use nettoie_win::faux::FauxSysteme;
use nettoie_win::interface::Fenetre;
use nettoie_win::journal::Journal;

type Banc = Harness<'static, Fenetre>;

fn fenetre(sys: FauxSysteme) -> Banc {
    let fenetre = Fenetre::nouvelle(Box::new(sys), Journal::en_memoire(), Box::new(|| {}), true);
    let mut banc = Harness::builder()
        .with_size(egui::vec2(900.0, 680.0))
        .build_ui_state(|ui, fenetre: &mut Fenetre| fenetre.afficher(ui), fenetre);
    repos(&mut banc);
    banc
}

/// Laisse le fil de travail finir, puis redessine.
fn repos(banc: &mut Banc) {
    for _ in 0..500 {
        banc.step();
        if !banc.state().occupee() {
            banc.step();
            banc.step();
            return;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("le travail ne se termine pas");
}

fn clic(banc: &mut Banc, libelle: &str) {
    banc.get_by_label(libelle).click();
    banc.step();
    repos(banc);
}

fn coche(banc: &Banc, libelle: &str) -> bool {
    banc.get_by_label(libelle).accesskit_node().toggled() == Some(egui::accesskit::Toggled::True)
}

fn visible(banc: &Banc, libelle: &str) -> bool {
    banc.query_by_label(libelle).is_some()
}

fn contient(banc: &Banc, morceau: &str) -> bool {
    banc.query_all_by_label_contains(morceau).next().is_some()
}

fn desactive(banc: &Banc, libelle: &str) -> bool {
    banc.get_by_label(libelle).accesskit_node().is_disabled()
}

#[test]
fn a_l_ouverture_les_elements_sans_risque_sont_coches_et_les_autres_non() {
    let banc = fenetre(FauxSysteme::windows_10_typique());
    assert!(coche(&banc, "Jeux promotionnels"));
    assert!(coche(&banc, "Skype"));
    assert!(!coche(&banc, "Xbox et Game Bar"));
    assert!(!coche(&banc, "OneDrive"));
    assert!(!coche(&banc, "Calculatrice"));
}

#[test]
fn une_appli_absente_du_pc_n_est_pas_affichee() {
    let mut sys = FauxSysteme::windows_10_typique();
    sys.paquets.retain(|p| p != "Microsoft.SkypeApp");
    let banc = fenetre(sys);
    assert!(!visible(&banc, "Skype"));
    assert!(visible(&banc, "Jeux promotionnels"));
}

#[test]
fn chaque_onglet_montre_sa_propre_liste() {
    let mut banc = fenetre(FauxSysteme::windows_10_typique());
    assert!(!visible(&banc, "Services de télémétrie"));

    banc.get_by_label_contains("Télémétrie (").click();
    repos(&mut banc);

    assert!(visible(&banc, "Services de télémétrie"));
    assert!(!visible(&banc, "Skype"));

    banc.get_by_label_contains("Pubs et suggestions (").click();
    repos(&mut banc);

    assert!(visible(&banc, "Suggestions dans le menu Démarrer"));
    assert!(!visible(&banc, "Services de télémétrie"));
}

#[test]
fn le_mode_demonstration_est_annonce() {
    let banc = fenetre(FauxSysteme::windows_10_typique());
    assert!(contient(&banc, "Démonstration"));
}

#[test]
fn sans_rien_de_coche_on_ne_peut_pas_appliquer() {
    let mut banc = fenetre(FauxSysteme::windows_10_typique());
    assert!(!desactive(&banc, "Appliquer"));

    clic(&mut banc, "Tout décocher");

    assert!(!coche(&banc, "Skype"));
    assert!(desactive(&banc, "Appliquer"));
    assert!(contient(&banc, "Aucun élément coché"));
}

#[test]
fn recocher_le_sans_risque_ne_coche_pas_ce_qui_demande_attention() {
    let mut banc = fenetre(FauxSysteme::windows_10_typique());
    clic(&mut banc, "Tout décocher");

    clic(&mut banc, "Cocher ce qui est sans risque");

    assert!(coche(&banc, "Skype"));
    assert!(!coche(&banc, "Xbox et Game Bar"));
}

#[test]
fn l_apercu_detaille_ce_qui_sera_fait_sans_rien_changer() {
    let mut banc = fenetre(FauxSysteme::windows_10_typique());

    clic(&mut banc, "Aperçu");

    assert!(visible(&banc, "Retirer l'appli Microsoft.SkypeApp"));
    assert!(visible(&banc, "Désactiver le service DiagTrack"));
    assert!(!contient(&banc, "Microsoft.XboxApp"));

    clic(&mut banc, "Fermer");

    assert!(visible(&banc, "Skype"), "l'aperçu ne doit rien retirer");
}

#[test]
fn appliquer_demande_confirmation_en_prevenant_pour_les_applis() {
    let mut banc = fenetre(FauxSysteme::windows_10_typique());

    clic(&mut banc, "Appliquer");

    assert!(contient(&banc, "Microsoft Store"));
    assert!(contient(&banc, "point de restauration"));

    clic(&mut banc, "Retour");

    assert!(visible(&banc, "Skype"), "revenir en arrière ne doit rien retirer");
}

#[test]
fn apres_confirmation_les_elements_coches_disparaissent_et_les_autres_restent() {
    let mut banc = fenetre(FauxSysteme::windows_10_typique());
    assert!(desactive(&banc, "Annuler les changements"));
    clic(&mut banc, "Appliquer");

    clic(&mut banc, "Confirmer");

    assert!(contient(&banc, "Point de restauration créé"));
    assert!(contient(&banc, "Redémarrez"));
    clic(&mut banc, "Fermer");
    assert!(!visible(&banc, "Skype"));
    assert!(!visible(&banc, "Jeux promotionnels"));
    assert!(visible(&banc, "Xbox et Game Bar"));
    assert!(!desactive(&banc, "Annuler les changements"));
}

#[test]
fn une_erreur_est_montree_dans_le_bilan() {
    let mut sys = FauxSysteme::windows_10_typique();
    sys.en_panne.insert("Microsoft.SkypeApp".into());
    let mut banc = fenetre(sys);
    clic(&mut banc, "Appliquer");

    clic(&mut banc, "Confirmer");

    assert!(contient(&banc, "accès refusé à Microsoft.SkypeApp"));
    clic(&mut banc, "Fermer");
    assert!(visible(&banc, "Skype"), "ce qui a échoué reste proposé");
}

#[test]
fn sans_point_de_restauration_on_peut_s_arreter_sans_rien_changer() {
    let mut sys = FauxSysteme::windows_10_typique();
    sys.point_restauration = Err("La restauration du système est désactivée".into());
    let mut banc = fenetre(sys);
    clic(&mut banc, "Appliquer");

    clic(&mut banc, "Confirmer");

    assert!(contient(&banc, "La restauration du système est désactivée"));
    clic(&mut banc, "Arrêter");
    assert!(visible(&banc, "Skype"));
    assert!(desactive(&banc, "Annuler les changements"));
}

#[test]
fn sans_point_de_restauration_on_peut_continuer_quand_meme() {
    let mut sys = FauxSysteme::windows_10_typique();
    sys.point_restauration = Err("désactivée".into());
    let mut banc = fenetre(sys);
    clic(&mut banc, "Appliquer");
    clic(&mut banc, "Confirmer");

    clic(&mut banc, "Continuer sans point de restauration");

    assert!(contient(&banc, "Aucun point de restauration"));
    clic(&mut banc, "Fermer");
    assert!(!visible(&banc, "Skype"));
}

#[test]
fn annuler_les_changements_remet_les_reglages_et_liste_les_applis_a_reinstaller() {
    let mut banc = fenetre(FauxSysteme::windows_10_typique());
    clic(&mut banc, "Appliquer");
    clic(&mut banc, "Confirmer");
    clic(&mut banc, "Fermer");
    banc.get_by_label_contains("Télémétrie (").click();
    repos(&mut banc);
    assert!(!visible(&banc, "Services de télémétrie"));

    clic(&mut banc, "Annuler les changements");
    clic(&mut banc, "Remettre comme avant");

    assert!(contient(&banc, "Microsoft.SkypeApp"));
    assert!(contient(&banc, "Microsoft Store"));
    clic(&mut banc, "Fermer");
    assert!(visible(&banc, "Services de télémétrie"));
    assert!(desactive(&banc, "Annuler les changements"));
}

/// Fabrique les images de contrôle : `cargo test --test fenetre captures -- --ignored`
#[test]
#[ignore = "produit des images, à lancer à la demande"]
fn captures() {
    let dossier = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("captures");
    std::fs::create_dir_all(&dossier).unwrap();
    for (theme, suffixe) in [(egui::Theme::Light, "clair"), (egui::Theme::Dark, "sombre")] {
        let fenetre = Fenetre::nouvelle(
            Box::new(FauxSysteme::windows_10_typique()),
            Journal::en_memoire(),
            Box::new(|| {}),
            true,
        );
        let mut banc = Harness::builder()
            .with_size(egui::vec2(900.0, 680.0))
            .with_theme(theme)
            .wgpu()
            .build_ui_state(|ui, fenetre: &mut Fenetre| fenetre.afficher(ui), fenetre);
        repos(&mut banc);
        let photo = |banc: &mut Banc, nom: &str| {
            banc.render().unwrap().save(dossier.join(format!("{nom}-{suffixe}.png"))).unwrap();
        };
        photo(&mut banc, "1-applis");
        // Une image très haute, pour voir toute la liste d'un coup.
        banc.set_size(egui::vec2(900.0, 2300.0));
        repos(&mut banc);
        photo(&mut banc, "0-applis-liste-entiere");
        banc.set_size(egui::vec2(900.0, 680.0));
        repos(&mut banc);
        banc.get_by_label_contains("Télémétrie (").click();
        repos(&mut banc);
        photo(&mut banc, "2-telemetrie");
        clic(&mut banc, "Aperçu");
        photo(&mut banc, "3-apercu");
        clic(&mut banc, "Fermer");
        clic(&mut banc, "Appliquer");
        photo(&mut banc, "4-confirmation");
        clic(&mut banc, "Confirmer");
        photo(&mut banc, "5-bilan");
    }
}
