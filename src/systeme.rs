//! Ce que le programme a besoin de demander à Windows.
//!
//! Tout le reste du code passe par ce trait : la vraie version parle à
//! Windows, la fausse (`faux`) sert aux tests et à la démonstration.

use serde::{Deserialize, Serialize};

pub use crate::catalogue::Ruche;

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

impl ValeurBrute {
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

pub trait Systeme {
    fn lire_valeur(&self, ruche: Ruche, cle: &str, nom: &str) -> Result<Option<ValeurBrute>, String>;
    fn ecrire_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str, valeur: &ValeurBrute) -> Result<(), String>;
    fn supprimer_valeur(&mut self, ruche: Ruche, cle: &str, nom: &str) -> Result<(), String>;

    /// Type de démarrage du service, ou `None` s'il n'existe pas.
    fn demarrage_service(&self, nom: &str) -> Result<Option<u32>, String>;
    fn regler_service(&mut self, nom: &str, demarrage: u32) -> Result<(), String>;
    fn arreter_service(&mut self, nom: &str) -> Result<(), String>;
    fn demarrer_service(&mut self, nom: &str) -> Result<(), String>;

    /// `None` si la tâche n'existe pas.
    fn tache_active(&self, chemin: &str) -> Result<Option<bool>, String>;
    fn activer_tache(&mut self, chemin: &str, active: bool) -> Result<(), String>;

    /// Noms des applis installées, pour un compte ou pour les futurs comptes.
    fn paquets(&self) -> Result<Vec<String>, String>;
    fn retirer_paquet(&mut self, nom: &str) -> Result<(), String>;

    fn onedrive_present(&self) -> Result<bool, String>;
    fn desinstaller_onedrive(&mut self) -> Result<(), String>;

    fn creer_point_restauration(&mut self) -> Result<PointRestauration, String>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_dword_se_relit_a_l_identique() {
        assert_eq!(ValeurBrute::dword(0x1234_5678).en_dword(), Some(0x1234_5678));
    }

    #[test]
    fn une_valeur_d_un_autre_type_n_est_pas_un_dword() {
        let texte = ValeurBrute { genre: 1, octets: vec![0, 0, 0, 0] };
        assert_eq!(texte.en_dword(), None);
    }
}
