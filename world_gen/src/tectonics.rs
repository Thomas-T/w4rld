use crate::graph::WorldGraph;
use crate::params::SimParams;
use rand::Rng;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct Plate {
    pub id: usize,
    pub center: (f64, f64),   // Point d'origine
    pub velocity: (f64, f64), // Vecteur de mouvement
    pub is_oceanic: bool,     // Densité (océanique = lourd/bas, continental = léger/haut)
}

/// Statistiques d'un cycle de tectonique, renvoyées au lieu d'être imprimées.
#[derive(Debug, Clone, Default)]
pub struct TectonicStats {
    pub convergent: usize,
    pub divergent: usize,
    /// Arêtes où la surrection de cœur a RÉELLEMENT été appliquée (au moins un côté).
    pub core_edges: usize,
    pub raw_max: f64,
    pub after_diff_max: f64,
    pub pos_sum: f64,
    pub neg_sum: f64,
    /// Somme des valeurs absolues du rappel (relaxation) appliqué ce cycle.
    pub relaxation_abs_sum: f64,
}

/// Génère les plaques tectoniques : graines espacées puis parcours en largeur à sources multiples.
pub fn generate_plates(
    graph: &mut WorldGraph,
    num_plates: usize,
    params: &SimParams,
    rng: &mut impl Rng,
) -> Vec<Plate> {
    let num_centers = graph.centers.len();
    if num_centers == 0 {
        return vec![];
    }

    let non_ghost_centers: Vec<usize> = (0..num_centers)
        .filter(|&i| !graph.centers[i].is_ghost)
        .collect();
    if non_ghost_centers.is_empty() {
        return vec![];
    }

    // Graines : la première au hasard, les suivantes en maximisant la distance minimale
    // aux graines déjà choisies (parmi un échantillon de candidats).
    let mut seeds: Vec<usize> = Vec::new();
    seeds.push(non_ghost_centers[rng.gen_range(0..non_ghost_centers.len())]);

    while seeds.len() < num_plates && seeds.len() < non_ghost_centers.len() {
        let mut best_candidate = None;
        let mut best_min_dist = 0.0;
        let num_candidates = (non_ghost_centers.len() / 10).max(10);
        for _ in 0..num_candidates {
            let candidate = non_ghost_centers[rng.gen_range(0..non_ghost_centers.len())];
            if seeds.contains(&candidate) {
                continue;
            }
            let mut min_dist = f64::INFINITY;
            for &seed_idx in &seeds {
                let dx = graph.centers[candidate].point.0 - graph.centers[seed_idx].point.0;
                let dy = graph.centers[candidate].point.1 - graph.centers[seed_idx].point.1;
                min_dist = min_dist.min(dx * dx + dy * dy);
            }
            if min_dist > best_min_dist {
                best_min_dist = min_dist;
                best_candidate = Some(candidate);
            }
        }
        match best_candidate {
            Some(c) => seeds.push(c),
            None => {
                let remaining: Vec<usize> = non_ghost_centers
                    .iter()
                    .filter(|&&idx| !seeds.contains(&idx))
                    .copied()
                    .collect();
                if remaining.is_empty() {
                    break;
                }
                seeds.push(remaining[rng.gen_range(0..remaining.len())]);
            }
        }
    }

    // Propriétés des plaques : direction, vitesse, nature.
    let mut plates = Vec::new();
    for (plate_id, &seed_idx) in seeds.iter().enumerate() {
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let magnitude = rng.gen_range(params.plate_speed_min..params.plate_speed_max);
        plates.push(Plate {
            id: plate_id,
            center: graph.centers[seed_idx].point,
            velocity: (angle.cos() * magnitude, angle.sin() * magnitude),
            is_oceanic: rng.gen_bool(0.6),
        });
    }

    // Parcours en largeur à sources multiples : (plate_id, distance) par centre.
    let mut queue = VecDeque::new();
    let mut plate_assignments: Vec<Option<(usize, usize)>> = vec![None; num_centers];
    for (plate_id, &seed_idx) in seeds.iter().enumerate() {
        plate_assignments[seed_idx] = Some((plate_id, 0));
        queue.push_back((seed_idx, plate_id, 0));
    }

    while let Some((current_idx, current_plate, current_dist)) = queue.pop_front() {
        match plate_assignments[current_idx] {
            Some((p, d)) if p == current_plate && d == current_dist => {}
            _ => continue, // réassigné depuis : entrée périmée
        }
        // Le clone évite d'emprunter `graph` en lecture pendant qu'on écrit `plate_assignments`.
        let neighbors = graph.centers[current_idx].neighbors.clone();
        for neighbor_idx in neighbors {
            if graph.centers[neighbor_idx].is_ghost {
                continue;
            }
            let candidate = (current_plate, current_dist + 1);
            match plate_assignments[neighbor_idx] {
                None => {
                    plate_assignments[neighbor_idx] = Some(candidate);
                    queue.push_back((neighbor_idx, current_plate, current_dist + 1));
                }
                Some((existing_plate, existing_dist)) => {
                    let closer = current_dist + 1 < existing_dist;
                    let tie_break = current_dist + 1 == existing_dist && current_plate < existing_plate;
                    if closer || tie_break {
                        plate_assignments[neighbor_idx] = Some(candidate);
                        queue.push_back((neighbor_idx, current_plate, current_dist + 1));
                    }
                }
            }
        }
    }

    for i in 0..num_centers {
        if graph.centers[i].is_ghost {
            continue;
        }
        let plate_id = match plate_assignments[i] {
            Some((plate_id, _)) => plate_id,
            None => {
                // Centre isolé (ne devrait pas arriver sur un graphe connexe) : plaque la plus proche.
                let mut best = (f64::INFINITY, 0);
                for plate in &plates {
                    let dx = graph.centers[i].point.0 - plate.center.0;
                    let dy = graph.centers[i].point.1 - plate.center.1;
                    let d = dx * dx + dy * dy;
                    if d < best.0 {
                        best = (d, plate.id);
                    }
                }
                best.1
            }
        };
        graph.centers[i].plate_id = plate_id;
        graph.centers[i].is_oceanic = plates[plate_id].is_oceanic;
    }

    // NOTE : l'équilibrage des tailles de plaques a été supprimé (épisode 1) : il cassait la
    // connexité des plaques. Le test `plaques_connexes` garde la trace de ce défaut.
    plates
}

/// Nombre de composantes connexes de chaque plaque (hors fantômes). Une plaque saine en a une.
pub fn plate_component_counts(graph: &WorldGraph, num_plates: usize) -> Vec<usize> {
    let mut visited = vec![false; graph.centers.len()];
    let mut counts = vec![0usize; num_plates];
    for start in 0..graph.centers.len() {
        if visited[start] || graph.centers[start].is_ghost {
            continue;
        }
        let plate = graph.centers[start].plate_id;
        counts[plate] += 1;
        let mut stack = vec![start];
        visited[start] = true;
        while let Some(i) = stack.pop() {
            for &n in &graph.centers[i].neighbors {
                if !visited[n] && !graph.centers[n].is_ghost && graph.centers[n].plate_id == plate {
                    visited[n] = true;
                    stack.push(n);
                }
            }
        }
    }
    counts
}

/// Composante normale de la vitesse relative des plaques de part et d'autre d'une arête
/// (négative = rapprochement). `None` si l'arête n'est pas une frontière de plaques exploitable.
fn normal_velocity(graph: &WorldGraph, plates: &[Plate], edge: &crate::graph::Edge) -> Option<f64> {
    let a = &graph.centers[edge.d0];
    let b = &graph.centers[edge.d1];
    if a.is_ghost || b.is_ghost || a.plate_id == b.plate_id {
        return None;
    }
    let dx = b.point.0 - a.point.0;
    let dy = b.point.1 - a.point.1;
    let len = (dx * dx + dy * dy).sqrt();
    if len < 1e-10 {
        return None;
    }
    let pa = &plates[a.plate_id];
    let pb = &plates[b.plate_id];
    let v_rel = (pb.velocity.0 - pa.velocity.0, pb.velocity.1 - pa.velocity.1);
    Some((v_rel.0 * dx + v_rel.1 * dy) / len)
}

/// Niveau de base de chaque cellule. Continents : constant. Océans : profil de subsidence
/// thermique, haut à la dorsale (frontière divergente océan-océan) et de plus en plus profond
/// avec la distance à la dorsale la plus proche, en racine carrée, jusqu'à la plaine abyssale.
/// La distance se propage à travers les cellules océaniques seulement : un bassin séparé de toute
/// dorsale par un continent est vieux, donc abyssal.
pub fn assign_base_levels(graph: &mut WorldGraph, plates: &[Plate], params: &SimParams) {
    let n = graph.centers.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut queue = VecDeque::new();
    for edge in &graph.edges {
        let a = &graph.centers[edge.d0];
        let b = &graph.centers[edge.d1];
        if !(a.is_oceanic && b.is_oceanic) {
            continue;
        }
        if let Some(v_n) = normal_velocity(graph, plates, edge) {
            if v_n > 0.0 {
                for idx in [edge.d0, edge.d1] {
                    if dist[idx].is_infinite() {
                        dist[idx] = 0.0;
                        queue.push_back(idx);
                    }
                }
            }
        }
    }
    while let Some(i) = queue.pop_front() {
        for &nb in &graph.centers[i].neighbors {
            let c = &graph.centers[nb];
            if !c.is_ghost && c.is_oceanic && dist[i] + 1.0 < dist[nb] {
                dist[nb] = dist[i] + 1.0;
                queue.push_back(nb);
            }
        }
    }
    for (i, center) in graph.centers.iter_mut().enumerate() {
        if center.is_ghost {
            continue;
        }
        center.base_level = if center.is_oceanic {
            let t = if dist[i].is_finite() { (dist[i] / params.thermal_subsidence_scale).sqrt().min(1.0) } else { 1.0 };
            params.ridge_depth + (params.abyssal_depth - params.ridge_depth) * t
        } else {
            params.base_continental
        };
    }
}

/// Initialise l'élévation au niveau de base de chaque cellule (une seule fois, au début).
/// Les océans partent donc déjà avec leur profil dorsale → plaine abyssale.
pub fn initialize_base_elevation(graph: &mut WorldGraph, _params: &SimParams) {
    for center in &mut graph.centers {
        if !center.is_ghost {
            center.elevation = center.base_level;
        }
    }
    sync_corners_from_centers(graph);
}

/// Un cycle de tectonique : calcule les deltas d'élévation à partir des interactions entre plaques,
/// les diffuse, applique la relaxation, met à jour les âges d'orogenèse. Renvoie les statistiques.
pub fn tectonic_step(graph: &mut WorldGraph, plates: &[Plate], params: &SimParams) -> TectonicStats {
    let mut deltas: Vec<f64> = vec![0.0; graph.centers.len()];
    let mut stats = TectonicStats::default();
    let core_threshold = params.core_threshold_abs();
    let mut core_edge_flags: Vec<bool> = vec![false; graph.edges.len()];

    // 1. Interactions entre plaques, arête par arête.
    for (edge_idx, edge) in graph.edges.iter().enumerate() {
        let center_a = &graph.centers[edge.d0];
        let center_b = &graph.centers[edge.d1];
        if center_a.is_ghost || center_b.is_ghost || center_a.plate_id == center_b.plate_id {
            continue;
        }
        // Vitesse relative projetée sur la normale : négative = rapprochement.
        let v_n = match normal_velocity(graph, plates, edge) {
            Some(v) => v,
            None => continue,
        };
        let compression = (-v_n).max(0.0);
        let extension = v_n.max(0.0);

        if compression > 0.0 {
            stats.convergent += 1;

            // Surrection de cœur : forte, localisée, réservée aux collisions entre deux plaques
            // CONTINENTALES dont les cellules sont émergées, au-dessus du seuil, et seulement tant
            // que l'orogenèse est jeune (fatigue + durée de vie). Le critère de nature de plaque
            // est nouveau : sur l'élévation seule, des cellules océaniques soulevées par diffusion
            // passaient pour des continents et l'orogenèse ne s'éteignait jamais.
            let is_continent_a = !center_a.is_oceanic && center_a.elevation > params.continent_threshold;
            let is_continent_b = !center_b.is_oceanic && center_b.elevation > params.continent_threshold;
            if compression > core_threshold && is_continent_a && is_continent_b {
                let base_core = compression * params.uplift_k * params.core_mult * params.dt;
                let mut applied = false;
                for (idx, age) in [(edge.d0, center_a.orogeny_age), (edge.d1, center_b.orogeny_age)] {
                    if age <= params.max_orogeny_age {
                        let fatigue = (-age / params.fatigue_tau).exp();
                        let uplift = (base_core * fatigue).min(params.core_uplift_cap);
                        if uplift > 0.0 {
                            deltas[idx] += uplift;
                            applied = true;
                        }
                    }
                }
                if applied {
                    core_edge_flags[edge_idx] = true;
                }
            }

            // Surrection / affaissement diffus selon la nature des deux côtés.
            if !center_a.is_oceanic && !center_b.is_oceanic {
                let diffuse_uplift = compression * params.uplift_k * params.dt;
                deltas[edge.d0] += diffuse_uplift;
                deltas[edge.d1] += diffuse_uplift;
            } else if center_a.is_oceanic != center_b.is_oceanic {
                let ocean_subsidence = -compression * params.trench_k * params.dt;
                let continent_diffuse = compression * params.uplift_k * 1.5 * params.dt;
                if center_a.is_oceanic {
                    deltas[edge.d0] += ocean_subsidence;
                    deltas[edge.d1] += continent_diffuse;
                } else {
                    deltas[edge.d0] += continent_diffuse;
                    deltas[edge.d1] += ocean_subsidence;
                }
            } else {
                let subsidence = -compression * params.subsidence_k * 0.8 * params.dt;
                deltas[edge.d0] += subsidence;
                deltas[edge.d1] += subsidence;
            }
        } else if extension > 0.0 {
            stats.divergent += 1;
            if center_a.is_oceanic && center_b.is_oceanic {
                let ridge_uplift = extension * params.ridge_k * params.dt;
                deltas[edge.d0] += ridge_uplift;
                deltas[edge.d1] += ridge_uplift;
            } else {
                let rift_subsidence = -extension * params.subsidence_k * 0.6 * params.dt;
                deltas[edge.d0] += rift_subsidence;
                deltas[edge.d1] += rift_subsidence;
            }
        }
        // v_n == 0 : glissement latéral, pas d'interaction.
    }
    stats.core_edges = core_edge_flags.iter().filter(|&&f| f).count();

    // 2. Statistiques des deltas bruts.
    for &d in &deltas {
        stats.raw_max = stats.raw_max.max(d.abs());
        if d > 0.0 {
            stats.pos_sum += d;
        } else {
            stats.neg_sum += -d;
        }
    }

    // 3. Diffusion des deltas (pas des élévations, pour ne pas aplatir les montagnes).
    diffuse_deltas(graph, &mut deltas, params);
    stats.after_diff_max = deltas.iter().fold(0.0, |m, d| m.max(d.abs()));

    // 4. Application : delta tectonique + relaxation vers le niveau de base de la cellule.
    //    Topographie dynamique : une cellule océanique en subsidence active (fosse, rift) est
    //    soutenue par le forçage lui-même, sa relaxation est suspendue en proportion. Les fonds
    //    ne descendent pas sous le plancher.
    for (idx, delta) in deltas.iter().enumerate() {
        let center = &mut graph.centers[idx];
        if center.is_ghost {
            continue;
        }
        let forcing = if center.is_oceanic && *delta < 0.0 {
            (delta.abs() / params.forcing_exemption_scale).min(1.0)
        } else {
            0.0
        };
        let relaxation = -params.relaxation_k * (1.0 - forcing) * (center.elevation - center.base_level) * params.dt;
        stats.relaxation_abs_sum += relaxation.abs();
        center.elevation += delta + relaxation;
        if center.is_oceanic && center.elevation < params.ocean_floor_min {
            center.elevation = params.ocean_floor_min;
        }

        // L'âge d'orogenèse ne compte que la surrection tectonique, pas la relaxation.
        if *delta > 0.001 && center.orogeny_age <= params.max_orogeny_age {
            center.orogeny_age += params.dt;
        }
    }

    // 5. Orogenèses arrivées en fin de vie.
    for center in &mut graph.centers {
        if !center.is_ghost && center.orogeny_age > params.max_orogeny_age {
            center.orogeny_age = params.max_orogeny_age + 1.0;
        }
    }

    sync_corners_from_centers(graph);
    stats
}

/// Élévation des coins = moyenne des centres réels qui les touchent.
pub fn sync_corners_from_centers(graph: &mut WorldGraph) {
    for corner in &mut graph.corners {
        let mut sum = 0.0;
        let mut count = 0;
        for &ci in &corner.touches {
            let c = &graph.centers[ci];
            if !c.is_ghost {
                sum += c.elevation;
                count += 1;
            }
        }
        if count > 0 {
            corner.elevation = sum / count as f64;
        }
    }
}

/// Diffuse les deltas entre voisins (1 à 3 passes, coefficient plafonné à 0,35).
fn diffuse_deltas(graph: &WorldGraph, deltas: &mut [f64], params: &SimParams) {
    if params.smoothing_k == 0.0 {
        return;
    }
    let num_passes = (params.smoothing_radius.round() as usize).min(3);
    let a = (params.smoothing_k * params.dt).clamp(0.0, 0.35);
    for _ in 0..num_passes {
        let mut new_deltas = vec![0.0; deltas.len()];
        for (idx, center) in graph.centers.iter().enumerate() {
            if center.is_ghost {
                new_deltas[idx] = deltas[idx];
                continue;
            }
            let mut sum = 0.0;
            let mut count = 0;
            for &n in &center.neighbors {
                if !graph.centers[n].is_ghost {
                    sum += deltas[n];
                    count += 1;
                }
            }
            new_deltas[idx] = if count > 0 {
                deltas[idx] * (1.0 - a) + (sum / count as f64) * a
            } else {
                deltas[idx]
            };
        }
        deltas.copy_from_slice(&new_deltas);
    }
}

/// Décale toutes les élévations pour que le quantile voulu soit exactement à zéro.
pub fn adjust_sea_level(graph: &mut WorldGraph, target_land_ratio: f64) {
    let mut elevations: Vec<f64> = graph.centers.iter().filter(|c| !c.is_ghost).map(|c| c.elevation).collect();
    if elevations.is_empty() {
        return;
    }
    elevations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let water_ratio = 1.0 - target_land_ratio;
    let threshold_index = ((water_ratio * elevations.len() as f64).floor() as usize).min(elevations.len() - 1);
    let sea_level = elevations[threshold_index];
    for center in &mut graph.centers {
        if !center.is_ghost {
            center.elevation -= sea_level;
        }
    }
    for corner in &mut graph.corners {
        corner.elevation -= sea_level;
    }
}
