//! Essai de la couche Windows, à lancer sous Windows ou sous Wine.
//! N'écrit que dans une clé d'essai du compte courant, effacée à la fin.

#[cfg(windows)]
fn main() {
    use nettoie_win::systeme::{Ruche, Systeme, ValeurBrute};
    use nettoie_win::windows::SystemeWindows;

    const CLE: &str = r"Software\Nettoie-Win-Essai\Sous-cle";
    let mut sys = SystemeWindows::nouveau().expect("dossier système");
    println!("version de Windows : {:?}", sys.build_windows());

    let lire = |sys: &SystemeWindows, nom: &str| sys.lire_valeur(Ruche::Utilisateur, CLE, nom).expect("lecture");

    assert_eq!(lire(&sys, "A"), None, "une valeur d'une clé absente doit être absente");
    sys.ecrire_valeur(Ruche::Utilisateur, CLE, "A", &ValeurBrute::dword(7)).expect("écriture");
    assert_eq!(lire(&sys, "A"), Some(ValeurBrute::dword(7)));
    assert_eq!(lire(&sys, "a"), Some(ValeurBrute::dword(7)), "le registre ignore la casse");
    assert_eq!(lire(&sys, "B"), None, "une valeur absente d'une clé présente doit être absente");

    // Une valeur d'un autre type (texte) doit revenir à l'octet près.
    let texte = ValeurBrute { genre: 1, octets: "oui\0".encode_utf16().flat_map(|u| u.to_le_bytes()).collect() };
    sys.ecrire_valeur(Ruche::Utilisateur, CLE, "A", &texte).expect("écriture d'un texte");
    assert_eq!(lire(&sys, "A"), Some(texte));

    sys.supprimer_valeur(Ruche::Utilisateur, CLE, "A").expect("suppression");
    assert_eq!(lire(&sys, "A"), None);
    sys.supprimer_valeur(Ruche::Utilisateur, CLE, "A").expect("supprimer deux fois ne doit pas échouer");
    sys.supprimer_valeur(Ruche::Utilisateur, r"Software\Nettoie-Win-Absente", "A")
        .expect("supprimer dans une clé absente ne doit pas échouer");

    assert!(sys.lire_valeur(Ruche::Fichier, "/etc/x", "A").is_err(), "réglage Linux refusé sous Windows");
    assert_eq!(sys.demarrage_service("ServiceQuiNexistePas").expect("service"), None);
    println!("registre : tout est conforme");
}

#[cfg(not(windows))]
fn main() {}
