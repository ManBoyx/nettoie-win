//! La vraie version de `Systeme` sous Linux : paquets, services systemd, fichiers de configuration.
//!
//! Lire ne demande aucun droit. Tout ce qui modifie le système passe par
//! l'assistant (voir `assistant`), lancé une seule fois avec `pkexec`.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use crate::assistant::{autorisee, Demande, Ordre, Reponse};
use crate::catalogue::Element;
use crate::fichier_conf::{lire_cle, poser_cle};
use crate::gestionnaire::{issue_retrait, verifier_retrait, Commande, Gestionnaire};
use crate::systeme::*;

/// Lit le type de démarrage dans la réponse de `systemctl is-enabled`.
/// `None` : le service n'existe pas, ou ne se règle pas (démarré par un autre).
pub fn demarrage_depuis_sortie(sortie: &str) -> Option<u32> {
    match sortie.split_whitespace().next()? {
        "enabled" | "enabled-runtime" => Some(DEMARRAGE_AUTO),
        "disabled" | "masked" => Some(DEMARRAGE_DESACTIVE),
        _ => None,
    }
}

/// Lit la valeur dans la réponse de `pro config show <nom>` (« nom valeur »).
pub fn valeur_pro_depuis_sortie(sortie: &str, nom: &str) -> Option<String> {
    sortie.lines().find_map(|ligne| {
        let mut mots = ligne.split_whitespace();
        (mots.next()? == nom).then(|| mots.next().map(str::to_string))?
    })
}

/// Nom de la distribution, lu dans le contenu de `/etc/os-release`.
pub fn nom_distribution(os_release: &str) -> Option<String> {
    let valeur = lire_cle(os_release, "PRETTY_NAME")?;
    let valeur = valeur.trim_matches('"').trim();
    (!valeur.is_empty()).then(|| valeur.to_string())
}

/// Identifiant effectif du compte, lu dans le contenu de `/proc/self/status`.
pub fn uid_effectif(statut: &str) -> Option<u32> {
    statut.lines().find_map(|ligne| ligne.strip_prefix("Uid:"))?.split_whitespace().nth(1)?.parse().ok()
}

pub fn est_root() -> bool {
    std::fs::read_to_string("/proc/self/status").ok().and_then(|s| uid_effectif(&s)) == Some(0)
}

/// Cherche un programme dans les dossiers habituels, y compris ceux réservés à l'administration.
fn chemin_de(programme: &str) -> Option<PathBuf> {
    let variable = std::env::var_os("PATH").unwrap_or_default();
    std::env::split_paths(&variable)
        .chain(["/usr/sbin", "/usr/bin", "/sbin", "/bin"].iter().map(PathBuf::from))
        .map(|dossier| dossier.join(programme))
        .find(|chemin| chemin.is_file())
}

fn existe(programme: &str) -> bool {
    chemin_de(programme).is_some()
}

/// Lance une commande et rend sa réussite et tout ce qu'elle a écrit.
fn lancer(commande: &Commande) -> Result<(bool, String), String> {
    let programme = chemin_de(commande.programme).ok_or_else(|| format!("{} introuvable", commande.programme))?;
    let sortie = Command::new(programme)
        .args(&commande.arguments)
        .envs(commande.environnement.iter().copied())
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("{} : {e}", commande.programme))?;
    let mut texte = String::from_utf8_lossy(&sortie.stdout).into_owned();
    texte.push_str(&String::from_utf8_lossy(&sortie.stderr));
    Ok((sortie.status.success(), texte))
}

fn outil(programme: &'static str, arguments: &[&str]) -> Result<(bool, String), String> {
    lancer(&Commande {
        programme,
        arguments: arguments.iter().map(|a| a.to_string()).collect(),
        environnement: vec![("LC_ALL", "C")],
    })
}

fn outil_exige(programme: &'static str, arguments: &[&str]) -> Result<(), String> {
    match outil(programme, arguments)? {
        (true, _) => Ok(()),
        (false, texte) => Err(texte.split_whitespace().collect::<Vec<_>>().join(" ")),
    }
}

fn gestionnaire() -> Option<Gestionnaire> {
    Gestionnaire::detecter(&existe)
}

/// Fait réellement une opération. Doit tourner en administrateur ; l'appelant
/// a déjà vérifié que la demande est autorisée.
pub fn executer_en_root(demande: &Demande) -> Result<(), String> {
    match demande {
        Demande::RetirerPaquet { nom } => {
            let gestionnaire = gestionnaire().ok_or("gestionnaire de paquets non reconnu")?;
            // Un paquet protégé est refusé avant même d'interroger le gestionnaire.
            verifier_retrait(nom, &[])?;
            // On regarde ensuite ce que le retrait emporterait, sans rien changer.
            let (reussie, sortie) = lancer(&gestionnaire.simulation(nom))?;
            verifier_retrait(nom, &gestionnaire.emportes(nom, reussie, &sortie)?)?;
            let (reussie, texte) = lancer(&gestionnaire.retrait(nom))?;
            let encore_installe = match lancer(&gestionnaire.liste()) {
                Ok((true, liste)) => gestionnaire.paquets(&liste).iter().any(|p| p == nom),
                // Sans liste lisible, on s'en tient à ce que le gestionnaire a répondu.
                _ => !reussie,
            };
            issue_retrait(reussie, encore_installe, &texte)
        }
        Demande::Service { nom, ordre } => {
            let verbe = match ordre {
                Ordre::Activer => "enable",
                Ordre::Desactiver => "disable",
                Ordre::Demarrer => "start",
                Ordre::Arreter => "stop",
            };
            outil_exige("systemctl", &[verbe, nom])
        }
        Demande::Fichier { chemin, nom, valeur } => {
            let chemin = Path::new(chemin);
            let ancien = std::fs::read_to_string(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;
            let nouveau = poser_cle(&ancien, nom, valeur.as_deref());
            // Écriture à côté puis remplacement, avec les mêmes droits que l'original.
            let provisoire = chemin.with_extension("nettoie-linux.tmp");
            let erreur = |e: std::io::Error| format!("{} : {e}", chemin.display());
            std::fs::write(&provisoire, nouveau).map_err(erreur)?;
            let droits = std::fs::metadata(chemin).map_err(erreur)?.permissions();
            std::fs::set_permissions(&provisoire, droits).map_err(erreur)?;
            std::fs::rename(&provisoire, chemin).map_err(erreur)
        }
        Demande::Pro { nom, valeur } => outil_exige("pro", &["config", "set", &format!("{nom}={valeur}")]),
    }
}

/// La liaison avec l'assistant lancé en administrateur.
struct Canal {
    enfant: Child,
    entree: Option<ChildStdin>,
    sortie: BufReader<ChildStdout>,
}

impl Canal {
    fn ouvrir() -> Result<Canal, String> {
        let pkexec = chemin_de("pkexec")
            .ok_or("pkexec est introuvable : relancez le programme avec sudo pour modifier le système")?;
        let moi = std::env::current_exe().map_err(|e| e.to_string())?;
        let mut enfant = Command::new(pkexec)
            .arg(moi)
            .arg("--assistant")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("pkexec : {e}"))?;
        let entree = enfant.stdin.take();
        let sortie = BufReader::new(enfant.stdout.take().ok_or("assistant sans sortie")?);
        Ok(Canal { enfant, entree, sortie })
    }

    fn echanger(&mut self, demande: &Demande) -> Result<Reponse, String> {
        const REFUS: &str = "droits d'administrateur refusés (ou assistant arrêté)";
        let entree = self.entree.as_mut().ok_or(REFUS)?;
        let ligne = serde_json::to_string(demande).map_err(|e| e.to_string())?;
        writeln!(entree, "{ligne}").and_then(|()| entree.flush()).map_err(|_| REFUS.to_string())?;
        let mut reponse = String::new();
        match self.sortie.read_line(&mut reponse) {
            Ok(n) if n > 0 => serde_json::from_str(&reponse).map_err(|e| format!("réponse illisible de l'assistant : {e}")),
            _ => Err(REFUS.to_string()),
        }
    }
}

impl Drop for Canal {
    fn drop(&mut self) {
        // Fermer son entrée suffit à arrêter l'assistant.
        self.entree = None;
        let _ = self.enfant.wait();
    }
}

enum Acces {
    /// Le programme tourne déjà en administrateur (lancé avec sudo).
    Direct,
    /// L'assistant est lancé à la première modification, pas avant.
    Assistant(Option<Canal>),
}

pub struct SystemeLinux {
    catalogue: &'static [Element],
    gestionnaire: Option<Gestionnaire>,
    acces: Acces,
}

impl SystemeLinux {
    pub fn nouveau(catalogue: &'static [Element]) -> Self {
        let acces = if est_root() { Acces::Direct } else { Acces::Assistant(None) };
        SystemeLinux { catalogue, gestionnaire: gestionnaire(), acces }
    }

    fn demander(&mut self, demande: Demande) -> Result<(), String> {
        if !autorisee(&demande, self.catalogue) {
            return Err("opération non prévue par le programme".to_string());
        }
        match &mut self.acces {
            Acces::Direct => executer_en_root(&demande),
            Acces::Assistant(canal) => {
                if canal.is_none() {
                    *canal = Some(Canal::ouvrir()?);
                }
                let reponse = canal.as_mut().map(|c| c.echanger(&demande));
                match reponse {
                    Some(Ok(Reponse::Fait)) => Ok(()),
                    Some(Ok(Reponse::Erreur(e))) => Err(e),
                    Some(Err(e)) => {
                        // La prochaine demande relancera l'assistant (et redemandera le mot de passe).
                        *canal = None;
                        Err(e)
                    }
                    None => Err("assistant indisponible".to_string()),
                }
            }
        }
    }

    fn service(&mut self, nom: &str, ordre: Ordre) -> Result<(), String> {
        self.demander(Demande::Service { nom: nom.to_string(), ordre })
    }
}

fn texte_de(valeur: &ValeurBrute) -> Result<String, String> {
    valeur.en_texte().map(str::to_string).ok_or_else(|| "valeur inattendue pour un réglage Linux".to_string())
}

const SANS_OBJET: &str = "réglage sans objet sous Linux";

impl Systeme for SystemeLinux {
    fn description(&self) -> Option<String> {
        nom_distribution(&std::fs::read_to_string("/etc/os-release").ok()?)
    }

    fn reglage_applicable(&self, ruche: Ruche, cle: &str, nom: &str) -> bool {
        match ruche {
            Ruche::Fichier => Path::new(cle).is_file(),
            // En administrateur, gsettings réglerait le compte root, pas le vôtre.
            Ruche::Gsettings => !est_root() && matches!(outil("gsettings", &["get", cle, nom]), Ok((true, _))),
            Ruche::Pro => matches!(outil("pro", &["config", "show", nom]), Ok((true, _))),
            Ruche::Utilisateur | Ruche::Machine => false,
        }
    }

    fn lire_valeur(&self, ruche: Ruche, cle: &str, nom: &str) -> Result<Option<ValeurBrute>, String> {
        let texte = match ruche {
            Ruche::Fichier => match std::fs::read_to_string(cle) {
                Ok(contenu) => lire_cle(&contenu, nom),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(format!("{cle} : {e}")),
            },
            Ruche::Gsettings => match outil("gsettings", &["get", cle, nom])? {
                (true, sortie) => Some(sortie.trim().to_string()),
                (false, _) => None,
            },
            Ruche::Pro => match outil("pro", &["config", "show", nom])? {
                (true, sortie) => valeur_pro_depuis_sortie(&sortie, nom),
                (false, _) => None,
            },
            Ruche::Utilisateur | Ruche::Machine => return Err(SANS_OBJET.to_string()),
        };
        Ok(texte.map(|t| ValeurBrute::texte(&t)))
    }

    fn ecrire_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str, valeur: &ValeurBrute) -> Result<(), String> {
        let valeur = texte_de(valeur)?;
        match ruche {
            Ruche::Fichier => {
                self.demander(Demande::Fichier { chemin: cle.to_string(), nom: nom.to_string(), valeur: Some(valeur) })
            }
            Ruche::Gsettings => outil_exige("gsettings", &["set", cle, nom, &valeur]),
            Ruche::Pro => self.demander(Demande::Pro { nom: nom.to_string(), valeur }),
            Ruche::Utilisateur | Ruche::Machine => Err(SANS_OBJET.to_string()),
        }
    }

    fn supprimer_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str) -> Result<(), String> {
        match ruche {
            Ruche::Fichier => self.demander(Demande::Fichier { chemin: cle.to_string(), nom: nom.to_string(), valeur: None }),
            Ruche::Gsettings => outil_exige("gsettings", &["reset", cle, nom]),
            _ => Err(SANS_OBJET.to_string()),
        }
    }

    fn demarrage_service(&self, nom: &str) -> Result<Option<u32>, String> {
        if !existe("systemctl") {
            return Ok(None);
        }
        Ok(demarrage_depuis_sortie(&outil("systemctl", &["is-enabled", nom])?.1))
    }

    fn regler_service(&mut self, nom: &str, demarrage: u32) -> Result<(), String> {
        match demarrage {
            DEMARRAGE_AUTO => self.service(nom, Ordre::Activer),
            DEMARRAGE_DESACTIVE => self.service(nom, Ordre::Desactiver),
            autre => Err(format!("type de démarrage inattendu ({autre})")),
        }
    }

    fn arreter_service(&mut self, nom: &str) -> Result<(), String> {
        self.service(nom, Ordre::Arreter)
    }

    fn demarrer_service(&mut self, nom: &str) -> Result<(), String> {
        self.service(nom, Ordre::Demarrer)
    }

    fn paquets(&self) -> Result<Vec<String>, String> {
        let gestionnaire = self.gestionnaire.ok_or("gestionnaire de paquets non reconnu (ni apt, ni rpm, ni pacman)")?;
        match lancer(&gestionnaire.liste())? {
            (true, sortie) => Ok(gestionnaire.paquets(&sortie)),
            (false, sortie) => Err(sortie.trim().to_string()),
        }
    }

    fn retirer_paquet(&mut self, nom: &str) -> Result<(), String> {
        self.demander(Demande::RetirerPaquet { nom: nom.to_string() })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_service_active_est_automatique() {
        assert_eq!(demarrage_depuis_sortie("enabled\n"), Some(DEMARRAGE_AUTO));
        assert_eq!(demarrage_depuis_sortie("enabled-runtime\n"), Some(DEMARRAGE_AUTO));
    }

    #[test]
    fn un_service_desactive_ou_masque_est_desactive() {
        assert_eq!(demarrage_depuis_sortie("disabled\n"), Some(DEMARRAGE_DESACTIVE));
        assert_eq!(demarrage_depuis_sortie("masked\n"), Some(DEMARRAGE_DESACTIVE));
    }

    #[test]
    fn un_service_absent_ou_non_reglable_n_est_pas_propose() {
        for sortie in ["static\n", "indirect\n", "alias\n", "generated\n", "", "Failed to get unit file state for x.service: No such file or directory\n", "not-found\n"] {
            assert_eq!(demarrage_depuis_sortie(sortie), None, "{sortie:?}");
        }
    }

    #[test]
    fn la_valeur_d_ubuntu_pro_est_lue_apres_son_nom() {
        assert_eq!(valeur_pro_depuis_sortie("apt_news True\n", "apt_news"), Some("True".to_string()));
        assert_eq!(valeur_pro_depuis_sortie("apt_news  False\n", "apt_news"), Some("False".to_string()));
        assert_eq!(valeur_pro_depuis_sortie("autre True\n", "apt_news"), None);
        assert_eq!(valeur_pro_depuis_sortie("", "apt_news"), None);
    }

    #[test]
    fn le_nom_de_la_distribution_est_lu_sans_ses_guillemets() {
        let contenu = "NAME=\"Ubuntu\"\nPRETTY_NAME=\"Ubuntu 24.04.1 LTS\"\nID=ubuntu\n";
        assert_eq!(nom_distribution(contenu), Some("Ubuntu 24.04.1 LTS".to_string()));
        assert_eq!(nom_distribution("PRETTY_NAME=Arch Linux\n"), Some("Arch Linux".to_string()));
        assert_eq!(nom_distribution("ID=x\n"), None);
    }

    #[test]
    fn le_compte_effectif_est_le_deuxieme_nombre_de_la_ligne_uid() {
        assert_eq!(uid_effectif("Name:\tx\nUid:\t1000\t1000\t1000\t1000\nGid:\t1000\n"), Some(1000));
        assert_eq!(uid_effectif("Uid:\t1000\t0\t0\t0\n"), Some(0));
        assert_eq!(uid_effectif("Name:\tx\n"), None);
    }

    /// Lecture seule sur la machine qui lance les tests : ne modifie rien.
    #[test]
    fn sur_cette_machine_la_liste_des_paquets_se_lit_sans_droits() {
        let sys = SystemeLinux::nouveau(crate::catalogue_linux::catalogue());
        if sys.gestionnaire.is_none() {
            return;
        }
        let paquets = sys.paquets().expect("liste des paquets");
        assert!(paquets.len() > 50, "{} paquets seulement", paquets.len());
        assert!(sys.description().is_some());
        assert_eq!(sys.demarrage_service("service-qui-n-existe-pas.service").unwrap(), None);
        assert_eq!(sys.lire_valeur(Ruche::Fichier, "/etc/fichier-qui-n-existe-pas", "A").unwrap(), None);
        assert!(!sys.reglage_applicable(Ruche::Fichier, "/etc/fichier-qui-n-existe-pas", "A"));
    }

    #[test]
    fn une_operation_hors_catalogue_n_atteint_jamais_l_assistant() {
        let mut sys = SystemeLinux::nouveau(crate::catalogue_linux::catalogue());
        assert!(sys.retirer_paquet("systemd").is_err());
        assert!(sys.regler_service("ssh.service", DEMARRAGE_DESACTIVE).is_err());
        assert!(matches!(sys.acces, Acces::Assistant(None) | Acces::Direct), "l'assistant ne doit pas avoir été lancé");
    }
}
