//! La liste de tout ce que le programme sait retirer ou régler.

use crate::motif::correspond;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Categorie {
    Applis,
    Pubs,
    Telemetrie,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Risque {
    /// Coché d'office.
    SansRisque,
    /// Proposé mais décoché : peut manquer à quelqu'un.
    Attention,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Ruche {
    /// HKEY_CURRENT_USER
    Utilisateur,
    /// HKEY_LOCAL_MACHINE
    Machine,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    /// Retirer les applis dont le nom correspond au motif.
    Paquet(&'static str),
    /// Mettre une valeur numérique dans le registre.
    Registre {
        ruche: Ruche,
        cle: &'static str,
        nom: &'static str,
        valeur: u32,
    },
    /// Désactiver un service Windows.
    Service(&'static str),
    /// Désactiver une tâche planifiée.
    Tache(&'static str),
    /// Désinstaller OneDrive.
    OneDrive,
}

#[derive(Debug)]
pub struct Element {
    pub id: &'static str,
    pub categorie: Categorie,
    pub nom: &'static str,
    pub explication: &'static str,
    pub risque: Risque,
    pub actions: &'static [Action],
}

impl Element {
    pub fn coche_par_defaut(&self) -> bool {
        self.risque == Risque::SansRisque
    }
}

/// Applis que le programme refuse toujours de retirer, même si un motif du
/// catalogue venait à les désigner.
pub const PROTEGES: &[&str] = &[
    // Le Store et ce qui installe les applis.
    "Microsoft.WindowsStore",
    "Microsoft.StorePurchaseApp",
    "Microsoft.DesktopAppInstaller",
    "Microsoft.Services.Store.Engagement*",
    // Edge et les composants web utilisés par d'autres programmes.
    "Microsoft.MicrosoftEdge*",
    "Microsoft.Win32WebViewHost",
    // Composants du système (menu Démarrer, recherche, Paramètres, sécurité...).
    "Microsoft.Windows.*",
    "MicrosoftWindows.*",
    "Windows.*",
    "Microsoft.SecHealthUI",
    "Microsoft.AAD.*",
    "Microsoft.AccountsControl",
    "Microsoft.LockApp",
    "Microsoft.CredDialogHost",
    "Microsoft.BioEnrollment",
    "Microsoft.ECApp",
    "Microsoft.AsyncTextService",
    "Microsoft.XboxGameCallableUI",
    "InputApp",
    // Bibliothèques dont dépendent les autres applis.
    "Microsoft.VCLibs*",
    "Microsoft.NET.*",
    "Microsoft.UI.Xaml*",
    "Microsoft.WindowsAppRuntime*",
    "Microsoft.DirectX*",
    "Microsoft.*Extension*",
    "Microsoft.LanguageExperiencePack*",
    // Panneaux de réglage du matériel.
    "NVIDIACorp.*",
    "RealtekSemiconductorCorp.*",
    "AppUp.Intel*",
    "AdvancedMicroDevicesInc*",
    "DolbyLaboratories.*",
];

/// Exceptions à `PROTEGES` : applis ordinaires dont le nom ressemble à celui
/// d'un composant du système.
pub const NON_PROTEGES: &[&str] = &["Microsoft.Windows.Photos"];

pub fn est_protege(nom: &str) -> bool {
    if NON_PROTEGES.iter().any(|m| correspond(m, nom)) {
        return false;
    }
    PROTEGES.iter().any(|m| correspond(m, nom))
}

pub fn catalogue() -> &'static [Element] {
    CATALOGUE
}

const fn hkcu(cle: &'static str, nom: &'static str, valeur: u32) -> Action {
    Action::Registre { ruche: Ruche::Utilisateur, cle, nom, valeur }
}

const fn hklm(cle: &'static str, nom: &'static str, valeur: u32) -> Action {
    Action::Registre { ruche: Ruche::Machine, cle, nom, valeur }
}

const fn appli(
    id: &'static str,
    nom: &'static str,
    explication: &'static str,
    risque: Risque,
    actions: &'static [Action],
) -> Element {
    Element { id, categorie: Categorie::Applis, nom, explication, risque, actions }
}

const fn pub_(
    id: &'static str,
    nom: &'static str,
    explication: &'static str,
    actions: &'static [Action],
) -> Element {
    Element { id, categorie: Categorie::Pubs, nom, explication, risque: Risque::SansRisque, actions }
}

const fn telemetrie(
    id: &'static str,
    nom: &'static str,
    explication: &'static str,
    risque: Risque,
    actions: &'static [Action],
) -> Element {
    Element { id, categorie: Categorie::Telemetrie, nom, explication, risque, actions }
}

use Action::{OneDrive, Paquet, Service, Tache};
use Risque::{Attention, SansRisque};

/// Réglages de « contenu suggéré » de Windows.
const CDM: &str = r"Software\Microsoft\Windows\CurrentVersion\ContentDeliveryManager";
const AVANCE: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const COLLECTE: &str = r"SOFTWARE\Policies\Microsoft\Windows\DataCollection";
const ACTIVITE: &str = r"SOFTWARE\Policies\Microsoft\Windows\System";
const SAISIE: &str = r"Software\Microsoft\InputPersonalization";

const CATALOGUE: &[Element] = &[
    // ----- Applis préinstallées -----
    appli(
        "jeux-promo",
        "Jeux promotionnels",
        "Candy Crush, Bubble Witch, Farm Heroes, March of Empires, Hidden City et les autres jeux installés d'office.",
        SansRisque,
        &[
            Paquet("king.com.*"),
            Paquet("*.MarchofEmpires"),
            Paquet("*HiddenCity*"),
            Paquet("*Asphalt8Airborne"),
            Paquet("*RoyalRevolt*"),
            Paquet("*CookingFever"),
            Paquet("*DragonManiaLegends"),
            Paquet("*DisneyMagicKingdoms"),
            Paquet("*FarmVille2CountryEscape"),
        ],
    ),
    appli(
        "solitaire",
        "Solitaire Collection",
        "Jeux de cartes de Microsoft, avec publicités.",
        SansRisque,
        &[Paquet("Microsoft.MicrosoftSolitaireCollection")],
    ),
    appli(
        "skype",
        "Skype",
        "La version Store de Skype, installée d'office.",
        SansRisque,
        &[Paquet("Microsoft.SkypeApp")],
    ),
    appli(
        "office-hub",
        "Raccourci « Office »",
        "Vitrine qui pousse à s'abonner à Microsoft 365. Word et Excel restent en place s'ils sont installés.",
        SansRisque,
        &[Paquet("Microsoft.MicrosoftOfficeHub")],
    ),
    appli(
        "applis-3d",
        "Visionneuse 3D, Print 3D et 3D Builder",
        "Outils pour modèles et impression en 3D.",
        SansRisque,
        &[
            Paquet("Microsoft.Microsoft3DViewer"),
            Paquet("Microsoft.Print3D"),
            Paquet("Microsoft.3DBuilder"),
        ],
    ),
    appli(
        "realite-mixte",
        "Portail de réalité mixte",
        "Ne sert qu'avec un casque Windows Mixed Reality.",
        SansRisque,
        &[Paquet("Microsoft.MixedReality.Portal")],
    ),
    appli(
        "hub-commentaires",
        "Hub de commentaires",
        "Sert à envoyer des avis à Microsoft.",
        SansRisque,
        &[Paquet("Microsoft.WindowsFeedbackHub")],
    ),
    appli(
        "obtenir-aide",
        "Obtenir de l'aide",
        "Assistant de support de Microsoft.",
        SansRisque,
        &[Paquet("Microsoft.GetHelp")],
    ),
    appli(
        "astuces",
        "Astuces",
        "Présentation des nouveautés de Windows.",
        SansRisque,
        &[Paquet("Microsoft.Getstarted")],
    ),
    appli(
        "actualites",
        "Actualités, Sport et Finance",
        "Applis de nouvelles de MSN.",
        SansRisque,
        &[
            Paquet("Microsoft.BingNews"),
            Paquet("Microsoft.BingSports"),
            Paquet("Microsoft.BingFinance"),
            Paquet("Microsoft.News"),
        ],
    ),
    appli(
        "forfaits-mobiles",
        "Forfaits mobiles, Messages et Portefeuille",
        "Applis prévues pour les téléphones et tablettes Windows, sans usage sur un PC.",
        SansRisque,
        &[
            Paquet("Microsoft.OneConnect"),
            Paquet("Microsoft.Messaging"),
            Paquet("Microsoft.Wallet"),
        ],
    ),
    appli(
        "paint-3d",
        "Paint 3D",
        "Le Paint classique reste en place.",
        Attention,
        &[Paquet("Microsoft.MSPaint")],
    ),
    appli(
        "xbox",
        "Xbox et Game Bar",
        "À garder pour les jeux du Microsoft Store ou du Game Pass (Minecraft pour Windows, par exemple) et pour enregistrer l'écran avec Win+G.",
        Attention,
        &[
            Paquet("Microsoft.XboxApp"),
            Paquet("Microsoft.GamingApp"),
            Paquet("Microsoft.XboxGamingOverlay"),
            Paquet("Microsoft.XboxGameOverlay"),
            Paquet("Microsoft.XboxSpeechToTextOverlay"),
            Paquet("Microsoft.XboxIdentityProvider"),
            Paquet("Microsoft.Xbox.TCUI"),
            // Sans ces deux réglages, Windows réclame la Game Bar au lancement d'un jeu.
            hkcu(r"System\GameConfigStore", "GameDVR_Enabled", 0),
            hkcu(r"Software\Microsoft\Windows\CurrentVersion\GameDVR", "AppCaptureEnabled", 0),
        ],
    ),
    appli(
        "onedrive",
        "OneDrive",
        "Les fichiers déjà synchronisés restent sur le PC et en ligne, mais la synchronisation s'arrête.",
        Attention,
        &[OneDrive],
    ),
    appli(
        "cortana",
        "Cortana",
        "L'assistante vocale de Windows.",
        Attention,
        &[Paquet("Microsoft.549981C3F5F10")],
    ),
    appli(
        "photos",
        "Photos",
        "Sans elle, il faut une autre visionneuse pour ouvrir les images.",
        Attention,
        &[Paquet("Microsoft.Windows.Photos")],
    ),
    appli(
        "courrier",
        "Courrier et Calendrier",
        "Les comptes de messagerie configurés dans l'appli ne seront plus relevés.",
        Attention,
        &[Paquet("microsoft.windowscommunicationsapps")],
    ),
    appli(
        "contacts",
        "Contacts",
        "Carnet d'adresses utilisé par Courrier et Calendrier.",
        Attention,
        &[Paquet("Microsoft.People")],
    ),
    appli(
        "meteo",
        "Météo",
        "L'appli météo de MSN.",
        Attention,
        &[Paquet("Microsoft.BingWeather")],
    ),
    appli(
        "cartes",
        "Cartes",
        "Cartes et itinéraires de Microsoft.",
        Attention,
        &[Paquet("Microsoft.WindowsMaps")],
    ),
    appli(
        "calculatrice",
        "Calculatrice",
        "Windows n'en fournit pas d'autre.",
        Attention,
        &[Paquet("Microsoft.WindowsCalculator")],
    ),
    appli(
        "groove-films",
        "Groove Musique et Films et TV",
        "Lecteurs audio et vidéo de Windows. Sans eux, il faut un autre lecteur (VLC, par exemple).",
        Attention,
        &[Paquet("Microsoft.ZuneMusic"), Paquet("Microsoft.ZuneVideo")],
    ),
    appli(
        "votre-telephone",
        "Votre téléphone",
        "Relie un téléphone Android au PC (messages, photos, notifications).",
        Attention,
        &[Paquet("Microsoft.YourPhone")],
    ),
    appli(
        "onenote",
        "OneNote",
        "La version Store du bloc-notes OneNote.",
        Attention,
        &[Paquet("Microsoft.Office.OneNote")],
    ),
    appli(
        "pense-betes",
        "Pense-bêtes",
        "Les notes déjà écrites disparaissent si elles ne sont pas synchronisées avec un compte Microsoft.",
        Attention,
        &[Paquet("Microsoft.MicrosoftStickyNotes")],
    ),
    appli(
        "camera",
        "Caméra",
        "L'appli de Windows pour la webcam.",
        Attention,
        &[Paquet("Microsoft.WindowsCamera")],
    ),
    appli(
        "alarmes",
        "Alarmes et horloge",
        "Alarmes, minuteur et chronomètre.",
        Attention,
        &[Paquet("Microsoft.WindowsAlarms")],
    ),
    appli(
        "enregistreur",
        "Enregistreur vocal",
        "Enregistre le son du micro.",
        Attention,
        &[Paquet("Microsoft.WindowsSoundRecorder")],
    ),
    appli(
        "spotify",
        "Spotify",
        "Souvent installé d'office, mais vous l'avez peut-être installé vous-même.",
        Attention,
        &[Paquet("SpotifyAB.SpotifyMusic")],
    ),
    appli(
        "netflix",
        "Netflix",
        "Souvent installé d'office, mais vous l'avez peut-être installé vous-même.",
        Attention,
        &[Paquet("*.Netflix")],
    ),
    appli(
        "disney",
        "Disney+",
        "Souvent installé d'office, mais vous l'avez peut-être installé vous-même.",
        Attention,
        &[Paquet("Disney.*")],
    ),
    appli(
        "prime-video",
        "Prime Video",
        "Souvent installé d'office, mais vous l'avez peut-être installé vous-même.",
        Attention,
        &[Paquet("AmazonVideo.PrimeVideo")],
    ),
    appli(
        "reseaux-sociaux",
        "Facebook, Instagram, TikTok, Twitter et LinkedIn",
        "Applis Store des réseaux sociaux. Les sites restent accessibles dans le navigateur.",
        Attention,
        &[
            Paquet("Facebook.*"),
            Paquet("*.Twitter"),
            Paquet("*TikTok*"),
            Paquet("*LinkedInforWindows"),
        ],
    ),
    // ----- Pubs et suggestions -----
    pub_(
        "applis-en-douce",
        "Installation d'applis sans demander",
        "Empêche Windows de télécharger tout seul des jeux et applis sponsorisés.",
        &[
            hkcu(CDM, "SilentInstalledAppsEnabled", 0),
            hkcu(CDM, "PreInstalledAppsEnabled", 0),
            hkcu(CDM, "PreInstalledAppsEverEnabled", 0),
            hkcu(CDM, "OemPreInstalledAppsEnabled", 0),
            hklm(r"SOFTWARE\Policies\Microsoft\Windows\CloudContent", "DisableWindowsConsumerFeatures", 1),
        ],
    ),
    pub_(
        "pubs-menu-demarrer",
        "Suggestions dans le menu Démarrer",
        "Retire les applis « suggérées » affichées dans le menu Démarrer.",
        &[
            hkcu(CDM, "SystemPaneSuggestionsEnabled", 0),
            hkcu(CDM, "SubscribedContent-338388Enabled", 0),
        ],
    ),
    pub_(
        "astuces-verrouillage",
        "Astuces sur l'écran de verrouillage",
        "Retire les « anecdotes et conseils » de l'écran de verrouillage. L'image de fond ne change pas.",
        &[
            hkcu(CDM, "RotatingLockScreenOverlayEnabled", 0),
            hkcu(CDM, "SubscribedContent-338387Enabled", 0),
        ],
    ),
    pub_(
        "conseils-windows",
        "Conseils et astuces de Windows",
        "Arrête les notifications « Obtenir des conseils lors de l'utilisation de Windows ».",
        &[
            hkcu(CDM, "SoftLandingEnabled", 0),
            hkcu(CDM, "SubscribedContent-338389Enabled", 0),
        ],
    ),
    pub_(
        "ecran-bienvenue",
        "Écrans « Tirez le meilleur parti de Windows »",
        "Arrête les écrans plein écran qui vantent les services Microsoft après une mise à jour.",
        &[
            hkcu(CDM, "SubscribedContent-310093Enabled", 0),
            hkcu(
                r"Software\Microsoft\Windows\CurrentVersion\UserProfileEngagement",
                "ScoobeSystemSettingEnabled",
                0,
            ),
        ],
    ),
    pub_(
        "suggestions-parametres",
        "Suggestions dans Paramètres et la chronologie",
        "Retire les contenus suggérés affichés dans l'appli Paramètres et dans la vue des tâches.",
        &[
            hkcu(CDM, "SubscribedContent-338393Enabled", 0),
            hkcu(CDM, "SubscribedContent-353694Enabled", 0),
            hkcu(CDM, "SubscribedContent-353696Enabled", 0),
            hkcu(CDM, "SubscribedContent-353698Enabled", 0),
        ],
    ),
    pub_(
        "pubs-explorateur",
        "Publicités dans l'Explorateur de fichiers",
        "Retire les bandeaux pour OneDrive et Microsoft 365 dans l'Explorateur.",
        &[hkcu(AVANCE, "ShowSyncProviderNotifications", 0)],
    ),
    pub_(
        "bing-menu-demarrer",
        "Recherche Bing dans le menu Démarrer",
        "La recherche du menu Démarrer ne cherche plus que sur le PC, sans résultats web.",
        &[
            hkcu(r"Software\Microsoft\Windows\CurrentVersion\Search", "BingSearchEnabled", 0),
            hkcu(r"Software\Microsoft\Windows\CurrentVersion\Search", "CortanaConsent", 0),
            hkcu(r"Software\Policies\Microsoft\Windows\Explorer", "DisableSearchBoxSuggestions", 1),
        ],
    ),
    pub_(
        "actualites-barre",
        "« Actualités et champs d'intérêt » dans la barre des tâches",
        "Retire le bouton météo et actualités à côté de l'horloge.",
        &[hklm(r"SOFTWARE\Policies\Microsoft\Windows\Windows Feeds", "EnableFeeds", 0)],
    ),
    pub_(
        "reunion-maintenant",
        "Icône « Démarrer une réunion »",
        "Retire l'icône Skype « Réunion maintenant » à côté de l'horloge.",
        &[hklm(
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer",
            "HideSCAMeetNow",
            1,
        )],
    ),
    // ----- Télémétrie -----
    telemetrie(
        "service-telemetrie",
        "Services de télémétrie",
        "Désactive les deux services qui envoient les données de diagnostic à Microsoft.",
        SansRisque,
        &[Service("DiagTrack"), Service("dmwappushservice")],
    ),
    telemetrie(
        "niveau-telemetrie",
        "Données de diagnostic au minimum",
        "Demande à Windows d'envoyer le moins de données possible. Les éditions Famille et Pro gardent un minimum obligatoire.",
        SansRisque,
        &[
            hklm(COLLECTE, "AllowTelemetry", 0),
            hklm(
                r"SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\DataCollection",
                "AllowTelemetry",
                0,
            ),
        ],
    ),
    telemetrie(
        "taches-collecte",
        "Tâches planifiées de collecte",
        "Désactive les tâches qui analysent le PC pour le programme d'amélioration de l'expérience utilisateur.",
        SansRisque,
        &[
            Tache(r"\Microsoft\Windows\Application Experience\Microsoft Compatibility Appraiser"),
            Tache(r"\Microsoft\Windows\Application Experience\ProgramDataUpdater"),
            Tache(r"\Microsoft\Windows\Autochk\Proxy"),
            Tache(r"\Microsoft\Windows\Customer Experience Improvement Program\Consolidator"),
            Tache(r"\Microsoft\Windows\Customer Experience Improvement Program\UsbCeip"),
            Tache(r"\Microsoft\Windows\DiskDiagnostic\Microsoft-Windows-DiskDiagnosticDataCollector"),
            Tache(r"\Microsoft\Windows\Feedback\Siuf\DmClient"),
            Tache(r"\Microsoft\Windows\Feedback\Siuf\DmClientOnScenarioDownload"),
        ],
    ),
    telemetrie(
        "identifiant-pub",
        "Identifiant de publicité",
        "Les applis ne peuvent plus utiliser votre identifiant pour cibler les publicités.",
        SansRisque,
        &[
            hkcu(r"Software\Microsoft\Windows\CurrentVersion\AdvertisingInfo", "Enabled", 0),
            hklm(r"SOFTWARE\Policies\Microsoft\Windows\AdvertisingInfo", "DisabledByGroupPolicy", 1),
        ],
    ),
    telemetrie(
        "historique-activite",
        "Historique d'activité",
        "Windows n'enregistre plus la liste des applis et fichiers ouverts, et ne l'envoie plus à Microsoft.",
        SansRisque,
        &[
            hklm(ACTIVITE, "EnableActivityFeed", 0),
            hklm(ACTIVITE, "PublishUserActivities", 0),
            hklm(ACTIVITE, "UploadUserActivities", 0),
        ],
    ),
    telemetrie(
        "demandes-avis",
        "Demandes d'avis",
        "Windows ne demande plus votre avis par des notifications.",
        SansRisque,
        &[
            hkcu(r"Software\Microsoft\Siuf\Rules", "NumberOfSIUFInPeriod", 0),
            hklm(COLLECTE, "DoNotShowFeedbackNotifications", 1),
        ],
    ),
    telemetrie(
        "experiences-personnalisees",
        "Expériences personnalisées",
        "Microsoft n'utilise plus vos données de diagnostic pour personnaliser conseils et publicités.",
        SansRisque,
        &[hkcu(
            r"Software\Microsoft\Windows\CurrentVersion\Privacy",
            "TailoredExperiencesWithDiagnosticDataEnabled",
            0,
        )],
    ),
    telemetrie(
        "saisie",
        "Collecte de la saisie et de l'écriture",
        "Windows n'envoie plus ce que vous tapez ou écrivez au stylet pour « améliorer la saisie ».",
        SansRisque,
        &[
            hkcu(SAISIE, "RestrictImplicitInkCollection", 1),
            hkcu(SAISIE, "RestrictImplicitTextCollection", 1),
            hkcu(r"Software\Microsoft\InputPersonalization\TrainedDataStore", "HarvestContacts", 0),
            hkcu(r"Software\Microsoft\Personalization\Settings", "AcceptedPrivacyPolicy", 0),
            hkcu(r"Software\Microsoft\Input\TIPC", "Enabled", 0),
        ],
    ),
    telemetrie(
        "langues-sites",
        "Liste des langues envoyée aux sites web",
        "Les sites ne reçoivent plus la liste des langues installées sur le PC.",
        SansRisque,
        &[hkcu(r"Control Panel\International\User Profile", "HttpAcceptLanguageOptOut", 1)],
    ),
    telemetrie(
        "cortana-recherche",
        "Cortana dans la recherche",
        "Désactive Cortana pour tous les comptes du PC. Les commandes vocales « Hey Cortana » ne répondent plus.",
        Attention,
        &[hklm(r"SOFTWARE\Policies\Microsoft\Windows\Windows Search", "AllowCortana", 0)],
    ),
    telemetrie(
        "rapports-erreurs",
        "Rapports d'erreurs",
        "Windows n'envoie plus de rapport à Microsoft quand un programme plante. Vous ne recevrez plus de solutions proposées.",
        Attention,
        &[
            hklm(r"SOFTWARE\Microsoft\Windows\Windows Error Reporting", "Disabled", 1),
            Tache(r"\Microsoft\Windows\Windows Error Reporting\QueueReporting"),
        ],
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Noms réels d'applis dont dépend Windows ou sa sécurité.
    const VITALES: &[&str] = &[
        "Microsoft.WindowsStore",
        "Microsoft.StorePurchaseApp",
        "Microsoft.DesktopAppInstaller",
        "Microsoft.MicrosoftEdge",
        "Microsoft.MicrosoftEdge.Stable",
        "Microsoft.MicrosoftEdgeDevToolsClient",
        "Microsoft.Windows.ShellExperienceHost",
        "Microsoft.Windows.StartMenuExperienceHost",
        "Microsoft.Windows.Search",
        "Microsoft.Windows.SecHealthUI",
        "Microsoft.SecHealthUI",
        "Microsoft.VCLibs.140.00",
        "Microsoft.NET.Native.Framework.2.2",
        "Microsoft.UI.Xaml.2.8",
        "Microsoft.XboxGameCallableUI",
        "Microsoft.AAD.BrokerPlugin",
        "Microsoft.AccountsControl",
        "Microsoft.LockApp",
        "Microsoft.HEIFImageExtension",
        "Microsoft.VP9VideoExtensions",
        "Microsoft.WebMediaExtensions",
        "MicrosoftWindows.Client.CBS",
        "windows.immersivecontrolpanel",
        "NVIDIACorp.NVIDIAControlPanel",
        "RealtekSemiconductorCorp.RealtekAudioControl",
    ];

    fn motifs() -> Vec<&'static str> {
        catalogue()
            .iter()
            .flat_map(|e| e.actions.iter())
            .filter_map(|a| match a {
                Action::Paquet(m) => Some(*m),
                _ => None,
            })
            .collect()
    }

    fn element(id: &str) -> &'static Element {
        catalogue()
            .iter()
            .find(|e| e.id == id)
            .unwrap_or_else(|| panic!("élément « {id} » absent du catalogue"))
    }

    #[test]
    fn les_applis_vitales_sont_protegees() {
        for nom in VITALES {
            assert!(est_protege(nom), "{nom} devrait être protégée");
        }
    }

    #[test]
    fn les_applis_ordinaires_ne_sont_pas_protegees() {
        for nom in [
            "Microsoft.SkypeApp",
            "king.com.CandyCrushSaga",
            "Microsoft.Windows.Photos",
            "Microsoft.WindowsCalculator",
            "Microsoft.XboxApp",
            "Microsoft.549981C3F5F10",
        ] {
            assert!(!est_protege(nom), "{nom} ne devrait pas être protégée");
        }
    }

    #[test]
    fn aucun_motif_du_catalogue_ne_vise_une_appli_vitale() {
        for motif in motifs() {
            for nom in VITALES {
                assert!(!correspond(motif, nom), "le motif {motif} attrape {nom}");
            }
        }
    }

    #[test]
    fn les_identifiants_sont_uniques() {
        let mut vus = HashSet::new();
        for e in catalogue() {
            assert!(vus.insert(e.id), "identifiant en double : {}", e.id);
        }
    }

    #[test]
    fn chaque_element_a_un_nom_une_explication_et_une_action() {
        assert!(!catalogue().is_empty());
        for e in catalogue() {
            assert!(!e.nom.trim().is_empty(), "{} : nom vide", e.id);
            assert!(e.explication.trim().len() > 15, "{} : explication trop courte", e.id);
            assert!(!e.actions.is_empty(), "{} : aucune action", e.id);
        }
    }

    #[test]
    fn les_trois_categories_sont_remplies() {
        for c in [Categorie::Applis, Categorie::Pubs, Categorie::Telemetrie] {
            assert!(catalogue().iter().any(|e| e.categorie == c), "{c:?} est vide");
        }
    }

    #[test]
    fn seule_la_categorie_applis_retire_des_applis() {
        for e in catalogue() {
            let retire = e
                .actions
                .iter()
                .any(|a| matches!(a, Action::Paquet(_) | Action::OneDrive));
            if retire {
                assert_eq!(e.categorie, Categorie::Applis, "{}", e.id);
            }
        }
    }

    #[test]
    fn les_cles_de_registre_sont_relatives_a_leur_ruche() {
        for e in catalogue() {
            for a in e.actions {
                if let Action::Registre { cle, nom, .. } = a {
                    assert!(!cle.starts_with('\\') && !cle.ends_with('\\'), "{}", e.id);
                    assert!(!cle.to_uppercase().starts_with("HK"), "{}", e.id);
                    assert!(!nom.is_empty(), "{}", e.id);
                }
            }
        }
    }

    #[test]
    fn les_chemins_de_taches_commencent_par_une_barre() {
        for e in catalogue() {
            for a in e.actions {
                if let Action::Tache(chemin) = a {
                    assert!(chemin.starts_with('\\'), "{} : {chemin}", e.id);
                }
            }
        }
    }

    #[test]
    fn les_jeux_promotionnels_sont_coches_d_office() {
        assert!(element("jeux-promo").coche_par_defaut());
        assert!(element("pubs-menu-demarrer").coche_par_defaut());
        assert!(element("service-telemetrie").coche_par_defaut());
    }

    #[test]
    fn ce_qui_peut_manquer_est_decoche() {
        for id in [
            "xbox", "onedrive", "cortana", "photos", "courrier", "meteo", "cartes", "calculatrice",
            "spotify",
        ] {
            assert!(!element(id).coche_par_defaut(), "{id} devrait être décoché");
        }
    }

    #[test]
    fn rien_ne_touche_aux_mises_a_jour_ni_a_la_securite() {
        for e in catalogue() {
            for a in e.actions {
                let texte = format!("{a:?}").to_lowercase();
                for interdit in ["wuauserv", "windowsupdate", "windefend", "windows defender", "usosvc", "securityhealth"] {
                    assert!(!texte.contains(interdit), "{} touche à {interdit}", e.id);
                }
            }
        }
    }
}
