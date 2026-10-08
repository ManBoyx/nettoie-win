//! Lecture et modification d'une ligne `NOM=valeur` dans un fichier de configuration Linux.

/// Valeur de la dernière ligne `nom=…` qui n'est pas un commentaire.
pub fn lire_cle(contenu: &str, nom: &str) -> Option<String> {
    contenu.lines().filter_map(|ligne| valeur_de(ligne, nom)).last().map(str::to_string)
}

/// Rend le contenu avec `nom=valeur` (ligne remplacée, ou ajoutée à la fin),
/// ou sans aucune ligne `nom=…` si `valeur` est `None`.
pub fn poser_cle(contenu: &str, nom: &str, valeur: Option<&str>) -> String {
    let lignes: Vec<&str> = contenu.lines().collect();
    let derniere = lignes.iter().rposition(|ligne| valeur_de(ligne, nom).is_some());
    let nouvelle = valeur.map(|v| format!("{nom}={v}"));
    let mut sortie = String::new();
    for (rang, ligne) in lignes.iter().enumerate() {
        if Some(rang) == derniere {
            if let Some(nouvelle) = &nouvelle {
                sortie.push_str(nouvelle);
                sortie.push('\n');
            }
        } else if valeur.is_some() || valeur_de(ligne, nom).is_none() {
            sortie.push_str(ligne);
            sortie.push('\n');
        }
    }
    if derniere.is_none() {
        if let Some(nouvelle) = &nouvelle {
            sortie.push_str(nouvelle);
            sortie.push('\n');
        }
    }
    sortie
}

/// Si `ligne` est une définition de `nom`, rend sa valeur.
fn valeur_de<'a>(ligne: &'a str, nom: &str) -> Option<&'a str> {
    let reste = ligne.trim_start().strip_prefix(nom)?;
    Some(reste.trim_start().strip_prefix('=')?.trim())
}

#[cfg(test)]
mod tests {
    use super::*;

    const APPORT: &str = "# set this to 0 to disable apport, or to 1 to enable it\n# enabled=0 dans un commentaire\nenabled=1\n";

    #[test]
    fn la_valeur_est_lue_en_ignorant_les_commentaires() {
        assert_eq!(lire_cle(APPORT, "enabled"), Some("1".to_string()));
    }

    #[test]
    fn les_guillemets_et_les_espaces_autour_du_signe_egal_sont_acceptes() {
        assert_eq!(lire_cle("PARTICIPATE=\"yes\"\n", "PARTICIPATE"), Some("\"yes\"".to_string()));
        assert_eq!(lire_cle("  countme = false  \n", "countme"), Some("false".to_string()));
    }

    #[test]
    fn une_cle_absente_ou_seulement_ressemblante_n_est_pas_lue() {
        assert_eq!(lire_cle(APPORT, "autre"), None);
        assert_eq!(lire_cle("enabled_extra=1\n", "enabled"), None);
        assert_eq!(lire_cle("", "enabled"), None);
    }

    #[test]
    fn la_derniere_definition_l_emporte() {
        assert_eq!(lire_cle("A=1\nA=2\n", "A"), Some("2".to_string()));
    }

    #[test]
    fn poser_remplace_la_ligne_sans_toucher_au_reste() {
        assert_eq!(
            poser_cle(APPORT, "enabled", Some("0")),
            "# set this to 0 to disable apport, or to 1 to enable it\n# enabled=0 dans un commentaire\nenabled=0\n"
        );
    }

    #[test]
    fn poser_ajoute_la_ligne_si_elle_manque() {
        assert_eq!(poser_cle("A=1\n", "B", Some("2")), "A=1\nB=2\n");
        assert_eq!(poser_cle("A=1", "B", Some("2")), "A=1\nB=2\n");
        assert_eq!(poser_cle("", "B", Some("2")), "B=2\n");
    }

    #[test]
    fn poser_puis_lire_rend_la_valeur() {
        let contenu = poser_cle(APPORT, "enabled", Some("0"));
        assert_eq!(lire_cle(&contenu, "enabled"), Some("0".to_string()));
    }

    #[test]
    fn retirer_enleve_la_ligne_et_garde_les_commentaires() {
        assert_eq!(
            poser_cle(APPORT, "enabled", None),
            "# set this to 0 to disable apport, or to 1 to enable it\n# enabled=0 dans un commentaire\n"
        );
        assert_eq!(poser_cle("A=1\n", "B", None), "A=1\n");
    }

    #[test]
    fn remettre_l_ancienne_valeur_redonne_le_fichier_d_origine() {
        let modifie = poser_cle(APPORT, "enabled", Some("0"));
        assert_eq!(poser_cle(&modifie, "enabled", Some("1")), APPORT);
    }
}
