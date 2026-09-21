mod graph;
mod mesh;
mod drawer;
mod tectonics;
mod hydrology;
mod gif_export;
mod params;

use mesh::build_graph;
use drawer::{draw_mesh, draw_plates, draw_elevation_with_params};
use tectonics::{generate_plates, tectonic_step, initialize_base_elevation, adjust_sea_level};
use hydrology::{apply_thermal_erosion, simulate_rain, apply_hydraulic_erosion};
use gif_export::create_gif_from_pngs;
use params::SimParams;

fn main() {
    let width = 1000.0;
    let height = 1000.0;
    let num_points = 10000;
    let num_plates = 50;
    
    // Paramètres de simulation centralisés
    let params = SimParams::default();
    
    // Liste des fichiers PNG pour le GIF
    let mut png_files = Vec::new();
    
    println!("Génération du graphe Voronoï...");
    let mut graph = build_graph(width, height, num_points);
    
    println!("Dessin du mesh initial...");
    draw_mesh(&graph, "step1_mesh.png");
    png_files.push("step1_mesh.png".to_string());
    
    println!("Génération de {} plaques tectoniques...", num_plates);
    let plates = generate_plates(&mut graph, num_plates, &params);
    
    println!("Dessin des plaques...");
    draw_plates(&graph, &plates, "step2_plates.png");
    png_files.push("step2_plates.png".to_string());
    
    println!("Initialisation de l'élévation de base...");
    initialize_base_elevation(&mut graph);
    
    // PNG après l'élévation de base
    let filename = "step3_base_elevation.png";
    draw_elevation_with_params(&graph, filename, Some(&params), 0.0);
    png_files.push(filename.to_string());
    println!("  → {}", filename);
    
    println!("Simulation de croissance tectonique ({} cycles)...", params.tectonic_cycles);
    for cycle in 0..params.tectonic_cycles {
        // Étape incrémentale de tectonique
        tectonic_step(&mut graph, &plates, &params);
        
        // Érosion thermique
        if params.thermal_passes_per_cycle > 0 {
            apply_thermal_erosion(&mut graph, &params);
        }
        
        // Logs de debug : min/max/mean elevation après tectonique
        let elevations: Vec<f64> = graph.centers.iter()
            .filter(|c| !c.is_ghost)
            .map(|c| c.elevation)
            .collect();
        if !elevations.is_empty() {
            let min_elev = elevations.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max_elev = elevations.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let mean_elev = elevations.iter().sum::<f64>() / elevations.len() as f64;
            let variance = elevations.iter().map(|&e| (e - mean_elev).powi(2)).sum::<f64>() / elevations.len() as f64;
            let stddev = variance.sqrt();
            println!("  Cycle {}: elev min={:.3} max={:.3} mean={:.3} stddev={:.3}", 
                     cycle + 1, min_elev, max_elev, mean_elev, stddev);
        }
        
        // PNG à chaque cycle
        let filename = format!("step4_tectonic_cycle_{:02}.png", cycle + 1);
        draw_elevation_with_params(&graph, &filename, Some(&params), 0.0);
        png_files.push(filename);
        
        if cycle % 5 == 0 && cycle > 0 {
            println!("  → Cycle {}/{}...", cycle + 1, params.tectonic_cycles);
        }
    }
    
    println!("Ajustement du niveau de la mer...");
    adjust_sea_level(&mut graph, params.land_ratio);
    
    // PNG après ajustement du niveau de la mer (sea_level = 0.0 après adjust_sea_level)
    let filename = "step5_after_sea_level.png";
    draw_elevation_with_params(&graph, filename, Some(&params), 0.0);
    png_files.push(filename.to_string());
    println!("  → {}", filename);
    
    println!("Simulation de l'érosion hydraulique ({} cycles)...", params.rain_cycles);
    for cycle in 0..params.rain_cycles {
        simulate_rain(&mut graph, &params);
        apply_hydraulic_erosion(&mut graph, &params);
        
        // PNG à chaque cycle
        let filename = format!("step6_hydraulic_cycle_{:02}.png", cycle + 1);
        draw_elevation_with_params(&graph, &filename, Some(&params), 0.0);
        png_files.push(filename);
        
        if cycle % 2 == 0 {
            println!("  Cycle {}/{}...", cycle + 1, params.rain_cycles);
        }
    }
    
    // PNG final
    let filename = "step7_final_world.png";
    draw_elevation_with_params(&graph, filename, Some(&params), 0.0);
    png_files.push(filename.to_string());
    println!("  → {}", filename);
    
    println!("Création du GIF animé...");
    match create_gif_from_pngs(&png_files, "world_evolution.gif", 100) {
        Ok(_) => println!("  → world_evolution.gif créé avec succès!"),
        Err(e) => eprintln!("  Erreur lors de la création du GIF: {}", e),
    }
    
    println!("\nImages sauvegardées:");
    println!("  - step1_mesh.png");
    println!("  - step2_plates.png");
    println!("  - step3_base_elevation.png");
    println!("  - step4_tectonic_cycle_01.png à step4_tectonic_cycle_30.png (30 images)");
    println!("  - step5_after_sea_level.png");
    println!("  - step6_hydraulic_cycle_01.png à step6_hydraulic_cycle_10.png (10 images)");
    println!("  - step7_final_world.png");
    println!("  - world_evolution.gif (animation de toutes les étapes)");
}
