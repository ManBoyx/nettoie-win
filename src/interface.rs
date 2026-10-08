//! La fenêtre.

use eframe::egui::{self, Align, Id, Layout, RichText};

use crate::analyse::Ligne;
use crate::catalogue::{catalogue, Categorie, Risque};
use crate::execution::Resultat;
use crate::journal::{Annulation, Journal};
use crate::moteur::{Commande, Message, Moteur};
use crate::systeme::{PointRestauration, Systeme};

const ONGLETS: [(Categorie, &str); 3] = [
    (Categorie::Applis, "Applis préinstallées"),
    (Categorie::Pubs, "Pubs et suggestions"),
    (Categorie::Telemetrie, "Télémétrie"),
];

/// Un élément proposé et sa case.
struct Vue {
    ligne: Ligne<'static>,
    cochee: bool,
}

enum Occupation {
    Libre,
    Analyse,
    /// `rang` vaut 0 pendant la création du point de restauration.
    Application { rang: usize, total: usize, nom: String },
    Annulation,
}

enum Dialogue {
    Aucun,
    Apercu,
    Confirmation,
    PointImpossible { erreur: String, lignes: Vec<Ligne<'static>> },
    Bilan { resultats: Vec<Resultat>, point: Option<PointRestauration> },
    ConfirmationAnnulation,
    BilanAnnulation(Annulation),
}

pub struct Fenetre {
    moteur: Moteur,
    demonstration: bool,
    habillee: bool,
    onglet: Categorie,
    vues: Vec<Vue>,
    masques: usize,
    avertissements: Vec<String>,
    annulable: bool,
    occupation: Occupation,
    dialogue: Dialogue,
}

impl Fenetre {
    /// `reveil` est appelé quand le fil de travail a du nouveau à montrer.
    pub fn nouvelle(
        sys: Box<dyn Systeme + Send>,
        journal: Journal,
        reveil: Box<dyn Fn() + Send>,
        demonstration: bool,
    ) -> Self {
        let moteur = Moteur::demarrer(sys, journal, catalogue(), reveil);
        moteur.envoyer(Commande::Analyser);
        Fenetre {
            moteur,
            demonstration,
            habillee: false,
            onglet: Categorie::Applis,
            vues: Vec::new(),
            masques: 0,
            avertissements: Vec::new(),
            annulable: false,
            occupation: Occupation::Analyse,
            dialogue: Dialogue::Aucun,
        }
    }

    /// Ajoute un avertissement affiché au-dessus de la liste.
    pub fn avertir(&mut self, message: String) {
        self.avertissements.push(message);
    }

    /// Vrai tant qu'une analyse, une application ou une annulation est en cours.
    pub fn occupee(&self) -> bool {
        !matches!(self.occupation, Occupation::Libre)
    }

    pub fn afficher(&mut self, ui: &mut egui::Ui) {
        if !self.habillee {
            habiller(ui.ctx());
            self.habillee = true;
        }
        self.recevoir();

        egui::Panel::top("entete").show(ui, |ui| self.entete(ui));
        egui::Panel::bottom("actions").show(ui, |ui| self.actions(ui));
        egui::CentralPanel::default().show(ui, |ui| {
            if self.occupee() {
                self.attente(ui);
            } else {
                self.liste(ui);
            }
        });
        let ctx = ui.ctx().clone();
        self.dialogues(&ctx);
    }

    fn recevoir(&mut self) {
        while let Ok(message) = self.moteur.messages.try_recv() {
            match message {
                Message::Analyse { analyse, annulable } => {
                    // Une case déjà touchée garde son état d'une analyse à l'autre.
                    let anciennes: Vec<(&'static str, bool)> =
                        self.vues.iter().map(|v| (v.ligne.element.id, v.cochee)).collect();
                    self.vues = analyse
                        .lignes
                        .into_iter()
                        .map(|ligne| {
                            let cochee = anciennes
                                .iter()
                                .find(|(id, _)| *id == ligne.element.id)
                                .map(|(_, cochee)| *cochee)
                                .unwrap_or_else(|| ligne.element.coche_par_defaut());
                            Vue { ligne, cochee }
                        })
                        .collect();
                    self.masques = analyse.masques;
                    self.avertissements.extend(analyse.avertissements);
                    self.avertissements.dedup();
                    self.annulable = annulable;
                    self.occupation = Occupation::Libre;
                }
                Message::Progression { rang, total, nom } => {
                    self.occupation = Occupation::Application { rang, total, nom };
                }
                Message::PointImpossible { erreur, lignes } => {
                    self.occupation = Occupation::Libre;
                    self.dialogue = Dialogue::PointImpossible { erreur, lignes };
                }
                Message::Applique { resultats, point } => {
                    self.dialogue = Dialogue::Bilan { resultats, point };
                }
                Message::Annule(bilan) => {
                    self.dialogue = Dialogue::BilanAnnulation(bilan);
                }
            }
        }
    }

    fn choisies(&self) -> Vec<Ligne<'static>> {
        self.vues.iter().filter(|v| v.cochee).map(|v| v.ligne.clone()).collect()
    }

    fn appliquer(&mut self, lignes: Vec<Ligne<'static>>, avec_point: bool) {
        self.occupation = Occupation::Application { rang: 0, total: lignes.len(), nom: String::new() };
        self.moteur.envoyer(Commande::Appliquer { lignes, avec_point });
    }

    fn entete(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Nettoie-Win").size(22.0).strong());
            ui.add_space(6.0);
            ui.label(RichText::new("Retire de Windows 10 ce qui ne sert à rien").weak());
        });
        if self.demonstration {
            ui.add_space(4.0);
            ui.colored_label(
                ui.visuals().warn_fg_color,
                "Démonstration : ce programme ne tourne pas sous Windows, rien n'est modifié.",
            );
        }
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            for (categorie, nom) in ONGLETS {
                let nombre = self.vues.iter().filter(|v| v.ligne.element.categorie == categorie).count();
                if ui.selectable_label(self.onglet == categorie, format!("{nom} ({nombre})")).clicked() {
                    self.onglet = categorie;
                }
            }
        });
        ui.add_space(4.0);
    }

    fn liste(&mut self, ui: &mut egui::Ui) {
        let alerte = ui.visuals().warn_fg_color;
        for avertissement in &self.avertissements {
            ui.colored_label(alerte, avertissement);
        }
        let onglet = self.onglet;
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let mut vide = true;
            for vue in self.vues.iter_mut().filter(|v| v.ligne.element.categorie == onglet) {
                vide = false;
                let element = vue.ligne.element;
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.checkbox(&mut vue.cochee, RichText::new(element.nom).strong());
                    if element.risque == Risque::Attention {
                        ui.label(RichText::new("à garder si vous vous en servez").small().color(alerte));
                    }
                });
                // Aligné sous le nom, après la case.
                egui::Frame::new().inner_margin(egui::Margin { left: 18, ..Default::default() }).show(ui, |ui| {
                    ui.label(RichText::new(element.explication).weak());
                });
                ui.add_space(6.0);
                ui.separator();
            }
            if vide {
                ui.add_space(24.0);
                ui.label(RichText::new("Rien à faire ici : tout est déjà réglé ou absent de ce PC.").weak());
            }
        });
    }

    fn attente(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.35);
            ui.spinner();
            ui.add_space(10.0);
            match &self.occupation {
                Occupation::Libre => {}
                Occupation::Analyse => {
                    ui.label("Analyse du PC…");
                }
                Occupation::Annulation => {
                    ui.label("Remise en place des réglages…");
                }
                Occupation::Application { rang, total, nom } => {
                    if *rang == 0 {
                        ui.label("Création du point de restauration…");
                        ui.label(RichText::new("Windows peut mettre une minute.").weak());
                    } else {
                        ui.label(format!("{nom} ({rang} sur {total})"));
                    }
                    ui.add_space(8.0);
                    let avancement = *rang as f32 / (*total as f32 + 1.0);
                    ui.add(egui::ProgressBar::new(avancement).desired_width(320.0));
                }
            }
        });
    }

    fn actions(&mut self, ui: &mut egui::Ui) {
        let libre = !self.occupee();
        let nombre = self.vues.iter().filter(|v| v.cochee).count();
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.add_enabled_ui(libre, |ui| {
                if ui.button("Cocher ce qui est sans risque").clicked() {
                    for vue in &mut self.vues {
                        vue.cochee = vue.ligne.element.coche_par_defaut();
                    }
                }
                if ui.button("Tout décocher").clicked() {
                    for vue in &mut self.vues {
                        vue.cochee = false;
                    }
                }
            });
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let principal = egui::Button::new(RichText::new("Appliquer").strong())
                    .fill(ui.visuals().selection.bg_fill);
                if ui.add_enabled(libre && nombre > 0, principal).clicked() {
                    self.dialogue = Dialogue::Confirmation;
                }
                if ui.add_enabled(libre && nombre > 0, egui::Button::new("Aperçu")).clicked() {
                    self.dialogue = Dialogue::Apercu;
                }
                if ui.add_enabled(libre && self.annulable, egui::Button::new("Annuler les changements")).clicked() {
                    self.dialogue = Dialogue::ConfirmationAnnulation;
                }
            });
        });
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            ui.label(match nombre {
                0 => "Aucun élément coché".to_string(),
                1 => "1 élément coché".to_string(),
                n => format!("{n} éléments cochés"),
            });
            if self.masques > 0 {
                ui.label(
                    RichText::new(format!(
                        "· {} déjà réglés ou absents de ce PC, non affichés",
                        self.masques
                    ))
                    .weak(),
                );
            }
        });
        ui.add_space(8.0);
    }

    fn dialogues(&mut self, ctx: &egui::Context) {
        let dialogue = std::mem::replace(&mut self.dialogue, Dialogue::Aucun);
        self.dialogue = match dialogue {
            Dialogue::Aucun => Dialogue::Aucun,

            Dialogue::Apercu => {
                let mut fermer = false;
                let echappe = modale(ctx, "Ce qui sera fait", |ui| {
                    egui::ScrollArea::vertical().max_height(380.0).show(ui, |ui| {
                        for vue in self.vues.iter().filter(|v| v.cochee) {
                            ui.label(RichText::new(format!("{} :", vue.ligne.element.nom)).strong());
                            for travail in &vue.ligne.travaux {
                                ui.label(RichText::new(travail.decrire()).weak());
                            }
                            ui.add_space(6.0);
                        }
                    });
                    ui.add_space(8.0);
                    fermer = ui.button("Fermer").clicked();
                });
                if fermer || echappe {
                    Dialogue::Aucun
                } else {
                    Dialogue::Apercu
                }
            }

            Dialogue::Confirmation => {
                let lignes = self.choisies();
                let applis: Vec<&str> = lignes
                    .iter()
                    .filter(|l| l.element.categorie == Categorie::Applis)
                    .map(|l| l.element.nom)
                    .collect();
                let (mut confirmer, mut retour) = (false, false);
                let echappe = modale(ctx, "Appliquer les changements ?", |ui| {
                    ui.label(match lignes.len() {
                        1 => "1 élément va être modifié.".to_string(),
                        n => format!("{n} éléments vont être modifiés."),
                    });
                    ui.add_space(6.0);
                    ui.label("Un point de restauration Windows est créé avant de commencer.");
                    if !applis.is_empty() {
                        ui.add_space(6.0);
                        ui.label(format!("Applis retirées : {}.", applis.join(", ")));
                        ui.colored_label(
                            ui.visuals().warn_fg_color,
                            "« Annuler les changements » ne remet pas les applis : elles se réinstallent depuis le Microsoft Store.",
                        );
                    }
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        confirmer = ui.button("Confirmer").clicked();
                        retour = ui.button("Retour").clicked();
                    });
                });
                if confirmer {
                    self.appliquer(lignes, true);
                    Dialogue::Aucun
                } else if retour || echappe {
                    Dialogue::Aucun
                } else {
                    Dialogue::Confirmation
                }
            }

            Dialogue::PointImpossible { erreur, lignes } => {
                let (mut continuer, mut arreter) = (false, false);
                modale(ctx, "Pas de point de restauration", |ui| {
                    ui.label("Windows n'a pas pu créer de point de restauration :");
                    ui.colored_label(ui.visuals().error_fg_color, &erreur);
                    ui.add_space(6.0);
                    ui.label(
                        "Rien n'a encore été modifié. Sans ce filet, « Annuler les changements » \
                         reste possible pour les réglages, mais pas pour les applis.",
                    );
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        arreter = ui.button("Arrêter").clicked();
                        continuer = ui.button("Continuer sans point de restauration").clicked();
                    });
                });
                if continuer {
                    self.appliquer(lignes, false);
                    Dialogue::Aucun
                } else if arreter {
                    Dialogue::Aucun
                } else {
                    Dialogue::PointImpossible { erreur, lignes }
                }
            }

            Dialogue::Bilan { resultats, point } => {
                let rates: Vec<&Resultat> = resultats.iter().filter(|r| !r.reussi()).collect();
                let titre = if rates.is_empty() { "Terminé" } else { "Terminé, avec des erreurs" };
                let mut fermer = false;
                modale(ctx, titre, |ui| {
                    let reussis = resultats.len() - rates.len();
                    ui.label(match reussis {
                        1 => "1 élément traité.".to_string(),
                        n => format!("{n} éléments traités."),
                    });
                    ui.label(match point {
                        Some(PointRestauration::Cree) => "Point de restauration créé.",
                        Some(PointRestauration::DejaRecent) => {
                            "Windows avait déjà un point de restauration de moins de 24 heures : il sert de filet."
                        }
                        None => "Aucun point de restauration n'a été créé.",
                    });
                    if !rates.is_empty() {
                        ui.add_space(8.0);
                        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                            for resultat in &rates {
                                ui.label(RichText::new(format!("{} :", resultat.nom)).strong());
                                for erreur in &resultat.erreurs {
                                    ui.colored_label(ui.visuals().error_fg_color, erreur);
                                }
                                ui.add_space(4.0);
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.label("Redémarrez le PC pour que tout soit pris en compte.");
                    ui.add_space(12.0);
                    fermer = ui.button("Fermer").clicked();
                });
                if fermer {
                    Dialogue::Aucun
                } else {
                    Dialogue::Bilan { resultats, point }
                }
            }

            Dialogue::ConfirmationAnnulation => {
                let (mut confirmer, mut retour) = (false, false);
                let echappe = modale(ctx, "Remettre les réglages comme avant ?", |ui| {
                    ui.label("Les réglages, services et tâches modifiés par Nettoie-Win retrouvent leur état d'origine.");
                    ui.add_space(6.0);
                    ui.label("Les applis retirées ne reviennent pas : elles se réinstallent depuis le Microsoft Store.");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        confirmer = ui.button("Remettre comme avant").clicked();
                        retour = ui.button("Retour").clicked();
                    });
                });
                if confirmer {
                    self.occupation = Occupation::Annulation;
                    self.moteur.envoyer(Commande::Annuler);
                    Dialogue::Aucun
                } else if retour || echappe {
                    Dialogue::Aucun
                } else {
                    Dialogue::ConfirmationAnnulation
                }
            }

            Dialogue::BilanAnnulation(bilan) => {
                let mut fermer = false;
                modale(ctx, "Réglages remis", |ui| {
                    ui.label(match bilan.remis {
                        0 => "Aucun réglage à remettre.".to_string(),
                        1 => "1 réglage remis comme avant.".to_string(),
                        n => format!("{n} réglages remis comme avant."),
                    });
                    if !bilan.a_reinstaller.is_empty() {
                        ui.add_space(6.0);
                        ui.label("Applis retirées, à réinstaller depuis le Microsoft Store si vous les voulez :");
                        egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                            ui.label(RichText::new(bilan.a_reinstaller.join(", ")).weak());
                        });
                    }
                    for echec in &bilan.echecs {
                        ui.colored_label(ui.visuals().error_fg_color, echec);
                    }
                    ui.add_space(8.0);
                    ui.label("Redémarrez le PC pour que tout soit pris en compte.");
                    ui.add_space(12.0);
                    fermer = ui.button("Fermer").clicked();
                });
                if fermer {
                    Dialogue::Aucun
                } else {
                    Dialogue::BilanAnnulation(bilan)
                }
            }
        };
    }
}

/// Affiche une boîte par-dessus la fenêtre. Rend vrai si on a voulu la quitter
/// (touche Échap ou clic à côté).
fn modale(ctx: &egui::Context, titre: &str, contenu: impl FnOnce(&mut egui::Ui)) -> bool {
    egui::Modal::new(Id::new("dialogue"))
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.label(RichText::new(titre).size(18.0).strong());
            ui.add_space(8.0);
            contenu(ui);
        })
        .should_close()
}

/// Tailles de texte et espacements : un peu plus aérés que les réglages d'origine.
fn habiller(ctx: &egui::Context) {
    ctx.all_styles_mut(|style| {
        for (genre, police) in style.text_styles.iter_mut() {
            police.size = match genre {
                egui::TextStyle::Heading => 20.0,
                egui::TextStyle::Small => 12.0,
                _ => 15.0,
            };
        }
        style.spacing.item_spacing = egui::vec2(10.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
    });
}

impl eframe::App for Fenetre {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.afficher(ui);
    }
}
