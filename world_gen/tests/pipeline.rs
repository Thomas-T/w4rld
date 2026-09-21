//! Tests d'intégration du pipeline. Chaque test verrouille un défaut de l'épisode 1 : il doit
//! passer avec les paramètres corrigés et, quand c'est pertinent, échouer en mode `legacy`.

use world_gen::params::SimParams;
use world_gen::pipeline::{run, Outcome};
use world_gen::tectonics::plate_component_counts;

fn petit_monde(legacy: bool) -> SimParams {
    let mut p = if legacy { SimParams::legacy() } else { SimParams::default() };
    p.width = 400.0;
    p.height = 400.0;
    p.num_points = 1500;
    p.num_plates = 12;
    p.tectonic_cycles = 20;
    p.rain_cycles = 6;
    p
}

fn lance(p: &SimParams, seed: u64) -> Outcome {
    run(p, seed, |_, _, _| {})
}

fn ligne<'a>(o: &'a Outcome, phase: &str, cycle: usize) -> &'a world_gen::pipeline::CycleStats {
    o.stats.iter().find(|s| s.phase == phase && s.cycle == cycle).expect("mesure absente")
}

#[test]
fn meme_seed_meme_empreinte() {
    let p = petit_monde(false);
    let a = lance(&p, 7);
    let b = lance(&p, 7);
    assert_eq!(a.fingerprint, b.fingerprint, "deux exécutions du même seed doivent être identiques");
}

#[test]
fn seeds_differents_mondes_differents() {
    let p = petit_monde(false);
    assert_ne!(lance(&p, 7).fingerprint, lance(&p, 8).fingerprint);
}

#[test]
fn le_niveau_de_la_mer_donne_le_ratio_de_terres() {
    let p = petit_monde(false);
    let o = lance(&p, 7);
    let s = ligne(&o, "sea_level", 0);
    assert!((s.land_ratio - p.land_ratio).abs() < 0.01, "terres = {:.3}, attendu {:.2}", s.land_ratio, p.land_ratio);
}

#[test]
fn l_erosion_ne_remonte_pas_le_fond_marin() {
    let p = petit_monde(false);
    let o = lance(&p, 7);
    let avant = ligne(&o, "sea_level", 0);
    let apres = ligne(&o, "hydro", p.rain_cycles);
    assert_eq!(apres.zero_corners, 0, "aucun coin ne doit finir exactement à 0,0");
    assert!(
        apres.underwater_corners as f64 >= 0.98 * avant.underwater_corners as f64,
        "le fond marin a été remonté : {} coins sous l'eau avant, {} après",
        avant.underwater_corners, apres.underwater_corners
    );
}

#[test]
fn en_mode_legacy_l_erosion_remonte_le_fond_marin() {
    let p = petit_monde(true);
    let o = lance(&p, 7);
    let apres = ligne(&o, "hydro", p.rain_cycles);
    assert!(apres.zero_corners > 0, "le défaut de l'épisode 1 doit être reproductible en mode legacy");
}

#[test]
fn les_plaques_sont_connexes() {
    let p = petit_monde(false);
    let o = lance(&p, 7);
    let counts = plate_component_counts(&o.graph, p.num_plates);
    assert!(counts.iter().all(|&c| c == 1), "composantes par plaque : {:?}", counts);
}

#[test]
fn l_orogenese_de_coeur_se_declenche_puis_s_eteint() {
    let p = petit_monde(false);
    // Sur quelques seeds, au moins un monde doit contenir une collision continent-continent
    // assez forte ; et dans tout monde, l'orogenèse doit s'être éteinte au 20e cycle
    // (durée de vie 15 cycles).
    let mut un_monde_a_declenche = false;
    for seed in 1..=5 {
        let o = lance(&p, seed);
        if ligne(&o, "tectonic", 1).core_edges > 0 {
            un_monde_a_declenche = true;
        }
        assert_eq!(ligne(&o, "tectonic", p.tectonic_cycles).core_edges, 0, "seed {} : l'orogenèse devrait être éteinte", seed);
    }
    assert!(un_monde_a_declenche, "l'orogenèse de cœur ne s'est jamais déclenchée : le seuil est encore inatteignable");

    let legacy = petit_monde(true);
    for seed in 1..=5 {
        let o = lance(&legacy, seed);
        assert!(
            o.stats.iter().filter(|s| s.phase == "tectonic").all(|s| s.core_edges == 0),
            "en mode legacy le seuil 0,55 > 0,5 ne doit jamais être franchi"
        );
    }
}

#[test]
fn le_relief_sature_avec_l_isostasie() {
    // Avec isostasie, l'altitude maximale atteint un plateau (voire redescend quand l'orogenèse
    // s'éteint) ; sans, elle continue de monter, seulement freinée par l'érosion thermique.
    let mut p = petit_monde(false);
    p.tectonic_cycles = 30;
    let o = lance(&p, 7);
    let max = |c: usize| ligne(&o, "tectonic", c).max;
    let croissance_fin = max(30) - max(20);
    assert!(croissance_fin < 0.05, "le relief devrait avoir saturé : max(20) = {:.3}, max(30) = {:.3}", max(20), max(30));

    let mut legacy = petit_monde(true);
    legacy.tectonic_cycles = 30;
    let o = lance(&legacy, 7);
    let max = |c: usize| ligne(&o, "tectonic", c).max;
    let croissance_fin = max(30) - max(20);
    assert!(croissance_fin > 0.1, "en mode legacy le relief monte encore : max(20) = {:.3}, max(30) = {:.3}", max(20), max(30));
}
