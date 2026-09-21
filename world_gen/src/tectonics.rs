use crate::graph::WorldGraph;
use crate::params::SimParams;
use rand::Rng;
use std::collections::{VecDeque, BTreeMap};

#[derive(Debug, Clone)]
pub struct Plate {
    pub id: usize,
    pub center: (f64, f64), // Point d'origine
    pub velocity: (f64, f64), // Vecteur de mouvement
    pub is_oceanic: bool, // Densité (Oceanic = lourd/bas, Continental = léger/haut)
}

/// Génère les plaques tectoniques en utilisant un multi-source BFS
pub fn generate_plates(graph: &mut WorldGraph, num_plates: usize, params: &SimParams) -> Vec<Plate> {
    let num_centers = graph.centers.len();
    if num_centers == 0 {
        return vec![];
    }
    
    // Filtre les centers non-ghosts pour la sélection des seeds
    let non_ghost_centers: Vec<usize> = (0..num_centers)
        .filter(|&i| !graph.centers[i].is_ghost)
        .collect();
    
    if non_ghost_centers.is_empty() {
        return vec![];
    }
    
    let mut rng = rand::thread_rng();
    let mut plates = Vec::new();
    
    // Choisit num_plates Centers non-ghosts comme graines avec une meilleure distribution
    // Utilise une approche de "maximisation de la distance minimale" pour espacer les graines
    let mut seeds: Vec<usize> = Vec::new();
    
    // Première graine : aléatoire parmi les non-ghosts
    if !non_ghost_centers.is_empty() {
        let first_seed = non_ghost_centers[rng.gen_range(0..non_ghost_centers.len())];
        seeds.push(first_seed);
    }
    
    // Graines suivantes : choisies pour maximiser la distance minimale aux graines existantes
    while seeds.len() < num_plates && seeds.len() < non_ghost_centers.len() {
        let mut best_candidate = None;
        let mut best_min_dist = 0.0;
        
        // Teste plusieurs candidats aléatoires parmi les non-ghosts
        let num_candidates = (non_ghost_centers.len() / 10).max(10);
        for _ in 0..num_candidates {
            let candidate_idx = rng.gen_range(0..non_ghost_centers.len());
            let candidate = non_ghost_centers[candidate_idx];
            
            if seeds.contains(&candidate) {
                continue;
            }
            
            // Calcule la distance minimale aux graines existantes
            let mut min_dist = f64::INFINITY;
            for &seed_idx in &seeds {
                let dx = graph.centers[candidate].point.0 - graph.centers[seed_idx].point.0;
                let dy = graph.centers[candidate].point.1 - graph.centers[seed_idx].point.1;
                let dist = dx * dx + dy * dy;
                min_dist = min_dist.min(dist);
            }
            
            if min_dist > best_min_dist {
                best_min_dist = min_dist;
                best_candidate = Some(candidate);
            }
        }
        
        if let Some(candidate) = best_candidate {
            seeds.push(candidate);
        } else {
            // Fallback : graine aléatoire parmi les non-ghosts non encore sélectionnés
            let remaining: Vec<usize> = non_ghost_centers.iter()
                .filter(|&&idx| !seeds.contains(&idx))
                .copied()
                .collect();
            
            if !remaining.is_empty() {
                let idx = remaining[rng.gen_range(0..remaining.len())];
                seeds.push(idx);
            } else {
                break; // On ne peut plus ajouter de graines
            }
        }
    }
    
    // Initialise les plaques avec des propriétés aléatoires
    for (plate_id, &seed_idx) in seeds.iter().enumerate() {
        let seed_center = &graph.centers[seed_idx];
        
        // Génère une vélocité aléatoire (direction et magnitude)
        let angle = rng.gen_range(0.0..std::f64::consts::TAU);
        let magnitude = rng.gen_range(params.plate_speed_min..params.plate_speed_max);
        let velocity = (angle.cos() * magnitude, angle.sin() * magnitude);
        
        // 60% de chance d'être océanique
        let is_oceanic = rng.gen_bool(0.6);
        
        plates.push(Plate {
            id: plate_id,
            center: seed_center.point,
            velocity,
            is_oceanic,
        });
    }
    
    // Multi-source BFS : une seule propagation depuis toutes les seeds simultanément
    // Structure : (center_idx, plate_id, distance)
    let mut queue = VecDeque::new();
    let mut plate_assignments: Vec<Option<(usize, usize)>> = vec![None; num_centers];
    // (plate_id, distance) pour chaque center
    
    // Initialise la queue avec toutes les seeds
    for (plate_id, &seed_idx) in seeds.iter().enumerate() {
        plate_assignments[seed_idx] = Some((plate_id, 0));
        queue.push_back((seed_idx, plate_id, 0));
    }
    
    // Propagation multi-source BFS
    while let Some((current_idx, current_plate, current_dist)) = queue.pop_front() {
        // Vérifie que l'assignation actuelle correspond toujours (peut avoir changé si réassigné)
        match plate_assignments[current_idx] {
            Some((assigned_plate, assigned_dist)) => {
                // Si l'assignation a changé depuis qu'on a mis ce center dans la queue, ignore
                if assigned_plate != current_plate || assigned_dist != current_dist {
                    continue;
                }
            }
            None => {
                // Ne devrait pas arriver, mais par sécurité
                continue;
            }
        }
        
        let neighbors = graph.centers[current_idx].neighbors.clone();
        
        for neighbor_idx in neighbors {
            // Ignore les ghosts
            if graph.centers[neighbor_idx].is_ghost {
                continue;
            }
            
            // Si le voisin n'a pas encore été assigné, ou s'il a une distance plus grande
            match plate_assignments[neighbor_idx] {
                None => {
                    // Premier accès : assigne à cette plaque
                    plate_assignments[neighbor_idx] = Some((current_plate, current_dist + 1));
                    queue.push_back((neighbor_idx, current_plate, current_dist + 1));
                }
                Some((existing_plate, existing_dist)) => {
                    // Si on trouve une distance égale, utilise un tie-break stable (plate_id le plus bas)
                    if current_dist + 1 == existing_dist {
                        if current_plate < existing_plate {
                            plate_assignments[neighbor_idx] = Some((current_plate, current_dist + 1));
                            queue.push_back((neighbor_idx, current_plate, current_dist + 1));
                        }
                    } else if current_dist + 1 < existing_dist {
                        // Distance plus petite : réassigne
                        plate_assignments[neighbor_idx] = Some((current_plate, current_dist + 1));
                        queue.push_back((neighbor_idx, current_plate, current_dist + 1));
                    }
                }
            }
        }
    }
    
    // Assigne chaque center à sa plaque (ou utilise la distance euclidienne pour les isolés)
    for i in 0..num_centers {
        // Les ghosts gardent plate_id = 0 (par défaut)
        if graph.centers[i].is_ghost {
            continue;
        }
        
        match plate_assignments[i] {
            Some((plate_id, _)) => {
                graph.centers[i].plate_id = plate_id;
                // Assigne is_oceanic selon la plaque
                if let Some(plate) = plates.get(plate_id) {
                    graph.centers[i].is_oceanic = plate.is_oceanic;
                }
            }
            None => {
                // Center isolé : utilise la distance euclidienne
                let mut min_euclidean_dist = f64::INFINITY;
                let mut closest_plate = 0;
                
                for plate in &plates {
                    let dx = graph.centers[i].point.0 - plate.center.0;
                    let dy = graph.centers[i].point.1 - plate.center.1;
                    let dist = dx * dx + dy * dy;
                    
                    if dist < min_euclidean_dist {
                        min_euclidean_dist = dist;
                        closest_plate = plate.id;
                    }
                }
                
                graph.centers[i].plate_id = closest_plate;
                if let Some(plate) = plates.get(closest_plate) {
                    graph.centers[i].is_oceanic = plate.is_oceanic;
                }
            }
        }
    }
    
    // NOTE : L'équilibrage destructeur a été supprimé pour garantir la connexité
    // Chaque plaque forme maintenant une seule composante connexe
    
    plates
}

/// Initialise l'élévation de base selon le type de plaque (à appeler UNE SEULE FOIS au début)
pub fn initialize_base_elevation(graph: &mut WorldGraph) {
    for center in &mut graph.centers {
        if center.is_ghost {
            continue;
        }
        
        if center.is_oceanic {
            center.elevation = -0.5;
        } else {
            center.elevation = 0.1;
        }
    }
    
    // Initialise aussi les corners avec l'élévation moyenne des centers qui les touchent
    for corner in &mut graph.corners {
        if corner.touches.is_empty() {
            corner.elevation = 0.0;
            continue;
        }
        
        let mut sum_elevation = 0.0;
        let mut count = 0;
        for &center_idx in &corner.touches {
            if let Some(center) = graph.centers.get(center_idx) {
                if !center.is_ghost {
                    sum_elevation += center.elevation;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            corner.elevation = sum_elevation / count as f64;
        } else {
            corner.elevation = 0.0;
        }
    }
}

/// Étape incrémentale de tectonique : calcule et applique des deltas d'élévation
/// NE réinitialise PAS l'élévation, accumule les changements
pub fn tectonic_step(graph: &mut WorldGraph, plates: &[Plate], params: &SimParams) {
    // Système de deltas incrémentaux : chaque center reçoit un delta à ajouter
    let mut deltas: Vec<f64> = vec![0.0; graph.centers.len()];
    
    // Compteurs pour les logs
    let mut num_edges_convergent = 0;
    let mut num_edges_divergent = 0;
    let mut num_core_edges = 0;
    
    // 1. Calcule les deltas à partir des interactions entre plaques
    for edge in &graph.edges {
        let center_a = &graph.centers[edge.d0];
        let center_b = &graph.centers[edge.d1];
        
        // Ignore les ghosts
        if center_a.is_ghost || center_b.is_ghost {
            continue;
        }
        
        // Si les deux centers sont sur la même plaque, pas d'interaction
        if center_a.plate_id == center_b.plate_id {
            continue;
        }
        
        // Récupère les plaques
        let plate_a = match plates.get(center_a.plate_id) {
            Some(p) => p,
            None => continue,
        };
        let plate_b = match plates.get(center_b.plate_id) {
            Some(p) => p,
            None => continue,
        };
        
        // Calcule la normale entre les deux centers (vecteur de A vers B)
        let dx = center_b.point.0 - center_a.point.0;
        let dy = center_b.point.1 - center_a.point.1;
        let edge_len = (dx * dx + dy * dy).sqrt();
        
        if edge_len < 1e-10 {
            continue; // Évite la division par zéro
        }
        
        // Normalise le vecteur de l'arête
        let edge_norm = (dx / edge_len, dy / edge_len);
        // Normale perpendiculaire (pointant de A vers B)
        let normal = edge_norm;
        
        // Calcule la vitesse relative
        let v_rel = (plate_b.velocity.0 - plate_a.velocity.0, 
                     plate_b.velocity.1 - plate_a.velocity.1);
        
        // Composante normale de la vitesse relative
        let v_n = v_rel.0 * normal.0 + v_rel.1 * normal.1;
        
        // Définit compression et extension
        let compression = (-v_n).max(0.0); // v_n < 0 => compression > 0
        let extension = v_n.max(0.0);      // v_n > 0 => extension > 0
        
        if compression > 0.0 {
            // CONVERGENCE (compression)
            num_edges_convergent += 1;
            
            // Vérifie si les deux côtés sont continentaux (basé sur l'élévation)
            let is_continent_a = graph.centers[edge.d0].elevation > params.continent_threshold;
            let is_continent_b = graph.centers[edge.d1].elevation > params.continent_threshold;
            
            // Core uplift orogénique : uniquement si compression > threshold ET les deux sont continentaux
            if compression > params.compression_threshold && is_continent_a && is_continent_b {
                num_core_edges += 1;
                
                // Core uplift : très localisé, fort, sans diffusion
                let base_core = compression * params.uplift_k * params.core_mult * params.dt;
                
                // Applique la fatigue tectonique basée sur l'âge de l'orogenèse
                let fatigue_a = (-graph.centers[edge.d0].orogeny_age / params.fatigue_tau).exp();
                let fatigue_b = (-graph.centers[edge.d1].orogeny_age / params.fatigue_tau).exp();
                
                let core_uplift_a = (base_core * fatigue_a).min(params.core_uplift_cap);
                let core_uplift_b = (base_core * fatigue_b).min(params.core_uplift_cap);
                
                // Vérifie si l'orogenèse est encore active
                if graph.centers[edge.d0].orogeny_age <= params.max_orogeny_age {
                    deltas[edge.d0] += core_uplift_a;
                }
                if graph.centers[edge.d1].orogeny_age <= params.max_orogeny_age {
                    deltas[edge.d1] += core_uplift_b;
                }
            }
            
            // Différenciation des types de collisions (uplift diffuse)
            if !center_a.is_oceanic && !center_b.is_oceanic {
                // Collision continent-continent : diffuse uplift
                let diffuse_uplift = compression * params.uplift_k * params.dt;
                deltas[edge.d0] += diffuse_uplift;
                deltas[edge.d1] += diffuse_uplift;
                
            } else if center_a.is_oceanic != center_b.is_oceanic {
                // SUBDUCTION continent-océan : PAS de core uplift, diffuse uplift uniquement
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
                // Collision océan-océan : AUCUN uplift orogénique
                let subsidence = -compression * params.subsidence_k * 0.8 * params.dt;
                deltas[edge.d0] += subsidence;
                deltas[edge.d1] += subsidence;
            }
        } else if extension > 0.0 {
            // DIVERGENCE (extension)
            num_edges_divergent += 1;
            
            if center_a.is_oceanic && center_b.is_oceanic {
                // Dorsale océanique : uplift léger
                let ridge_uplift = extension * params.ridge_k * params.dt;
                deltas[edge.d0] += ridge_uplift;
                deltas[edge.d1] += ridge_uplift;
            } else {
                // Rift continental : subsidence
                let rift_subsidence = -extension * params.subsidence_k * 0.6 * params.dt;
                deltas[edge.d0] += rift_subsidence;
                deltas[edge.d1] += rift_subsidence;
            }
        }
        // Si v_n == 0, pas d'interaction (glissement latéral)
    }
    
    // 2. Calcule les statistiques des deltas bruts (avant diffusion)
    let mut max_abs_delta_raw: f64 = 0.0;
    let mut sum_pos_delta: f64 = 0.0;
    let mut sum_neg_delta: f64 = 0.0;
    for delta in &deltas {
        let abs_delta = delta.abs();
        max_abs_delta_raw = max_abs_delta_raw.max(abs_delta);
        if *delta > 0.0 {
            sum_pos_delta += delta;
        } else if *delta < 0.0 {
            sum_neg_delta += delta.abs();
        }
    }
    
    // 3. Diffusion sur les deltas (pas sur les élévations)
    diffuse_deltas(graph, &mut deltas, params);
    
    // 4. Calcule les statistiques des deltas après diffusion
    let mut max_abs_delta_after: f64 = 0.0;
    for delta in &deltas {
        max_abs_delta_after = max_abs_delta_after.max(delta.abs());
    }
    
    // 5. Applique les deltas diffusés (incrémental)
    for (center_idx, delta) in deltas.iter().enumerate() {
        if let Some(center) = graph.centers.get_mut(center_idx) {
            if !center.is_ghost {
                center.elevation += delta;
                
                // Gère l'âge des orogenèses (incrémente si delta positif significatif)
                if *delta > 0.001 && center.orogeny_age <= params.max_orogeny_age {
                    center.orogeny_age += params.dt;
                }
            }
        }
    }
    
    // 6. Désactive les orogenèses qui ont dépassé leur durée de vie maximale
    for center in &mut graph.centers {
        if !center.is_ghost && center.orogeny_age > params.max_orogeny_age {
            center.orogeny_age = params.max_orogeny_age + 1.0; // Marque comme inactive
        }
    }
    
    // 7. Met à jour les corners avec l'élévation moyenne des centers qui les touchent
    for corner in &mut graph.corners {
        if corner.touches.is_empty() {
            continue;
        }
        
        let mut sum_elevation = 0.0;
        let mut count = 0;
        for &center_idx in &corner.touches {
            if let Some(center) = graph.centers.get(center_idx) {
                if !center.is_ghost {
                    sum_elevation += center.elevation;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            corner.elevation = sum_elevation / count as f64;
        }
    }
    
    // 8. Logs tectoniques détaillés
    println!("    Tectonics: convergent={} divergent={} core_edges={}", 
             num_edges_convergent, num_edges_divergent, num_core_edges);
    println!("      Deltas: raw_max={:.4} after_diff={:.4} pos_sum={:.4} neg_sum={:.4}", 
             max_abs_delta_raw, max_abs_delta_after, sum_pos_delta, sum_neg_delta);
}

/// Diffuse les deltas (pas les élévations) pour éviter d'aplatir les montagnes
fn diffuse_deltas(graph: &WorldGraph, deltas: &mut [f64], params: &SimParams) {
    if params.smoothing_k == 0.0 {
        return; // Pas de diffusion si smoothing_k = 0
    }
    
    // Calcul du nombre de passes de diffusion (1 à 3 max)
    let num_passes = (params.smoothing_radius.round() as usize).min(3);
    if num_passes == 0 {
        return;
    }
    
    // Coefficient de diffusion par passe (clampé à 0.35 max pour éviter d'écraser)
    let a = (params.smoothing_k * params.dt).min(0.35).max(0.0);
    
    for _pass in 0..num_passes {
        // Calcule les nouveaux deltas basés sur la moyenne des voisins
        let mut new_deltas = vec![0.0; deltas.len()];
        
        for (center_idx, center) in graph.centers.iter().enumerate() {
            if center.is_ghost {
                new_deltas[center_idx] = deltas[center_idx];
                continue;
            }
            
            // Calcule la moyenne des deltas des voisins
            let mut sum_delta = 0.0;
            let mut count = 0;
            for &neighbor_idx in &center.neighbors {
                if neighbor_idx < graph.centers.len() && !graph.centers[neighbor_idx].is_ghost {
                    sum_delta += deltas[neighbor_idx];
                    count += 1;
                }
            }
            
            if count > 0 {
                let avg_delta = sum_delta / count as f64;
                // Diffusion : new = old * (1 - a) + avg * a
                new_deltas[center_idx] = deltas[center_idx] * (1.0 - a) + avg_delta * a;
            } else {
                new_deltas[center_idx] = deltas[center_idx];
            }
        }
        
        // Applique les nouveaux deltas
        deltas.copy_from_slice(&new_deltas);
    }
}

/// Ajuste le niveau de la mer pour garantir un ratio précis de terres émergées
pub fn adjust_sea_level(graph: &mut WorldGraph, target_land_ratio: f64) {
    // Étape A : Récupère l'élévation de tous les Centers non-ghosts
    let mut elevations: Vec<f64> = graph.centers.iter()
        .filter(|center| !center.is_ghost)
        .map(|center| center.elevation)
        .collect();
    
    if elevations.is_empty() {
        return;
    }
    
    // Étape B : Trie par ordre croissant
    elevations.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    
    // Étape C : Trouve la valeur seuil (quantile)
    // Si on veut 40% de terre (target_land_ratio = 0.4), 
    // alors 60% doit être sous l'eau, donc le seuil est à l'index (1.0 - 0.4) * total
    let total_elements = elevations.len();
    let water_ratio = 1.0 - target_land_ratio;
    let threshold_index = (water_ratio * total_elements as f64).floor() as usize;
    let threshold_index = threshold_index.min(total_elements - 1);
    let sea_level_threshold = elevations[threshold_index];
    
    // Étape D : Soustrait cette valeur seuil à toutes les élévations
    // Ainsi, le point 0.0 devient exactement la ligne de côte
    for center in &mut graph.centers {
        if !center.is_ghost {
            center.elevation -= sea_level_threshold;
        }
    }
    
    for corner in &mut graph.corners {
        corner.elevation -= sea_level_threshold;
    }
}

