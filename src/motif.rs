//! Comparaison d'un nom d'appli à un motif.

/// Dit si `nom` correspond à `motif`. Dans le motif, `*` remplace n'importe
/// quelle suite de caractères. La casse est ignorée.
pub fn correspond(motif: &str, nom: &str) -> bool {
    let motif = motif.to_lowercase();
    let nom = nom.to_lowercase();
    let morceaux: Vec<&str> = motif.split('*').collect();
    if morceaux.len() == 1 {
        return motif == nom;
    }
    let dernier = morceaux.len() - 1;
    let mut reste = nom.as_str();
    for (i, morceau) in morceaux.iter().enumerate() {
        if i == 0 {
            match reste.strip_prefix(morceau) {
                Some(suite) => reste = suite,
                None => return false,
            }
        } else if i == dernier {
            return reste.ends_with(morceau);
        } else {
            match reste.find(morceau) {
                Some(position) => reste = &reste[position + morceau.len()..],
                None => return false,
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::correspond;

    #[test]
    fn un_nom_identique_correspond() {
        assert!(correspond("Microsoft.SkypeApp", "Microsoft.SkypeApp"));
    }

    #[test]
    fn la_casse_est_ignoree() {
        assert!(correspond("microsoft.skypeapp", "Microsoft.SkypeApp"));
    }

    #[test]
    fn sans_etoile_un_nom_plus_long_ne_correspond_pas() {
        assert!(!correspond("Microsoft.Xbox", "Microsoft.XboxApp"));
    }

    #[test]
    fn l_etoile_finale_accepte_n_importe_quelle_suite() {
        assert!(correspond("king.com.*", "king.com.CandyCrushSaga"));
        assert!(!correspond("king.com.*", "Microsoft.SkypeApp"));
    }

    #[test]
    fn l_etoile_fonctionne_au_debut_et_au_milieu() {
        assert!(correspond("*Netflix", "4DF9E0F8.Netflix"));
        assert!(correspond("*Disney*", "Disney.37853FC22B2CE"));
        assert!(correspond("A*B*C", "AxxBxxC"));
        assert!(!correspond("A*B*C", "AxxCxxB"));
    }

    #[test]
    fn l_etoile_peut_ne_rien_remplacer() {
        assert!(correspond("Microsoft.Bing*", "Microsoft.Bing"));
    }
}
