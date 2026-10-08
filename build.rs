//! Intègre l'icône et le manifeste (demande des droits d'administrateur) au `.exe`.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=ressources");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let sortie = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("nettoie-win.res");
    // `zig rc` est le compilateur de ressources fourni avec zig, déjà nécessaire à la compilation.
    let reussi = Command::new("zig")
        .args(["rc", "/:auto-includes", "none", "/fo"])
        .arg(&sortie)
        .arg("nettoie-win.rc")
        .current_dir("ressources")
        .status()
        .map(|etat| etat.success())
        .unwrap_or(false);
    if reussi {
        println!("cargo:rustc-link-arg-bins={}", sortie.display());
    } else {
        // Le programme reste utilisable : il demande alors les droits lui-même au lancement.
        println!("cargo:warning=zig rc indisponible : .exe sans icône ni manifeste");
    }
}
