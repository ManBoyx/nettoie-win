// Pas de fenêtre noire de console derrière le programme.
#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui;
use nettoie_win::interface::Fenetre;

const TITRE: &str = "Nettoie-Win";

fn main() {
    #[cfg(windows)]
    if !sous_windows::droits_administrateur() {
        return;
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(TITRE)
            .with_icon(egui::IconData {
                rgba: include_bytes!("../ressources/icone-64.rgba").to_vec(),
                width: 64,
                height: 64,
            })
            .with_inner_size([900.0, 680.0])
            .with_min_inner_size([700.0, 480.0]),
        ..Default::default()
    };
    let lancement = eframe::run_native(
        TITRE,
        options,
        Box::new(|creation| {
            let ctx = creation.egui_ctx.clone();
            let reveil = Box::new(move || ctx.request_repaint());
            #[cfg(windows)]
            sous_windows::police_du_systeme(&creation.egui_ctx);
            Ok(Box::new(fenetre(reveil)?))
        }),
    );
    if let Err(erreur) = lancement {
        signaler(&format!("La fenêtre n'a pas pu s'ouvrir : {erreur}"));
    }
}

#[cfg(windows)]
fn fenetre(reveil: Box<dyn Fn() + Send>) -> Result<Fenetre, String> {
    use nettoie_win::journal::Journal;
    use nettoie_win::windows::SystemeWindows;

    let sys = SystemeWindows::nouveau()?;
    let dossier = std::env::var_os("LOCALAPPDATA").ok_or("dossier de données de l'utilisateur introuvable")?;
    let fichier = std::path::PathBuf::from(dossier).join(TITRE).join("journal.json");
    let mut avertissement = None;
    let journal = match Journal::ouvrir(fichier.clone()) {
        Ok(journal) => journal,
        Err(erreur) => {
            // On met de côté le journal abîmé plutôt que de l'écraser.
            let _ = std::fs::rename(&fichier, fichier.with_file_name("journal-illisible.json"));
            avertissement = Some(format!(
                "Le journal des changements précédents est illisible et a été mis de côté ({erreur})."
            ));
            Journal::ouvrir(fichier)?
        }
    };
    let mut fenetre = Fenetre::nouvelle(Box::new(sys), journal, reveil, false);
    if let Some(message) = avertissement {
        fenetre.avertir(message);
    }
    Ok(fenetre)
}

/// Hors de Windows, le programme tourne sur un faux PC : cela sert à voir la fenêtre.
#[cfg(not(windows))]
fn fenetre(reveil: Box<dyn Fn() + Send>) -> Result<Fenetre, String> {
    use nettoie_win::faux::FauxSysteme;
    use nettoie_win::journal::Journal;

    Ok(Fenetre::nouvelle(Box::new(FauxSysteme::windows_10_typique()), Journal::en_memoire(), reveil, true))
}

#[cfg(windows)]
fn signaler(message: &str) {
    sous_windows::boite(message);
}

#[cfg(not(windows))]
fn signaler(message: &str) {
    eprintln!("{message}");
}

#[cfg(windows)]
mod sous_windows {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    use std::ptr::{null, null_mut};

    use eframe::egui;
    use windows_sys::Win32::UI::Shell::{IsUserAnAdmin, ShellExecuteW};
    use windows_sys::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, SW_SHOWNORMAL};

    fn large(texte: &OsStr) -> Vec<u16> {
        texte.encode_wide().chain(std::iter::once(0)).collect()
    }

    pub fn boite(message: &str) {
        let texte = large(OsStr::new(message));
        let titre = large(OsStr::new(super::TITRE));
        unsafe {
            MessageBoxW(null_mut(), texte.as_ptr(), titre.as_ptr(), MB_OK | MB_ICONERROR);
        }
    }

    /// Vrai si le programme a déjà les droits. Sinon il se relance en les
    /// demandant (la fenêtre « Voulez-vous autoriser… » de Windows) et rend faux.
    pub fn droits_administrateur() -> bool {
        if unsafe { IsUserAnAdmin() } != 0 {
            return true;
        }
        let relance = std::env::current_exe().ok().map(|programme| {
            let programme = large(programme.as_os_str());
            let verbe = large(OsStr::new("runas"));
            unsafe { ShellExecuteW(null_mut(), verbe.as_ptr(), programme.as_ptr(), null(), null(), SW_SHOWNORMAL) }
        });
        // ShellExecute rend une valeur supérieure à 32 quand le lancement a réussi.
        if !relance.is_some_and(|code| code as isize > 32) {
            boite("Nettoie-Win a besoin des droits d'administrateur pour modifier Windows.");
        }
        false
    }

    /// Utilise Segoe UI, la police de Windows, à la place de celle fournie avec la bibliothèque.
    pub fn police_du_systeme(ctx: &egui::Context) {
        let Some(windows) = std::env::var_os("WINDIR") else {
            return;
        };
        let fichier = std::path::PathBuf::from(windows).join("Fonts").join("segoeui.ttf");
        let Ok(octets) = std::fs::read(fichier) else {
            return;
        };
        let mut polices = egui::FontDefinitions::default();
        polices.font_data.insert("segoe".to_owned(), egui::FontData::from_owned(octets).into());
        if let Some(famille) = polices.families.get_mut(&egui::FontFamily::Proportional) {
            famille.insert(0, "segoe".to_owned());
        }
        ctx.set_fonts(polices);
    }
}
