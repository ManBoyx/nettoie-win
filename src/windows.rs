//! La vraie version de `Systeme` : registre, services, tâches planifiées et applis de Windows.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_PATH_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, HKEY,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_QUERY_VALUE, KEY_SET_VALUE, REG_OPTION_NON_VOLATILE,
};
use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

use crate::systeme::*;
use crate::texte;

/// Empêche l'apparition d'une fenêtre noire à chaque commande lancée.
const SANS_FENETRE: u32 = 0x0800_0000;

const DESINSTALLEURS_ONEDRIVE: [(Ruche, &str); 3] = [
    (Ruche::Utilisateur, r"Software\Microsoft\Windows\CurrentVersion\Uninstall\OneDriveSetup.exe"),
    (Ruche::Machine, r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\OneDriveSetup.exe"),
    (Ruche::Machine, r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\OneDriveSetup.exe"),
];

pub struct SystemeWindows {
    /// `C:\Windows\System32`, demandé à Windows plutôt que deviné : les
    /// commandes sont lancées par leur chemin complet, jamais par leur seul nom.
    systeme32: PathBuf,
}

/// Résultat d'une commande : a-t-elle réussi, et ce qu'elle a écrit.
struct Sortie {
    reussie: bool,
    texte: String,
}

fn large(texte: &str) -> Vec<u16> {
    OsStr::new(texte).encode_wide().chain(std::iter::once(0)).collect()
}

fn racine(ruche: Ruche) -> Result<HKEY, String> {
    match ruche {
        Ruche::Utilisateur => Ok(HKEY_CURRENT_USER),
        Ruche::Machine => Ok(HKEY_LOCAL_MACHINE),
        Ruche::Fichier | Ruche::Gsettings | Ruche::Pro => Err("réglage sans objet sous Windows".to_string()),
    }
}

fn message(code: u32) -> String {
    std::io::Error::from_raw_os_error(code as i32).to_string()
}

fn absent(code: u32) -> bool {
    code == ERROR_FILE_NOT_FOUND || code == ERROR_PATH_NOT_FOUND
}

/// Clé de registre ouverte, refermée automatiquement.
struct Cle(HKEY);

impl Drop for Cle {
    fn drop(&mut self) {
        unsafe {
            RegCloseKey(self.0);
        }
    }
}

impl Cle {
    /// `Ok(None)` si la clé n'existe pas.
    fn ouvrir(ruche: Ruche, cle: &str, droits: u32) -> Result<Option<Cle>, String> {
        let chemin = large(cle);
        let mut poignee: HKEY = null_mut();
        let code = unsafe { RegOpenKeyExW(racine(ruche)?, chemin.as_ptr(), 0, droits, &mut poignee) };
        match code {
            ERROR_SUCCESS => Ok(Some(Cle(poignee))),
            code if absent(code) => Ok(None),
            code => Err(message(code)),
        }
    }

    fn creer(ruche: Ruche, cle: &str) -> Result<Cle, String> {
        let chemin = large(cle);
        let racine = racine(ruche)?;
        let mut poignee: HKEY = null_mut();
        let code = unsafe {
            RegCreateKeyExW(
                racine,
                chemin.as_ptr(),
                0,
                null(),
                REG_OPTION_NON_VOLATILE,
                KEY_SET_VALUE,
                null(),
                &mut poignee,
                null_mut(),
            )
        };
        match code {
            ERROR_SUCCESS => Ok(Cle(poignee)),
            code => Err(message(code)),
        }
    }
}

impl SystemeWindows {
    pub fn nouveau() -> Result<Self, String> {
        let mut tampon = [0u16; 512];
        let longueur = unsafe { GetSystemDirectoryW(tampon.as_mut_ptr(), tampon.len() as u32) } as usize;
        if longueur == 0 || longueur >= tampon.len() {
            return Err("dossier système de Windows introuvable".to_string());
        }
        Ok(SystemeWindows { systeme32: PathBuf::from(String::from_utf16_lossy(&tampon[..longueur])) })
    }

    fn lancer(&self, programme: PathBuf, arguments: &[&str]) -> Result<Sortie, String> {
        let sortie = Command::new(&programme)
            .args(arguments)
            .stdin(Stdio::null())
            .creation_flags(SANS_FENETRE)
            .output()
            .map_err(|e| format!("{} : {e}", programme.display()))?;
        let mut texte = texte::decoder_sortie(&sortie.stdout);
        texte.push_str(&texte::decoder_sortie(&sortie.stderr));
        Ok(Sortie { reussie: sortie.status.success(), texte })
    }

    fn outil(&self, nom: &str, arguments: &[&str]) -> Result<Sortie, String> {
        self.lancer(self.systeme32.join(nom), arguments)
    }

    /// Lance un outil et transforme un échec en erreur lisible.
    fn outil_exige(&self, nom: &str, arguments: &[&str]) -> Result<(), String> {
        let sortie = self.outil(nom, arguments)?;
        if sortie.reussie {
            Ok(())
        } else {
            Err(sortie.texte.split_whitespace().collect::<Vec<_>>().join(" "))
        }
    }

    fn powershell(&self, script: &str) -> Result<String, String> {
        let programme = self.systeme32.join(r"WindowsPowerShell\v1.0\powershell.exe");
        let arguments = ["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script];
        Ok(self.lancer(programme, &arguments)?.texte)
    }
}

impl Systeme for SystemeWindows {
    fn build_windows(&self) -> Option<u32> {
        let valeur = self
            .lire_valeur(Ruche::Machine, r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", "CurrentBuildNumber")
            .ok()??;
        texte::build_depuis_registre(&valeur.octets)
    }

    fn reglage_applicable(&self, ruche: Ruche, _cle: &str, _nom: &str) -> bool {
        matches!(ruche, Ruche::Utilisateur | Ruche::Machine)
    }

    fn lire_valeur(&self, ruche: Ruche, cle: &str, nom: &str) -> Result<Option<ValeurBrute>, String> {
        let Some(ouverte) = Cle::ouvrir(ruche, cle, KEY_QUERY_VALUE)? else {
            return Ok(None);
        };
        let nom = large(nom);
        let mut genre = 0u32;
        let mut taille = 0u32;
        let code =
            unsafe { RegQueryValueExW(ouverte.0, nom.as_ptr(), null(), &mut genre, null_mut(), &mut taille) };
        if absent(code) {
            return Ok(None);
        }
        if code != ERROR_SUCCESS {
            return Err(message(code));
        }
        let mut octets = vec![0u8; taille as usize];
        let code = unsafe {
            RegQueryValueExW(ouverte.0, nom.as_ptr(), null(), &mut genre, octets.as_mut_ptr(), &mut taille)
        };
        if code != ERROR_SUCCESS {
            return Err(message(code));
        }
        octets.truncate(taille as usize);
        Ok(Some(ValeurBrute { genre, octets }))
    }

    fn ecrire_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str, valeur: &ValeurBrute) -> Result<(), String> {
        let ouverte = Cle::creer(ruche, cle)?;
        let nom = large(nom);
        let code = unsafe {
            RegSetValueExW(
                ouverte.0,
                nom.as_ptr(),
                0,
                valeur.genre,
                valeur.octets.as_ptr(),
                valeur.octets.len() as u32,
            )
        };
        match code {
            ERROR_SUCCESS => Ok(()),
            code => Err(message(code)),
        }
    }

    fn supprimer_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str) -> Result<(), String> {
        let Some(ouverte) = Cle::ouvrir(ruche, cle, KEY_SET_VALUE)? else {
            return Ok(());
        };
        let nom = large(nom);
        match unsafe { RegDeleteValueW(ouverte.0, nom.as_ptr()) } {
            ERROR_SUCCESS => Ok(()),
            code if absent(code) => Ok(()),
            code => Err(message(code)),
        }
    }

    fn demarrage_service(&self, nom: &str) -> Result<Option<u32>, String> {
        let cle = format!(r"SYSTEM\CurrentControlSet\Services\{nom}");
        Ok(self.lire_valeur(Ruche::Machine, &cle, "Start")?.and_then(|v| v.en_dword()))
    }

    fn regler_service(&mut self, nom: &str, demarrage: u32) -> Result<(), String> {
        let mode = match demarrage {
            DEMARRAGE_AUTO => "auto",
            DEMARRAGE_MANUEL => "demand",
            DEMARRAGE_DESACTIVE => "disabled",
            autre => return Err(format!("type de démarrage inattendu ({autre})")),
        };
        self.outil_exige("sc.exe", &["config", nom, "start=", mode])
    }

    fn arreter_service(&mut self, nom: &str) -> Result<(), String> {
        self.outil_exige("sc.exe", &["stop", nom])
    }

    fn demarrer_service(&mut self, nom: &str) -> Result<(), String> {
        self.outil_exige("sc.exe", &["start", nom])
    }

    fn tache_active(&self, chemin: &str) -> Result<Option<bool>, String> {
        let sortie = self.outil("schtasks.exe", &["/Query", "/TN", chemin, "/XML"])?;
        if !sortie.reussie {
            return Ok(None);
        }
        Ok(Some(texte::tache_active_depuis_xml(&sortie.texte)))
    }

    fn activer_tache(&mut self, chemin: &str, active: bool) -> Result<(), String> {
        let sens = if active { "/ENABLE" } else { "/DISABLE" };
        self.outil_exige("schtasks.exe", &["/Change", "/TN", chemin, sens])
    }

    fn paquets(&self) -> Result<Vec<String>, String> {
        texte::paquets_depuis_sortie(&self.powershell(texte::SCRIPT_LISTE)?)
    }

    fn retirer_paquet(&mut self, nom: &str) -> Result<(), String> {
        let script = texte::script_retrait(nom)?;
        texte::retrait_depuis_sortie(&self.powershell(&script)?)
    }

    fn onedrive_present(&self) -> Result<bool, String> {
        for (ruche, cle) in DESINSTALLEURS_ONEDRIVE {
            if self.lire_valeur(ruche, cle, "UninstallString")?.is_some() {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn desinstaller_onedrive(&mut self) -> Result<(), String> {
        let windows = self.systeme32.parent().map(PathBuf::from).unwrap_or_default();
        let programme = [windows.join("SysWOW64"), self.systeme32.clone()]
            .into_iter()
            .map(|dossier| dossier.join("OneDriveSetup.exe"))
            .find(|chemin| chemin.exists())
            .ok_or_else(|| "programme de désinstallation de OneDrive introuvable".to_string())?;
        // OneDrive doit être fermé pour se laisser désinstaller ; sans gravité s'il ne tournait pas.
        let _ = self.outil("taskkill.exe", &["/F", "/IM", "OneDrive.exe"]);
        if !self.lancer(programme, &["/uninstall"])?.reussie {
            return Err("la désinstallation de OneDrive a échoué".to_string());
        }
        // Le programme de désinstallation rend la main avant d'avoir fini : on
        // attend que OneDrive ait vraiment disparu avant de l'annoncer.
        for _ in 0..60 {
            if !self.onedrive_present()? {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(500));
        }
        Err("OneDrive est toujours présent après la désinstallation".to_string())
    }

    fn creer_point_restauration(&mut self) -> Result<PointRestauration, String> {
        texte::point_depuis_sortie(&self.powershell(texte::SCRIPT_POINT)?)
    }
}
