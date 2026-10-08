// Pas de fenêtre noire de console derrière le programme.
#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui;
use nettoie_win::interface::{Fenetre, Profil};
use nettoie_win::journal::Journal;

#[cfg(windows)]
const PROFIL: Profil = Profil::WINDOWS;
#[cfg(not(windows))]
const PROFIL: Profil = Profil::LINUX;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let demande = |nom: &str| arguments.iter().any(|a| a == nom);

    #[cfg(unix)]
    {
        if demande("--assistant") {
            std::process::exit(sous_linux::assistant());
        }
        if demande("--liste") {
            sous_linux::lister();
            return;
        }
    }
    // `--demonstration` : ouvre la fenêtre sur un PC fictif, sans rien modifier.
    let demonstration = demande("--demonstration");

    #[cfg(windows)]
    if !demonstration && !sous_windows::droits_administrateur() {
        return;
    }

    let profil = if demonstration { Profil::WINDOWS.en_demonstration() } else { PROFIL };
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(profil.titre)
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
        profil.titre,
        options,
        Box::new(move |creation| {
            let ctx = creation.egui_ctx.clone();
            let reveil: Box<dyn Fn() + Send> = Box::new(move || ctx.request_repaint());
            #[cfg(windows)]
            sous_windows::police_du_systeme(&creation.egui_ctx);
            if demonstration {
                let sys = nettoie_win::faux::FauxSysteme::windows_11_typique();
                return Ok(Box::new(Fenetre::nouvelle(Box::new(sys), Journal::en_memoire(), reveil, profil)));
            }
            Ok(Box::new(fenetre(reveil)?))
        }),
    );
    if let Err(erreur) = lancement {
        signaler(&format!("La fenêtre n'a pas pu s'ouvrir : {erreur}"));
    }
}

/// Ouvre le journal des changements. S'il est abîmé, il est mis de côté plutôt qu'écrasé.
fn ouvrir_journal(fichier: std::path::PathBuf) -> Result<(Journal, Option<String>), String> {
    match Journal::ouvrir(fichier.clone()) {
        Ok(journal) => Ok((journal, None)),
        Err(erreur) => {
            let _ = std::fs::rename(&fichier, fichier.with_file_name("journal-illisible.json"));
            let message =
                format!("Le journal des changements précédents est illisible et a été mis de côté ({erreur}).");
            Ok((Journal::ouvrir(fichier)?, Some(message)))
        }
    }
}

fn avec_journal(
    sys: Box<dyn nettoie_win::systeme::Systeme + Send>,
    fichier: std::path::PathBuf,
    reveil: Box<dyn Fn() + Send>,
) -> Result<Fenetre, String> {
    let (journal, avertissement) = ouvrir_journal(fichier)?;
    let mut fenetre = Fenetre::nouvelle(sys, journal, reveil, PROFIL);
    if let Some(message) = avertissement {
        fenetre.avertir(message);
    }
    Ok(fenetre)
}

#[cfg(windows)]
fn fenetre(reveil: Box<dyn Fn() + Send>) -> Result<Fenetre, String> {
    let sys = nettoie_win::windows::SystemeWindows::nouveau()?;
    let dossier = std::env::var_os("LOCALAPPDATA").ok_or("dossier de données de l'utilisateur introuvable")?;
    let fichier = std::path::PathBuf::from(dossier).join("Nettoie-Win").join("journal.json");
    avec_journal(Box::new(sys), fichier, reveil)
}

#[cfg(unix)]
fn fenetre(reveil: Box<dyn Fn() + Send>) -> Result<Fenetre, String> {
    let sys = nettoie_win::linux::SystemeLinux::nouveau(PROFIL.catalogue);
    avec_journal(Box::new(sys), sous_linux::fichier_journal()?, reveil)
}

#[cfg(unix)]
mod sous_linux {
    use nettoie_win::analyse::analyser;
    use nettoie_win::assistant::servir;
    use nettoie_win::catalogue_linux::catalogue;
    use nettoie_win::linux::{est_root, executer_en_root, SystemeLinux};
    use nettoie_win::systeme::Systeme;

    /// `~/.local/state/nettoie-linux/journal.json`, ou l'équivalent choisi par l'utilisateur.
    pub fn fichier_journal() -> Result<std::path::PathBuf, String> {
        let dossier = match std::env::var_os("XDG_STATE_HOME").filter(|d| !d.is_empty()) {
            Some(dossier) => std::path::PathBuf::from(dossier),
            None => std::path::PathBuf::from(std::env::var_os("HOME").ok_or("dossier personnel introuvable")?)
                .join(".local")
                .join("state"),
        };
        Ok(dossier.join("nettoie-linux").join("journal.json"))
    }

    /// La partie lancée en administrateur par la fenêtre : exécute ses demandes, rien d'autre.
    pub fn assistant() -> i32 {
        if !est_root() {
            eprintln!("L'assistant ne sert qu'à la fenêtre de Nettoie-Linux, qui le lance elle-même en administrateur.");
            return 1;
        }
        servir(std::io::stdin().lock(), std::io::stdout(), catalogue(), &mut |demande| executer_en_root(demande));
        0
    }

    /// `--liste` : écrit ce qui serait proposé sur ce PC, sans fenêtre et sans rien modifier.
    pub fn lister() {
        let sys = SystemeLinux::nouveau(catalogue());
        let analyse = analyser(&sys, catalogue());
        println!("{}", sys.description().unwrap_or_else(|| "Système non reconnu".to_string()));
        for avertissement in &analyse.avertissements {
            println!("! {avertissement}");
        }
        for ligne in &analyse.lignes {
            let case = if ligne.element.coche_par_defaut() { "x" } else { " " };
            println!("[{case}] {}", ligne.element.nom);
            for travail in &ligne.travaux {
                println!("      {}", travail.decrire());
            }
        }
        println!(
            "{} élément(s) proposé(s), {} déjà réglé(s) ou absent(s) de ce PC.",
            analyse.lignes.len(),
            analyse.masques
        );
    }
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
        let titre = large(OsStr::new(super::PROFIL.titre));
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
