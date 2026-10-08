//! Fait le travail dans un fil à part, pour que la fenêtre reste fluide.
//!
//! La fenêtre envoie des commandes et reçoit des messages ; elle ne touche
//! jamais au système elle-même.

use std::sync::mpsc::{channel, Receiver, Sender};

use crate::analyse::{analyser, Analyse, Ligne};
use crate::catalogue::Element;
use crate::execution::{appliquer, Resultat};
use crate::journal::{annuler, Annulation, Journal};
use crate::systeme::{PointRestauration, Systeme};

pub enum Commande {
    Analyser,
    /// `avec_point` : créer d'abord un point de restauration.
    Appliquer { lignes: Vec<Ligne<'static>>, avec_point: bool },
    Annuler,
}

#[derive(Debug)]
pub enum Message {
    /// `annulable` : le journal contient des changements à défaire.
    Analyse { analyse: Analyse<'static>, annulable: bool },
    /// Ce qui est en cours : rang (à partir de 1), total et nom de l'élément.
    Progression { rang: usize, total: usize, nom: String },
    /// Rien n'a été changé : à l'utilisateur de dire s'il continue sans filet.
    PointImpossible { erreur: String, lignes: Vec<Ligne<'static>> },
    Applique { resultats: Vec<Resultat>, point: Option<PointRestauration> },
    Annule(Annulation),
}

/// Nom affiché pendant la création du point de restauration.
pub const POINT: &str = "Point de restauration";

pub struct Moteur {
    commandes: Sender<Commande>,
    pub messages: Receiver<Message>,
}

impl Moteur {
    /// `reveil` est appelé après chaque message, pour que la fenêtre se redessine.
    pub fn demarrer(
        sys: Box<dyn Systeme + Send>,
        journal: Journal,
        catalogue: &'static [Element],
        reveil: Box<dyn Fn() + Send>,
    ) -> Moteur {
        let (commandes, a_faire) = channel::<Commande>();
        let (vers_fenetre, messages) = channel::<Message>();
        std::thread::spawn(move || {
            let mut sys = sys;
            let mut journal = journal;
            let dire = |message: Message| {
                let _ = vers_fenetre.send(message);
                reveil();
            };
            // La boucle s'arrête d'elle-même quand la fenêtre se ferme.
            for commande in a_faire {
                match commande {
                    Commande::Analyser => {}
                    Commande::Appliquer { lignes, avec_point } => {
                        let total = lignes.len();
                        let mut point = None;
                        if avec_point {
                            dire(Message::Progression { rang: 0, total, nom: POINT.to_string() });
                            match sys.creer_point_restauration() {
                                Ok(cree) => point = Some(cree),
                                Err(erreur) => {
                                    dire(Message::PointImpossible { erreur, lignes });
                                    continue;
                                }
                            }
                        }
                        let resultats = appliquer(sys.as_mut(), &mut journal, &lignes, &mut |rang, nom| {
                            dire(Message::Progression { rang: rang + 1, total, nom: nom.to_string() });
                        });
                        dire(Message::Applique { resultats, point });
                    }
                    Commande::Annuler => {
                        dire(Message::Annule(annuler(sys.as_mut(), &mut journal, catalogue)));
                    }
                }
                // Toute commande se termine par un état des lieux à jour.
                let analyse = analyser(sys.as_ref(), catalogue);
                dire(Message::Analyse { analyse, annulable: !journal.est_vide() });
            }
        });
        Moteur { commandes, messages }
    }

    pub fn envoyer(&self, commande: Commande) {
        // Si le fil de travail s'est arrêté, il n'y a plus personne à qui parler.
        let _ = self.commandes.send(commande);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue::catalogue;
    use crate::faux::FauxSysteme;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    fn moteur(sys: FauxSysteme) -> Moteur {
        Moteur::demarrer(Box::new(sys), Journal::en_memoire(), catalogue(), Box::new(|| {}))
    }

    fn suivant(moteur: &Moteur) -> Message {
        moteur.messages.recv_timeout(Duration::from_secs(5)).expect("le moteur n'a pas répondu")
    }

    fn analyse(moteur: &Moteur) -> (Analyse<'static>, bool) {
        moteur.envoyer(Commande::Analyser);
        match suivant(moteur) {
            Message::Analyse { analyse, annulable } => (analyse, annulable),
            autre => panic!("message inattendu : {autre:?}"),
        }
    }

    /// Lit les messages jusqu'à celui qui n'est pas une progression.
    fn apres_progression(moteur: &Moteur) -> (usize, Message) {
        let mut etapes = 0;
        loop {
            match suivant(moteur) {
                Message::Progression { .. } => etapes += 1,
                autre => return (etapes, autre),
            }
        }
    }

    #[test]
    fn l_analyse_d_un_pc_neuf_propose_des_elements_et_rien_a_annuler() {
        let moteur = moteur(FauxSysteme::windows_10_typique());
        let (analyse, annulable) = analyse(&moteur);
        assert!(!analyse.lignes.is_empty());
        assert!(!annulable);
    }

    #[test]
    fn appliquer_cree_le_point_puis_rend_compte_et_refait_l_analyse() {
        let moteur = moteur(FauxSysteme::windows_10_typique());
        let (premiere, _) = analyse(&moteur);
        let choisies = premiere.lignes.len();

        moteur.envoyer(Commande::Appliquer { lignes: premiere.lignes, avec_point: true });

        let (etapes, message) = apres_progression(&moteur);
        // Une étape pour le point de restauration, puis une par élément.
        assert_eq!(etapes, choisies + 1);
        match message {
            Message::Applique { resultats, point } => {
                assert_eq!(resultats.len(), choisies);
                assert_eq!(point, Some(PointRestauration::Cree));
            }
            autre => panic!("message inattendu : {autre:?}"),
        }
        match suivant(&moteur) {
            Message::Analyse { analyse, annulable } => {
                assert!(analyse.lignes.is_empty());
                assert!(annulable);
            }
            autre => panic!("message inattendu : {autre:?}"),
        }
    }

    #[test]
    fn sans_point_de_restauration_possible_rien_n_est_change() {
        let mut sys = FauxSysteme::windows_10_typique();
        sys.point_restauration = Err("La restauration du système est désactivée".into());
        let moteur = moteur(sys);
        let (premiere, _) = analyse(&moteur);
        let choisies = premiere.lignes.len();

        moteur.envoyer(Commande::Appliquer { lignes: premiere.lignes, avec_point: true });

        match apres_progression(&moteur).1 {
            Message::PointImpossible { erreur, lignes } => {
                assert!(erreur.contains("désactivée"));
                assert_eq!(lignes.len(), choisies);
            }
            autre => panic!("message inattendu : {autre:?}"),
        }
        let (seconde, annulable) = analyse(&moteur);
        assert_eq!(seconde.lignes.len(), choisies);
        assert!(!annulable);
    }

    #[test]
    fn on_peut_appliquer_sans_point_de_restauration_si_on_le_demande() {
        let mut sys = FauxSysteme::windows_10_typique();
        sys.point_restauration = Err("désactivée".into());
        let moteur = moteur(sys);
        let (premiere, _) = analyse(&moteur);

        moteur.envoyer(Commande::Appliquer { lignes: premiere.lignes, avec_point: false });

        match apres_progression(&moteur).1 {
            Message::Applique { resultats, point } => {
                assert!(resultats.iter().all(|r| r.reussi()));
                assert_eq!(point, None);
            }
            autre => panic!("message inattendu : {autre:?}"),
        }
    }

    #[test]
    fn annuler_rend_compte_puis_les_reglages_sont_de_nouveau_proposes() {
        let moteur = moteur(FauxSysteme::windows_10_typique());
        let (premiere, _) = analyse(&moteur);
        moteur.envoyer(Commande::Appliquer { lignes: premiere.lignes, avec_point: false });
        apres_progression(&moteur);
        suivant(&moteur);

        moteur.envoyer(Commande::Annuler);

        match suivant(&moteur) {
            Message::Annule(bilan) => {
                assert!(bilan.remis > 0);
                assert!(bilan.echecs.is_empty());
                assert!(!bilan.a_reinstaller.is_empty());
            }
            autre => panic!("message inattendu : {autre:?}"),
        }
        match suivant(&moteur) {
            Message::Analyse { analyse, annulable } => {
                assert!(!analyse.lignes.is_empty());
                assert!(!annulable);
            }
            autre => panic!("message inattendu : {autre:?}"),
        }
    }

    #[test]
    fn la_fenetre_est_reveillee_a_chaque_message() {
        let reveils = Arc::new(AtomicUsize::new(0));
        let compteur = reveils.clone();
        let moteur = Moteur::demarrer(
            Box::new(FauxSysteme::windows_10_typique()),
            Journal::en_memoire(),
            catalogue(),
            Box::new(move || {
                compteur.fetch_add(1, Ordering::SeqCst);
            }),
        );
        moteur.envoyer(Commande::Analyser);
        suivant(&moteur);
        moteur.envoyer(Commande::Analyser);
        suivant(&moteur);
        // Le réveil suit l'envoi : on laisse au fil le temps de le faire.
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(reveils.load(Ordering::SeqCst), 2);
    }
}
