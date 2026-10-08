//! Un Windows en mémoire, pour les tests et pour la démonstration hors Windows.

use std::collections::{BTreeMap, BTreeSet};

use crate::systeme::*;

#[derive(Clone, Debug, PartialEq)]
pub struct FauxSysteme {
    pub registre: BTreeMap<(Ruche2, String, String), ValeurBrute>,
    pub services: BTreeMap<String, u32>,
    /// Services en cours d'exécution.
    pub en_marche: BTreeSet<String>,
    pub taches: BTreeMap<String, bool>,
    pub paquets: Vec<String>,
    pub onedrive: bool,
    /// Ce que répond la création d'un point de restauration.
    pub point_restauration: Result<PointRestauration, String>,
    pub points_crees: u32,
    /// Noms (appli, service, tâche ou valeur de registre) dont la modification échoue.
    pub en_panne: BTreeSet<String>,
    /// Si vrai, la liste des applis ne peut pas être lue.
    pub paquets_illisibles: bool,
    /// Numéro de version de Windows ; `None` pour un PC sous Linux.
    pub build: Option<u32>,
    /// Clés (fichiers, schémas) qui n'existent pas sur ce PC.
    pub inapplicables: BTreeSet<String>,
    /// Nom de la distribution, pour un PC sous Linux.
    pub distribution: Option<String>,
}

/// `Ruche` ordonnable, pour servir de clé.
pub type Ruche2 = u8;

fn code(ruche: Ruche) -> Ruche2 {
    match ruche {
        Ruche::Utilisateur => 0,
        Ruche::Machine => 1,
        Ruche::Fichier => 2,
        Ruche::Gsettings => 3,
        Ruche::Pro => 4,
    }
}

fn cle_registre(ruche: Ruche, cle: &str, nom: &str) -> (Ruche2, String, String) {
    // Le registre de Windows ne tient pas compte de la casse.
    (code(ruche), cle.to_lowercase(), nom.to_lowercase())
}

impl Default for FauxSysteme {
    fn default() -> Self {
        FauxSysteme {
            registre: BTreeMap::new(),
            services: BTreeMap::new(),
            en_marche: BTreeSet::new(),
            taches: BTreeMap::new(),
            paquets: Vec::new(),
            onedrive: false,
            point_restauration: Ok(PointRestauration::Cree),
            points_crees: 0,
            en_panne: BTreeSet::new(),
            paquets_illisibles: false,
            build: Some(19045),
            inapplicables: BTreeSet::new(),
            distribution: None,
        }
    }
}

impl FauxSysteme {
    pub fn vide() -> Self {
        Self::default()
    }

    /// Un Windows 10 fraîchement installé, tel qu'on le trouve sur un PC du commerce.
    pub fn windows_10_typique() -> Self {
        let mut sys = Self::default();
        sys.paquets = [
            "king.com.CandyCrushSaga",
            "king.com.CandyCrushFriends",
            "Microsoft.MicrosoftSolitaireCollection",
            "Microsoft.SkypeApp",
            "Microsoft.MicrosoftOfficeHub",
            "Microsoft.Microsoft3DViewer",
            "Microsoft.MixedReality.Portal",
            "Microsoft.WindowsFeedbackHub",
            "Microsoft.GetHelp",
            "Microsoft.Getstarted",
            "Microsoft.MSPaint",
            "Microsoft.XboxApp",
            "Microsoft.XboxGamingOverlay",
            "Microsoft.XboxGameOverlay",
            "Microsoft.XboxIdentityProvider",
            "Microsoft.Xbox.TCUI",
            "Microsoft.XboxGameCallableUI",
            "Microsoft.549981C3F5F10",
            "Microsoft.Windows.Photos",
            "microsoft.windowscommunicationsapps",
            "Microsoft.People",
            "Microsoft.BingWeather",
            "Microsoft.WindowsMaps",
            "Microsoft.WindowsCalculator",
            "Microsoft.ZuneMusic",
            "Microsoft.ZuneVideo",
            "Microsoft.YourPhone",
            "Microsoft.Office.OneNote",
            "Microsoft.MicrosoftStickyNotes",
            "Microsoft.WindowsCamera",
            "Microsoft.WindowsAlarms",
            "Microsoft.WindowsSoundRecorder",
            "SpotifyAB.SpotifyMusic",
            "Disney.37853FC22B2CE",
            "Microsoft.WindowsStore",
            "Microsoft.StorePurchaseApp",
            "Microsoft.DesktopAppInstaller",
            "Microsoft.MicrosoftEdge",
            "Microsoft.Windows.ShellExperienceHost",
            "Microsoft.VCLibs.140.00",
            "Microsoft.NET.Native.Framework.2.2",
            "Microsoft.UI.Xaml.2.8",
            "Microsoft.HEIFImageExtension",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        sys.onedrive = true;
        sys.services.insert("DiagTrack".into(), DEMARRAGE_AUTO);
        sys.en_marche.insert("DiagTrack".into());
        sys.services.insert("dmwappushservice".into(), DEMARRAGE_MANUEL);
        for tache in [
            r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser",
            r"\Microsoft\Windows\Application Experience\ProgramDataUpdater",
            r"\Microsoft\Windows\Autochk\Proxy",
            r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator",
            r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip",
            r"\Microsoft\Windows\DiskDiagnostic\Microsoft-Windows-DiskDiagnosticDataCollector",
            r"\Microsoft\Windows\Feedback\Siuf\DmClient",
            r"\Microsoft\Windows\Windows Error Reporting\QueueReporting",
        ] {
            sys.taches.insert(tache.into(), true);
        }
        // Quelques réglages que Windows pose lui-même, pour que l'annulation
        // ait autre chose à remettre que « valeur absente ».
        let cdm = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
        for nom in ["SilentInstalledAppsEnabled", "SystemPaneSuggestionsEnabled", "SoftLandingEnabled"] {
            sys.poser(Ruche::Utilisateur, cdm, nom, 1);
        }
        sys.poser(
            Ruche::Utilisateur,
            r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo",
            "Enabled",
            1,
        );
        sys
    }

    /// Un Windows 11 du commerce : comme Windows 10, avec ses propres applis en plus.
    pub fn windows_11_typique() -> Self {
        let mut sys = Self::windows_10_typique();
        sys.build = Some(22631);
        sys.paquets.retain(|p| {
            !["Microsoft.Microsoft3DViewer", "Microsoft.MixedReality.Portal", "Microsoft.SkypeApp", "Microsoft.MSPaint"]
                .contains(&p.as_str())
        });
        for nom in [
            "MicrosoftTeams",
            "MSTeams",
            "Clipchamp.Clipchamp",
            "Microsoft.Copilot",
            "Microsoft.Todos",
            "Microsoft.PowerAutomateDesktop",
            "MicrosoftCorporationII.MicrosoftFamily",
            "MicrosoftCorporationII.QuickAssist",
            "Microsoft.OutlookForWindows",
            "Microsoft.Windows.DevHome",
            "Microsoft.BingNews",
            "Microsoft.GamingApp",
            "Microsoft.Paint",
            "Microsoft.WindowsNotepad",
            "Microsoft.WindowsTerminal",
            "MicrosoftWindows.Client.WebExperience",
            "Microsoft.Windows.Ai.Copilot.Provider",
        ] {
            sys.paquets.push(nom.to_string());
        }
        sys
    }

    /// Un Ubuntu de bureau fraîchement installé.
    pub fn ubuntu_typique() -> Self {
        let mut sys = Self::default();
        sys.build = None;
        sys.distribution = Some("Ubuntu 24.04.1 LTS".to_string());
        sys.paquets = [
            "aisleriot",
            "gnome-mahjongg",
            "gnome-mines",
            "gnome-sudoku",
            "thunderbird",
            "rhythmbox",
            "cheese",
            "shotwell",
            "transmission-gtk",
            "remmina",
            "ubuntu-report",
            "systemd",
            "bash",
            "sudo",
            "ubuntu-desktop",
            "gnome-shell",
            "network-manager",
            "firefox",
            "libreoffice-writer",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        for service in ["apport.service", "whoopsie.service", "kerneloops.service"] {
            sys.services.insert(service.into(), DEMARRAGE_AUTO);
            sys.en_marche.insert(service.into());
        }
        sys.poser_texte(Ruche::Fichier, "/etc/default/motd-news", "ENABLED", "1");
        sys.poser_texte(Ruche::Fichier, "/etc/default/apport", "enabled", "1");
        sys.poser_texte(Ruche::Fichier, "/etc/popularity-contest.conf", "PARTICIPATE", "\"yes\"");
        sys.poser_texte(Ruche::Pro, "config", "apt_news", "True");
        sys.poser_texte(Ruche::Gsettings, "org.gnome.desktop.privacy", "report-technical-problems", "true");
        sys.poser_texte(Ruche::Gsettings, "org.gnome.desktop.privacy", "send-software-usage-stats", "false");
        sys
    }

    pub fn poser_texte(&mut self, ruche: Ruche, cle: &str, nom: &str, valeur: &str) {
        self.registre.insert(cle_registre(ruche, cle, nom), ValeurBrute::texte(valeur));
    }

    pub fn poser(&mut self, ruche: Ruche, cle: &str, nom: &str, valeur: u32) {
        self.registre.insert(cle_registre(ruche, cle, nom), ValeurBrute::dword(valeur));
    }

    pub fn valeur(&self, ruche: Ruche, cle: &str, nom: &str) -> Option<u32> {
        self.registre.get(&cle_registre(ruche, cle, nom)).and_then(|v| v.en_dword())
    }

    fn verifier(&self, nom: &str) -> Result<(), String> {
        if self.en_panne.contains(nom) {
            Err(format!("accès refusé à {nom}"))
        } else {
            Ok(())
        }
    }
}

impl Systeme for FauxSysteme {
    fn build_windows(&self) -> Option<u32> {
        self.build
    }

    fn description(&self) -> Option<String> {
        self.distribution.clone().or_else(|| self.build.map(nom_windows))
    }

    fn reglage_applicable(&self, _ruche: Ruche, cle: &str, _nom: &str) -> bool {
        !self.inapplicables.contains(cle)
    }

    fn lire_valeur(&self, ruche: Ruche, cle: &str, nom: &str) -> Result<Option<ValeurBrute>, String> {
        Ok(self.registre.get(&cle_registre(ruche, cle, nom)).cloned())
    }

    fn ecrire_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str, valeur: &ValeurBrute) -> Result<(), String> {
        self.verifier(nom)?;
        self.registre.insert(cle_registre(ruche, cle, nom), valeur.clone());
        Ok(())
    }

    fn supprimer_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str) -> Result<(), String> {
        self.verifier(nom)?;
        self.registre.remove(&cle_registre(ruche, cle, nom));
        Ok(())
    }

    fn demarrage_service(&self, nom: &str) -> Result<Option<u32>, String> {
        Ok(self.services.get(nom).copied())
    }

    fn regler_service(&mut self, nom: &str, demarrage: u32) -> Result<(), String> {
        self.verifier(nom)?;
        match self.services.get_mut(nom) {
            Some(actuel) => {
                *actuel = demarrage;
                Ok(())
            }
            None => Err(format!("le service {nom} n'existe pas")),
        }
    }

    fn arreter_service(&mut self, nom: &str) -> Result<(), String> {
        self.en_marche.remove(nom);
        Ok(())
    }

    fn demarrer_service(&mut self, nom: &str) -> Result<(), String> {
        self.en_marche.insert(nom.to_string());
        Ok(())
    }

    fn tache_active(&self, chemin: &str) -> Result<Option<bool>, String> {
        Ok(self.taches.get(chemin).copied())
    }

    fn activer_tache(&mut self, chemin: &str, active: bool) -> Result<(), String> {
        self.verifier(chemin)?;
        match self.taches.get_mut(chemin) {
            Some(actuel) => {
                *actuel = active;
                Ok(())
            }
            None => Err(format!("la tâche {chemin} n'existe pas")),
        }
    }

    fn paquets(&self) -> Result<Vec<String>, String> {
        if self.paquets_illisibles {
            return Err("PowerShell n'a pas répondu".into());
        }
        Ok(self.paquets.clone())
    }

    fn retirer_paquet(&mut self, nom: &str) -> Result<(), String> {
        self.verifier(nom)?;
        self.paquets.retain(|p| p != nom);
        Ok(())
    }

    fn onedrive_present(&self) -> Result<bool, String> {
        Ok(self.onedrive)
    }

    fn desinstaller_onedrive(&mut self) -> Result<(), String> {
        self.verifier("OneDrive")?;
        self.onedrive = false;
        Ok(())
    }

    fn creer_point_restauration(&mut self) -> Result<PointRestauration, String> {
        if self.point_restauration == Ok(PointRestauration::Cree) {
            self.points_crees += 1;
        }
        self.point_restauration.clone()
    }
}
