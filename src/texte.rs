//! Fabrication des commandes envoyées à Windows et lecture de leurs réponses.
//!
//! Rien ici ne touche au système : ce sont des fonctions sur du texte.

use crate::catalogue::est_protege;
use crate::systeme::PointRestauration;

/// Convertit la sortie d'un programme Windows, qui arrive en UTF-8 ou en UTF-16.
pub fn decoder_sortie(octets: &[u8]) -> String {
    let utf16 = |octets: &[u8]| {
        let unites: Vec<u16> = octets.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&unites)
    };
    if let Some(suite) = octets.strip_prefix(&[0xFF, 0xFE]) {
        return utf16(suite);
    }
    // En UTF-16, un texte occidental a un octet nul sur deux.
    let nuls = octets.iter().skip(1).step_by(2).filter(|o| **o == 0).count();
    if octets.len() >= 2 && nuls * 4 >= octets.len() {
        return utf16(octets);
    }
    let octets = octets.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(octets);
    String::from_utf8_lossy(octets).into_owned()
}

/// Lit le numéro de version de Windows tel que le registre le range (texte en UTF-16).
pub fn build_depuis_registre(octets: &[u8]) -> Option<u32> {
    let unites: Vec<u16> = octets.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&unites).trim_matches('\0').trim().parse().ok()
}

/// Lit la définition XML d'une tâche planifiée et dit si elle est active.
pub fn tache_active_depuis_xml(xml: &str) -> bool {
    // Seul le <Enabled> du bloc <Settings> concerne la tâche ; les déclencheurs ont le leur.
    let Some(debut) = xml.find("<Settings>") else {
        return true;
    };
    let reglages = &xml[debut..];
    let reglages = &reglages[..reglages.find("</Settings>").unwrap_or(reglages.len())];
    !reglages.to_lowercase().contains("<enabled>false</enabled>")
}

/// Un nom d'appli ne contient que des lettres, chiffres, points, tirets et soulignés.
pub fn nom_paquet_valide(nom: &str) -> bool {
    !nom.is_empty()
        && nom.len() <= 128
        && nom.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// Script PowerShell qui liste les applis installées.
pub const SCRIPT_LISTE: &str = concat!(
    "$ErrorActionPreference = 'Stop'; $ProgressPreference = 'SilentlyContinue'; ",
    "[Console]::OutputEncoding = [Text.Encoding]::UTF8; ",
    "try { ",
    "Get-AppxPackage -AllUsers | ForEach-Object { 'NW_P|' + $_.Name }; ",
    // Applis prévues pour les futurs comptes. Si cette liste-là échoue, celle
    // des applis installées suffit pour travailler.
    "try { Get-AppxProvisionedPackage -Online | ForEach-Object { 'NW_P|' + $_.DisplayName } } catch { }; ",
    "'NW_FIN' ",
    "} catch { 'NW_ERREUR|' + $_.Exception.Message }",
);

/// Script PowerShell qui crée un point de restauration.
///
/// Windows n'en crée qu'un par jour et ne dit rien quand il s'abstient : on
/// compare donc le numéro du dernier point avant et après.
pub const SCRIPT_POINT: &str = concat!(
    "$ErrorActionPreference = 'Stop'; $ProgressPreference = 'SilentlyContinue'; ",
    "[Console]::OutputEncoding = [Text.Encoding]::UTF8; ",
    "try { ",
    "$avant = (Get-ComputerRestorePoint | Measure-Object -Property SequenceNumber -Maximum).Maximum; ",
    "Checkpoint-Computer -Description 'Nettoie-Win' -RestorePointType 'MODIFY_SETTINGS' -WarningAction SilentlyContinue; ",
    "$apres = (Get-ComputerRestorePoint | Measure-Object -Property SequenceNumber -Maximum).Maximum; ",
    "if ($apres -gt $avant) { 'NW_CREE' } elseif ($apres) { 'NW_RECENT' } else { 'NW_ERREUR|aucun point de restauration' } ",
    "} catch { 'NW_ERREUR|' + $_.Exception.Message }",
);

/// Script PowerShell qui retire une appli pour tous les comptes, présents et futurs.
pub fn script_retrait(nom: &str) -> Result<String, String> {
    if !nom_paquet_valide(nom) {
        return Err(format!("nom d'appli invalide : {nom:?}"));
    }
    if est_protege(nom) {
        return Err(format!("{nom} est nécessaire à Windows"));
    }
    Ok(format!(
        concat!(
            "$ErrorActionPreference = 'Stop'; $ProgressPreference = 'SilentlyContinue'; ",
            "[Console]::OutputEncoding = [Text.Encoding]::UTF8; ",
            "try {{ ",
            "$nom = '{nom}'; ",
            // Les vieilles versions de Windows 10 ne connaissent pas -AllUsers pour le retrait.
            "Get-AppxPackage -AllUsers -Name $nom | ForEach-Object {{ ",
            "try {{ Remove-AppxPackage -Package $_.PackageFullName -AllUsers }} ",
            "catch {{ Remove-AppxPackage -Package $_.PackageFullName }} }}; ",
            // Sans cela, l'appli reviendrait à chaque nouveau compte.
            "Get-AppxProvisionedPackage -Online | Where-Object {{ $_.DisplayName -eq $nom }} | ",
            "ForEach-Object {{ Remove-AppxProvisionedPackage -Online -PackageName $_.PackageName | Out-Null }}; ",
            "'NW_OK' ",
            "}} catch {{ 'NW_ERREUR|' + $_.Exception.Message }}",
        ),
        nom = nom
    ))
}

pub fn paquets_depuis_sortie(sortie: &str) -> Result<Vec<String>, String> {
    if let Some(erreur) = erreur_signalee(sortie) {
        return Err(erreur);
    }
    if !contient(sortie, "NW_FIN") {
        return Err("la liste des applis est incomplète".to_string());
    }
    let mut noms: Vec<String> = Vec::new();
    for nom in sortie.lines().filter_map(|l| l.trim().strip_prefix("NW_P|")) {
        if !nom.is_empty() && !noms.iter().any(|n| n.eq_ignore_ascii_case(nom)) {
            noms.push(nom.to_string());
        }
    }
    Ok(noms)
}

fn erreur_signalee(sortie: &str) -> Option<String> {
    sortie
        .lines()
        .find_map(|l| l.trim().strip_prefix("NW_ERREUR|"))
        .map(|message| message.trim().to_string())
}

fn contient(sortie: &str, marque: &str) -> bool {
    sortie.lines().any(|l| l.trim() == marque)
}

const SANS_REPONSE: &str = "PowerShell n'a pas répondu";

pub fn retrait_depuis_sortie(sortie: &str) -> Result<(), String> {
    match erreur_signalee(sortie) {
        Some(erreur) => Err(erreur),
        None if contient(sortie, "NW_OK") => Ok(()),
        None => Err(SANS_REPONSE.to_string()),
    }
}

pub fn point_depuis_sortie(sortie: &str) -> Result<PointRestauration, String> {
    match erreur_signalee(sortie) {
        Some(erreur) => Err(erreur),
        None if contient(sortie, "NW_CREE") => Ok(PointRestauration::Cree),
        None if contient(sortie, "NW_RECENT") => Ok(PointRestauration::DejaRecent),
        None => Err(SANS_REPONSE.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn une_sortie_utf8_est_lue_telle_quelle() {
        assert_eq!(decoder_sortie("Tâche prête".as_bytes()), "Tâche prête");
    }

    #[test]
    fn une_sortie_utf16_avec_marque_est_reconnue() {
        let mut octets = vec![0xFF, 0xFE];
        octets.extend("<Task>é</Task>".encode_utf16().flat_map(|u| u.to_le_bytes()));
        assert_eq!(decoder_sortie(&octets), "<Task>é</Task>");
    }

    #[test]
    fn une_sortie_utf16_sans_marque_est_reconnue() {
        let octets: Vec<u8> = "<Task/>".encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
        assert_eq!(decoder_sortie(&octets), "<Task/>");
    }

    #[test]
    fn des_octets_invalides_ne_font_pas_echouer_la_lecture() {
        assert!(decoder_sortie(&[b'o', b'k', 0xE9]).starts_with("ok"));
    }

    fn utf16(texte: &str) -> Vec<u8> {
        texte.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()
    }

    #[test]
    fn le_numero_de_version_de_windows_est_lu_dans_le_registre() {
        assert_eq!(build_depuis_registre(&utf16("19045\0")), Some(19045));
        assert_eq!(build_depuis_registre(&utf16("22631")), Some(22631));
    }

    #[test]
    fn un_numero_de_version_illisible_est_ignore() {
        assert_eq!(build_depuis_registre(&utf16("inconnu\0")), None);
        assert_eq!(build_depuis_registre(&[]), None);
    }

    const TACHE: &str = "<Task><Triggers><TimeTrigger><Enabled>true</Enabled></TimeTrigger></Triggers>\
                         <Settings><Enabled>REGLAGE</Enabled><Hidden>false</Hidden></Settings></Task>";

    #[test]
    fn une_tache_marquee_active_est_active() {
        assert!(tache_active_depuis_xml(&TACHE.replace("REGLAGE", "true")));
    }

    #[test]
    fn une_tache_marquee_inactive_est_inactive_meme_si_son_declencheur_est_actif() {
        assert!(!tache_active_depuis_xml(&TACHE.replace("REGLAGE", "false")));
    }

    #[test]
    fn une_tache_sans_mention_est_active() {
        assert!(tache_active_depuis_xml("<Task><Settings><Hidden>true</Hidden></Settings></Task>"));
        assert!(tache_active_depuis_xml("<Task></Task>"));
    }

    #[test]
    fn un_declencheur_inactif_ne_rend_pas_la_tache_inactive() {
        let xml = "<Task><Triggers><TimeTrigger><Enabled>false</Enabled></TimeTrigger></Triggers>\
                   <Settings><Enabled>true</Enabled></Settings></Task>";
        assert!(tache_active_depuis_xml(xml));
    }

    #[test]
    fn les_vrais_noms_d_applis_sont_valides() {
        for nom in ["Microsoft.SkypeApp", "king.com.CandyCrushSaga", "Microsoft.549981C3F5F10", "A-b_c.1"] {
            assert!(nom_paquet_valide(nom), "{nom}");
        }
    }

    #[test]
    fn un_nom_qui_pourrait_detourner_la_commande_est_refuse() {
        for nom in ["", "a'; Remove-Item C:\\ -Recurse; '", "a b", "a\"b", "a$b", "a;b", "a*", "a|b", "a`b", "é"] {
            assert!(!nom_paquet_valide(nom), "{nom:?}");
        }
        assert!(!nom_paquet_valide(&"a".repeat(200)));
    }

    #[test]
    fn le_script_de_retrait_vise_le_nom_exact() {
        let script = script_retrait("Microsoft.SkypeApp").unwrap();
        assert!(script.contains("'Microsoft.SkypeApp'"));
        assert!(script.contains("Remove-AppxPackage"));
        assert!(script.contains("Remove-AppxProvisionedPackage"));
    }

    #[test]
    fn le_script_de_retrait_refuse_un_nom_invalide() {
        assert!(script_retrait("a'; calc; '").is_err());
    }

    #[test]
    fn le_script_de_retrait_refuse_une_appli_protegee() {
        assert!(script_retrait("Microsoft.WindowsStore").is_err());
    }

    #[test]
    fn les_scripts_tiennent_en_un_seul_argument_de_ligne_de_commande() {
        // Ni guillemet double ni retour à la ligne : ils casseraient le passage à PowerShell.
        for script in [SCRIPT_LISTE.to_string(), SCRIPT_POINT.to_string(), script_retrait("A.B").unwrap()] {
            assert!(!script.is_empty());
            assert!(!script.contains('"') && !script.contains('\n'), "{script}");
        }
    }

    #[test]
    fn la_liste_des_applis_est_lue_sans_doublon() {
        let sortie = "bruit\r\nNW_P|Microsoft.SkypeApp\r\nNW_P|king.com.CandyCrushSaga\r\nNW_P|microsoft.skypeapp\r\nNW_FIN\r\n";
        assert_eq!(
            paquets_depuis_sortie(sortie).unwrap(),
            vec!["Microsoft.SkypeApp".to_string(), "king.com.CandyCrushSaga".to_string()]
        );
    }

    #[test]
    fn une_liste_interrompue_est_une_erreur() {
        assert!(paquets_depuis_sortie("NW_P|Microsoft.SkypeApp\r\n").is_err());
    }

    #[test]
    fn l_erreur_de_powershell_est_transmise() {
        let erreur = paquets_depuis_sortie("NW_P|A.B\r\nNW_ERREUR|Accès refusé\r\n").unwrap_err();
        assert!(erreur.contains("Accès refusé"), "{erreur}");
    }

    #[test]
    fn un_retrait_reussi_est_reconnu() {
        assert_eq!(retrait_depuis_sortie("NW_OK\r\n"), Ok(()));
    }

    #[test]
    fn un_retrait_rate_donne_le_message_de_windows() {
        assert_eq!(retrait_depuis_sortie("NW_ERREUR|0x80073CFA\r\n"), Err("0x80073CFA".to_string()));
    }

    #[test]
    fn un_retrait_sans_reponse_est_une_erreur() {
        assert!(retrait_depuis_sortie("").is_err());
    }

    #[test]
    fn les_reponses_du_point_de_restauration_sont_reconnues() {
        assert_eq!(point_depuis_sortie("NW_CREE\r\n"), Ok(PointRestauration::Cree));
        assert_eq!(point_depuis_sortie("NW_RECENT\r\n"), Ok(PointRestauration::DejaRecent));
        assert_eq!(point_depuis_sortie("NW_ERREUR|désactivé\r\n"), Err("désactivé".to_string()));
        assert!(point_depuis_sortie("").is_err());
    }
}
