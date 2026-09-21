use crate::graph::WorldGraph;
use crate::hydrology::{apply_hydraulic_erosion, apply_thermal_erosion, simulate_rain};
use crate::mesh::build_graph;
use crate::params::SimParams;
use crate::tectonics::{
    adjust_sea_level, generate_plates, initialize_base_elevation, tectonic_step, Plate, TectonicStats,
};
use rand::{rngs::StdRng, SeedableRng};
use std::time::{Duration, Instant};

/// Statistiques prises après chaque étape, pour le log, le CSV et les tests.
#[derive(Debug, Clone)]
pub struct CycleStats {
    pub phase: &'static str, // "base", "tectonic", "sea_level", "hydro"
    pub cycle: usize,
    pub min: f64,
    pub max: f64,
    pub mean: f64,
    pub std: f64,
    pub convergent: usize,
    pub divergent: usize,
    pub core_edges: usize,
    /// Coins du maillage exactement à l'altitude 0,0 (symptôme du bug d'érosion de l'épisode 1).
    pub zero_corners: usize,
    pub underwater_corners: usize,
    pub land_ratio: f64,
}

impl CycleStats {
    pub const CSV_HEADER: &'static str =
        "phase,cycle,min,max,mean,std,convergent,divergent,core_edges,zero_corners,underwater_corners,land_ratio";

    pub fn csv_row(&self) -> String {
        format!(
            "{},{},{:.4},{:.4},{:.4},{:.4},{},{},{},{},{},{:.4}",
            self.phase, self.cycle, self.min, self.max, self.mean, self.std, self.convergent,
            self.divergent, self.core_edges, self.zero_corners, self.underwater_corners, self.land_ratio
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Timings {
    pub mesh: Duration,
    pub plates: Duration,
    pub tectonics: Duration,
    pub hydrology: Duration,
    pub render: Duration,
    pub total: Duration,
}

pub struct Outcome {
    pub graph: WorldGraph,
    pub plates: Vec<Plate>,
    /// Empreinte FNV-1a des élévations finales : même seed + même code = même empreinte.
    pub fingerprint: u64,
    pub stats: Vec<CycleStats>,
    pub timings: Timings,
}

/// FNV-1a 64 bits sur les bits des élévations des centres réels, dans l'ordre des indices.
pub fn fingerprint(graph: &WorldGraph) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for c in graph.centers.iter().filter(|c| !c.is_ghost) {
        for b in c.elevation.to_bits().to_le_bytes() {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    h
}

/// Prend une photo statistique du monde.
pub fn measure(graph: &WorldGraph, phase: &'static str, cycle: usize, t: Option<&TectonicStats>) -> CycleStats {
    let elevations: Vec<f64> = graph.centers.iter().filter(|c| !c.is_ghost).map(|c| c.elevation).collect();
    let n = elevations.len().max(1) as f64;
    let min = elevations.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = elevations.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mean = elevations.iter().sum::<f64>() / n;
    let std = (elevations.iter().map(|e| (e - mean).powi(2)).sum::<f64>() / n).sqrt();
    let land = elevations.iter().filter(|&&e| e > 0.0).count();
    CycleStats {
        phase,
        cycle,
        min,
        max,
        mean,
        std,
        convergent: t.map(|t| t.convergent).unwrap_or(0),
        divergent: t.map(|t| t.divergent).unwrap_or(0),
        core_edges: t.map(|t| t.core_edges).unwrap_or(0),
        zero_corners: graph.corners.iter().filter(|c| c.elevation == 0.0).count(),
        underwater_corners: graph.corners.iter().filter(|c| c.elevation < 0.0).count(),
        land_ratio: land as f64 / n,
    }
}

/// Déroule le pipeline complet. `on_frame(nom, graphe, plaques)` est appelé à chaque étape
/// rendable ; passer une closure vide pour simuler sans dessiner.
pub fn run<F>(params: &SimParams, seed: u64, mut on_frame: F) -> Outcome
where
    F: FnMut(&str, &WorldGraph, &[Plate]),
{
    let t_total = Instant::now();
    let mut timings = Timings::default();
    let mut stats = Vec::new();
    let mut rng = StdRng::seed_from_u64(seed);

    let mut frame = |name: &str, g: &WorldGraph, p: &[Plate], timings: &mut Timings| {
        let t = Instant::now();
        on_frame(name, g, p);
        timings.render += t.elapsed();
    };

    let t = Instant::now();
    let mut graph = build_graph(params.width, params.height, params.num_points, &mut rng);
    timings.mesh = t.elapsed();
    frame("step1_mesh", &graph, &[], &mut timings);

    let t = Instant::now();
    let plates = generate_plates(&mut graph, params.num_plates, params, &mut rng);
    timings.plates = t.elapsed();
    frame("step2_plates", &graph, &plates, &mut timings);

    initialize_base_elevation(&mut graph, params);
    stats.push(measure(&graph, "base", 0, None));
    frame("step3_base_elevation", &graph, &plates, &mut timings);

    let render_before = timings.render;
    let t = Instant::now();
    for cycle in 1..=params.tectonic_cycles {
        let ts = tectonic_step(&mut graph, &plates, params);
        if params.thermal_passes_per_cycle > 0 {
            apply_thermal_erosion(&mut graph, params);
        }
        stats.push(measure(&graph, "tectonic", cycle, Some(&ts)));
        frame(&format!("step4_tectonic_cycle_{:02}", cycle), &graph, &plates, &mut timings);
    }
    // Le chrono englobe le rendu des images du cycle : on le retranche pour ne garder que la simulation.
    timings.tectonics = t.elapsed() - (timings.render - render_before);

    adjust_sea_level(&mut graph, params.land_ratio);
    stats.push(measure(&graph, "sea_level", 0, None));
    frame("step5_after_sea_level", &graph, &plates, &mut timings);

    let render_before_hydro = timings.render;
    let t = Instant::now();
    for cycle in 1..=params.rain_cycles {
        simulate_rain(&mut graph, params);
        apply_hydraulic_erosion(&mut graph, params);
        stats.push(measure(&graph, "hydro", cycle, None));
        frame(&format!("step6_hydraulic_cycle_{:02}", cycle), &graph, &plates, &mut timings);
    }
    timings.hydrology = t.elapsed() - (timings.render - render_before_hydro);

    frame("step7_final_world", &graph, &plates, &mut timings);
    timings.total = t_total.elapsed();

    let fp = fingerprint(&graph);
    Outcome { graph, plates, fingerprint: fp, stats, timings }
}
