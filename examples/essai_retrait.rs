//! Essai du retrait réel d'un paquet Linux, avec les mêmes garde-fous que le programme.
//! À ne lancer qu'en administrateur dans une machine d'essai jetable.

#[cfg(unix)]
fn main() {
    use nettoie_win::assistant::Demande;
    let nom = std::env::args().nth(1).expect("usage : essai_retrait <paquet>");
    println!("{nom} : {:?}", nettoie_win::linux::executer_en_root(&Demande::RetirerPaquet { nom: nom.clone() }));
}

#[cfg(not(unix))]
fn main() {}
