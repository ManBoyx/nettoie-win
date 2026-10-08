//! Ce que le programme a besoin de demander à Windows.
//!
//! Tout le reste du code passe par ce trait : la vraie version parle à
//! Windows, la fausse (`faux`) sert aux tests et à la démonstration.

use serde::{Deserialize, Serialize};

pub use crate::catalogue::{Ruche, Valeur};

/// Type « nombre 32 bits » du registre.
pub const REG_DWORD: u32 = 4;

/// Types de démarrage d'un service, tels que Windows les note.
pub const DEMARRAGE_AUTO: u32 = 2;
pub const DEMARRAGE_MANUEL: u32 = 3;
pub const DEMARRAGE_DESACTIVE: u32 = 4;

/// Une valeur du registre telle quelle, pour pouvoir la remettre à l'identique.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValeurBrute {
    pub genre: u32,
    pub octets: Vec<u8>,
}

/// Type « texte » des réglages Linux (hors des types du registre de Windows).
pub const GENRE_TEXTE: u32 = 0x1000;

impl From<Valeur> for ValeurBrute {
    fn from(valeur: Valeur) -> Self {
        match valeur {
            Valeur::Nombre(n) => ValeurBrute::dword(n),
            Valeur::Texte(t) => ValeurBrute::texte(t),
        }
    }
}

impl ValeurBrute {
    pub fn texte(texte: &str) -> Self {
        ValeurBrute { genre: GENRE_TEXTE, octets: texte.as_bytes().to_vec() }
    }

    pub fn en_texte(&self) -> Option<&str> {
        if self.genre != GENRE_TEXTE {
            return None;
        }
        std::str::from_utf8(&self.octets).ok()
    }

    /// La valeur telle qu'on l'affiche.
    pub fn lisible(&self) -> String {
        match (self.en_dword(), self.en_texte()) {
            (Some(n), _) => n.to_string(),
            (_, Some(t)) => t.to_string(),
            _ => "(valeur binaire)".to_string(),
        }
    }

    pub fn dword(n: u32) -> Self {
        ValeurBrute { genre: REG_DWORD, octets: n.to_le_bytes().to_vec() }
    }

    pub fn en_dword(&self) -> Option<u32> {
        if self.genre != REG_DWORD {
            return None;
        }
        let octets: [u8; 4] = self.octets.as_slice().try_into().ok()?;
        Some(u32::from_le_bytes(octets))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointRestauration {
    Cree,
    /// Windows n'en crée qu'un par jour : celui du jour sert de filet.
    DejaRecent,
}

const SANS_OBJET: &str = "sans objet sur ce système";

/// « Windows 11 (version 22631) », à partir du numéro de version.
pub fn nom_windows(build: u32) -> String {
    let generation = if build >= crate::catalogue::PREMIER_WINDOWS_11 { 11 } else { 10 };
    format!("Windows {generation} (version {build})")
}

pub trait Systeme {
    /// Le système tel qu'on le nomme à l'utilisateur : « Windows 11 (version 22631) », « Ubuntu 24.04 LTS »…
    fn description(&self) -> Option<String> {
        self.build_windows().map(nom_windows)
    }

    /// Numéro de version (« build ») de Windows, ou `None` hors de Windows.
    fn build_windows(&self) -> Option<u32> {
        None
    }

    /// Faux si le réglage n'existe pas sur ce PC (fichier ou programme absent) :
    /// il n'y a alors rien à proposer.
    fn reglage_applicable(&self, _ruche: Ruche, _cle: &str, _nom: &str) -> bool {
        true
    }

    fn lire_valeur(&self, ruche: Ruche, cle: &str, nom: &str) -> Result<Option<ValeurBrute>, String>;
    fn ecrire_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str, valeur: &ValeurBrute) -> Result<(), String>;
    fn supprimer_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str) -> Result<(), String>;

    /// Type de démarrage du service, ou `None` s'il n'existe pas.
    fn demarrage_service(&self, nom: &str) -> Result<Option<u32>, String>;
    fn regler_service(&mut self, nom: &str, demarrage: u32) -> Result<(), String>;
    fn arreter_service(&mut self, nom: &str) -> Result<(), String>;
    fn demarrer_service(&mut self, nom: &str) -> Result<(), String>;

    /// Tâche planifiée de Windows ; `None` si elle n'existe pas.
    fn tache_active(&self, _chemin: &str) -> Result<Option<bool>, String> {
        Ok(None)
    }
    fn activer_tache(&mut self, _chemin: &str, _active: bool) -> Result<(), String> {
        Err(SANS_OBJET.to_string())
    }

    /// Noms des applis installées, pour un compte ou pour les futurs comptes.
    fn paquets(&self) -> Result<Vec<String>, String>;
    fn retirer_paquet(&mut self, nom: &str) -> Result<(), String>;

    fn onedrive_present(&self) -> Result<bool, String> {
        Ok(false)
    }
    fn desinstaller_onedrive(&mut self) -> Result<(), String> {
        Err(SANS_OBJET.to_string())
    }

    fn creer_point_restauration(&mut self) -> Result<PointRestauration, String> {
        Err(SANS_OBJET.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_dword_se_relit_a_l_identique() {
        assert_eq!(ValeurBrute::dword(0x1234_5678).en_dword(), Some(0x1234_5678));
    }

    #[test]
    fn un_texte_se_relit_a_l_identique_et_n_est_pas_un_nombre() {
        let valeur = ValeurBrute::texte("\"no\"");
        assert_eq!(valeur.en_texte(), Some("\"no\""));
        assert_eq!(valeur.en_dword(), None);
        assert_eq!(ValeurBrute::dword(1).en_texte(), None);
    }

    #[test]
    fn les_valeurs_du_catalogue_se_convertissent() {
        assert_eq!(ValeurBrute::from(Valeur::Nombre(3)), ValeurBrute::dword(3));
        assert_eq!(ValeurBrute::from(Valeur::Texte("0")).lisible(), "0");
        assert_eq!(ValeurBrute::dword(7).lisible(), "7");
    }

    #[test]
    fn une_valeur_d_un_autre_type_n_est_pas_un_dword() {
        let texte = ValeurBrute { genre: 1, octets: vec![0, 0, 0, 0] };
        assert_eq!(texte.en_dword(), None);
    }
}
