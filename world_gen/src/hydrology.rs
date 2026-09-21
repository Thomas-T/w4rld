use crate::graph::WorldGraph;
use crate::params::SimParams;
use std::collections::VecDeque;

/// Calcule la distance à l'océan pour chaque center en utilisant BFS multi-source
pub fn compute_distance_to_ocean(graph: &WorldGraph, params: &SimParams) -> Vec<f64> {
    let mut dist_to_ocean = vec![f64::INFINITY; graph.centers.len()];
    let mut queue = VecDeque::new();
    
    // Initialise la queue avec tous les centers océaniques
    for (i, center) in graph.centers.iter().enumerate() {
        if !center.is_ghost && center.elevation < 0.0 {
            dist_to_ocean[i] = 0.0;
            queue.push_back(i);
        }
    }
    
    // BFS multi-source pour propager la distance
    while let Some(current_idx) = queue.pop_front() {
        let current_dist = dist_to_ocean[current_idx];
        
        for &neighbor_idx in &graph.centers[current_idx].neighbors {
            if neighbor_idx >= graph.centers.len() {
                continue;
            }
            
            if graph.centers[neighbor_idx].is_ghost {
                continue;
            }
            
            let new_dist = current_dist + params.ocean_distance_unit;
            if new_dist < dist_to_ocean[neighbor_idx] {
                dist_to_ocean[neighbor_idx] = new_dist;
                queue.push_back(neighbor_idx);
            }
        }
    }
    
    dist_to_ocean
}

/// Applique l'érosion thermique (éboulements) pour adoucir les pentes raides
pub fn apply_thermal_erosion(graph: &mut WorldGraph, params: &SimParams) {
    
    for _iteration in 0..params.thermal_passes_per_cycle {
        // Collecte tous les changements d'élévation avant de les appliquer
        // Utilise un HashMap pour accumuler les changements par corner
        let mut elevation_changes: std::collections::HashMap<usize, f64> = std::collections::HashMap::new();
        
        // Pour chaque Corner, regarde ses voisins adjacent
        for corner_idx in 0..graph.corners.len() {
            let corner = &graph.corners[corner_idx];
            let corner_elevation = corner.elevation;
            
            // Parcourt tous les corners adjacents
            for &adjacent_idx in &corner.adjacent {
                if adjacent_idx >= graph.corners.len() {
                    continue;
                }
                
                let adjacent_corner = &graph.corners[adjacent_idx];
                let adjacent_elevation = adjacent_corner.elevation;
                let dh = corner_elevation - adjacent_elevation;
                
                // Si la différence d'élévation dépasse le seuil
                if dh > params.talus {
                    // Calcule la quantité à transférer du point haut vers le point bas
                    let transfer = dh * params.thermal_k;
                    
                    // Le corner actuel est plus haut, donc il perd de l'élévation
                    *elevation_changes.entry(corner_idx).or_insert(0.0) -= transfer;
                    // Le corner adjacent est plus bas, donc il gagne de l'élévation
                    *elevation_changes.entry(adjacent_idx).or_insert(0.0) += transfer;
                }
                // Note : on ne traite que le cas dh > threshold pour éviter de traiter la même paire deux fois
            }
        }
        
        // Applique les changements d'élévation
        for (corner_idx, change) in elevation_changes {
            if let Some(corner) = graph.corners.get_mut(corner_idx) {
                corner.elevation += change;
            }
        }
        
        // Met à jour les élévations des centers avec la moyenne des corners qui les touchent
        for center in &mut graph.centers {
            if center.is_ghost {
                continue;
            }
            
            if center.corners.is_empty() {
                continue;
            }
            
            let mut sum_elevation = 0.0;
            let mut count = 0;
            for &corner_idx in &center.corners {
                if let Some(corner) = graph.corners.get(corner_idx) {
                    sum_elevation += corner.elevation;
                    count += 1;
                }
            }
            
            if count > 0 {
                center.elevation = sum_elevation / count as f64;
            }
        }
    }
}

/// Simule la pluie en calculant la précipitation pour chaque Center
pub fn simulate_rain(graph: &mut WorldGraph, params: &SimParams) {
    // Calcule la distance à l'océan pour tous les centers en utilisant BFS multi-source
    let dist_to_ocean = compute_distance_to_ocean(graph, params);
    
    // Direction du vent : Ouest -> Est (vecteur (1.0, 0.0))
    let wind_direction = (1.0, 0.0);
    
    // Pour chaque center, calcule la précipitation
    let mut moisture_values = vec![0.0; graph.centers.len()];
    
    for center_idx in 0..graph.centers.len() {
        let center = &graph.centers[center_idx];
        if center.is_ghost {
            continue;
        }
        
        // Base : humidité provenant de l'océan (décroît avec la distance)
        let dist = dist_to_ocean[center_idx];
        let base_moisture = if dist < f64::INFINITY {
            // Décroissance exponentielle avec la distance
            (-dist / params.moisture_decay).exp().max(0.0)
        } else {
            0.0
        };
        
        // Étape 3 : Effet orographique
        // Si l'élévation augmente dans la direction du vent, la pluie augmente
        let mut orographic_boost = 1.0;
        
        // Regarde les voisins dans la direction opposée au vent (d'où vient le vent)
        let wind_opposite = (-wind_direction.0, -wind_direction.1);
        
        for &neighbor_idx in &center.neighbors {
            if neighbor_idx >= graph.centers.len() {
                continue;
            }
            
            let neighbor = &graph.centers[neighbor_idx];
            if neighbor.is_ghost {
                continue;
            }
            
            // Vecteur du center vers le voisin
            let dx = neighbor.point.0 - center.point.0;
            let dy = neighbor.point.1 - center.point.1;
            let dist = (dx * dx + dy * dy).sqrt();
            
            if dist > 0.0 {
                // Normalise le vecteur
                let dir = (dx / dist, dy / dist);
                
                // Produit scalaire avec la direction opposée au vent
                let dot = dir.0 * wind_opposite.0 + dir.1 * wind_opposite.1;
                
                // Si le voisin est dans la direction d'où vient le vent
                if dot > 0.5 {
                    // Si l'élévation augmente (voisin plus bas), boost orographique
                    if neighbor.elevation < center.elevation {
                        let elevation_diff = center.elevation - neighbor.elevation;
                        // Plus la différence est grande, plus le boost est important
                        orographic_boost += elevation_diff * 2.0;
                    }
                }
            }
        }
        
        // Applique la précipitation (humidité)
        let final_moisture = base_moisture * orographic_boost.min(5.0);
        moisture_values[center_idx] = params.rain_amount * final_moisture;
    }
    
    // Applique les valeurs de moisture aux centers
    for (center_idx, moisture) in moisture_values.iter().enumerate() {
        if let Some(center) = graph.centers.get_mut(center_idx) {
            if !center.is_ghost {
                center.moisture = *moisture;
            }
        }
    }
    
    // Met à jour aussi les corners avec la moyenne des centers qui les touchent
    for corner in &mut graph.corners {
        let mut sum_moisture = 0.0;
        let mut count = 0;
        for &touch_idx in &corner.touches {
            if let Some(touch_center) = graph.centers.get(touch_idx) {
                if !touch_center.is_ghost {
                    sum_moisture += touch_center.moisture;
                    count += 1;
                }
            }
        }
        if count > 0 {
            corner.moisture = sum_moisture / count as f64;
        }
    }
    
    // Met à jour la moisture des corners avec la moyenne des centers qui les touchent
    for corner in &mut graph.corners {
        if corner.touches.is_empty() {
            corner.moisture = 0.0;
            continue;
        }
        
        let mut sum_moisture = 0.0;
        let mut count = 0;
        for &center_idx in &corner.touches {
            if let Some(center) = graph.centers.get(center_idx) {
                if !center.is_ghost {
                    sum_moisture += center.moisture;
                    count += 1;
                }
            }
        }
        
        if count > 0 {
            corner.moisture = sum_moisture / count as f64;
        } else {
            corner.moisture = 0.0;
        }
    }
}

/// Calcule le downslope (voisin le plus bas) pour chaque corner
fn calculate_downslope(graph: &WorldGraph) -> Vec<Option<usize>> {
    let mut downslopes = vec![None; graph.corners.len()];
    
    for corner_idx in 0..graph.corners.len() {
        let corner = &graph.corners[corner_idx];
        let corner_elevation = corner.elevation;
        
        let mut lowest_neighbor = None;
        let mut lowest_elevation = corner_elevation;
        
        // Trouve le voisin adjacent le plus bas
        for &adjacent_idx in &corner.adjacent {
            if adjacent_idx >= graph.corners.len() {
                continue;
            }
            
            let adjacent_elevation = graph.corners[adjacent_idx].elevation;
            if adjacent_elevation < lowest_elevation {
                lowest_elevation = adjacent_elevation;
                lowest_neighbor = Some(adjacent_idx);
            }
        }
        
        downslopes[corner_idx] = lowest_neighbor;
    }
    
    downslopes
}

/// Accumule le flux d'eau de corner en corner
fn accumulate_flux(graph: &WorldGraph, downslopes: &[Option<usize>], corner_moisture: &[f64]) -> Vec<f64> {
    let mut river_flow = vec![0.0; graph.corners.len()];
    
    // Initialise avec la moisture de chaque corner
    for i in 0..graph.corners.len() {
        river_flow[i] = corner_moisture[i];
    }
    
    // Fait couler l'eau vers le downslope
    // On fait plusieurs passes pour accumuler le flux
    for _pass in 0..5 {
        let mut new_flow = river_flow.clone();
        
        for corner_idx in 0..graph.corners.len() {
            if let Some(downslope_idx) = downslopes[corner_idx] {
                // Transfère une partie du flux vers le downslope
                let transfer = river_flow[corner_idx] * 0.8;
                new_flow[corner_idx] -= transfer;
                new_flow[downslope_idx] += transfer;
            }
        }
        
        river_flow = new_flow;
    }
    
    river_flow
}

/// Érode le terrain là où le flux est fort et la pente est raide
/// SimParams hookup (hydraulic)
fn erode(graph: &mut WorldGraph, river_flow: &[f64], downslopes: &[Option<usize>], params: &SimParams) {
    // Calcule les sédiments disponibles (proxy basé sur le flux d'eau)
    let sediment: Vec<f64> = river_flow.iter().map(|&flow| flow * 0.1).collect();
    
    for corner_idx in 0..graph.corners.len() {
        let corner = &graph.corners[corner_idx];
        let flow = river_flow[corner_idx];
        
        // Si le flux est faible, pas d'érosion significative
        if flow < 0.1 {
            continue;
        }
        
        // Trouve la pente (différence d'élévation avec le downslope)
        if let Some(downslope_idx) = downslopes[corner_idx] {
            let corner_elevation = corner.elevation;
            let downslope_elevation = graph.corners[downslope_idx].elevation;
            let slope = corner_elevation - downslope_elevation;
            
            if slope > 0.0 {
                // A) Érosion fluviale : erosion = river_erosion_k * water_flux * slope * dt
                let erosion_amount = params.river_erosion_k * flow * slope * params.dt;
                
                // Clamp l'érosion pour éviter des valeurs extrêmes (max 10% de la hauteur locale par step)
                let max_erosion = corner_elevation * 0.1;
                let erosion_amount = erosion_amount.min(max_erosion);
                
                // Ne creuse pas sous le niveau de la mer (0.0)
                let new_elevation = (corner_elevation - erosion_amount).max(0.0);
                let actual_erosion = corner_elevation - new_elevation;
                
                if let Some(corner_mut) = graph.corners.get_mut(corner_idx) {
                    corner_mut.elevation = new_elevation;
                }
                
                // B) Dépôt de sédiments : deposit = deposition_k * sediment * (1 - slope_norm) * dt
                // Normalise la pente dans [0, 1] pour le calcul du dépôt
                // On utilise une pente relative (plus la pente est faible, plus le dépôt est fort)
                let slope_norm = (slope / 2.0).min(1.0).max(0.0); // Normalise sur une pente max de 2.0
                let deposit = params.deposition_k * sediment[corner_idx] * (1.0 - slope_norm) * params.dt;
                
                // Dépose dans le downslope (plaines, deltas)
                if deposit > 0.0 && actual_erosion > 0.0 {
                    // Le dépôt est proportionnel aux sédiments disponibles et à la faible pente
                    if let Some(downslope_corner) = graph.corners.get_mut(downslope_idx) {
                        downslope_corner.elevation += deposit;
                    }
                }
            }
        }
    }
    
    // Met à jour les élévations des centers avec la moyenne des corners qui les touchent
    for center in &mut graph.centers {
        if center.is_ghost {
            continue;
        }
        
        if center.corners.is_empty() {
            continue;
        }
        
        let mut sum_elevation = 0.0;
        let mut count = 0;
        for &corner_idx in &center.corners {
            if let Some(corner) = graph.corners.get(corner_idx) {
                sum_elevation += corner.elevation;
                count += 1;
            }
        }
        
        if count > 0 {
            center.elevation = sum_elevation / count as f64;
        }
    }
}

/// Applique l'érosion hydraulique (eau qui creuse les vallées)
pub fn apply_hydraulic_erosion(graph: &mut WorldGraph, params: &SimParams) {
    for _iteration in 0..1 {
        // Étape 1 : Calcule le downslope pour chaque corner
        let downslopes = calculate_downslope(graph);
        
        // Étape 2 : Récupère la moisture des corners
        let corner_moisture: Vec<f64> = graph.corners.iter().map(|c| c.moisture).collect();
        
        // Étape 3 : Accumule le flux d'eau
        let river_flow = accumulate_flux(graph, &downslopes, &corner_moisture);
        
        // SimParams hookup (hydraulic)
        // Étape 4 : Érode le terrain et dépose les sédiments
        erode(graph, &river_flow, &downslopes, params);
        
        // Met à jour les downslopes dans le graphe (pour visualisation future)
        for (corner_idx, &downslope) in downslopes.iter().enumerate() {
            if let Some(corner) = graph.corners.get_mut(corner_idx) {
                corner.downslope = downslope;
                corner.river_flow = river_flow[corner_idx];
            }
        }
    }
}

