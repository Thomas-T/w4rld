use crate::graph::WorldGraph;
use crate::tectonics::Plate;
use crate::params::SimParams;
use image::{ImageBuffer, Rgb, RgbImage};
use imageproc::drawing::{draw_polygon_mut, draw_filled_circle_mut};
use imageproc::point::Point as ImgPoint;

/// Trie les corners d'un center par angle autour du point central
fn sort_corners_by_angle(center: &crate::graph::Center, corners: &[usize], graph: &WorldGraph) -> Vec<usize> {
    let center_point = center.point;
    let mut corner_indices: Vec<(usize, f64)> = corners.iter()
        .filter_map(|&idx| {
            graph.corners.get(idx).map(|corner| {
                let dx = corner.point.0 - center_point.0;
                let dy = corner.point.1 - center_point.1;
                let angle = dy.atan2(dx); // Angle en radians
                (idx, angle)
            })
        })
        .collect();
    
    // Trie par angle
    corner_indices.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
    
    corner_indices.into_iter().map(|(idx, _)| idx).collect()
}

pub fn draw_mesh(graph: &WorldGraph, filename: &str) {
    let mut img: RgbImage = ImageBuffer::from_pixel(
        graph.width as u32, 
        graph.height as u32, 
        Rgb([255, 255, 255])
    );

    // Pour chaque Center
    for center in &graph.centers {
        // OPTIMISATION : On ne dessine PAS les Fantômes (points hors map)
        if center.point.0 < 0.0 || center.point.0 > graph.width || 
           center.point.1 < 0.0 || center.point.1 > graph.height {
            continue;
        }

        // 1. Trie les corners par angle autour du center
        let sorted_corners = sort_corners_by_angle(center, &center.corners, graph);
        
        // 2. Récupère les points du polygone dans l'ordre trié
        let points: Vec<ImgPoint<i32>> = sorted_corners.iter()
            .map(|&c_idx| {
                let c = &graph.corners[c_idx];
                ImgPoint::new(c.point.0 as i32, c.point.1 as i32)
            })
            .collect();

        if points.len() < 3 { continue; }

        // 3. Dessine le contour (Polygone gris clair)
        // Note : imageproc n'a pas de draw_polygon_outline simple, on dessine des lignes 
        // ou on remplit avec une couleur aléatoire légère pour debug.
        // Ici on va juste dessiner les arêtes une par une
        for i in 0..points.len() {
            let p1 = points[i];
            let p2 = points[(i + 1) % points.len()];
            imageproc::drawing::draw_line_segment_mut(
                &mut img, 
                (p1.x as f32, p1.y as f32), 
                (p2.x as f32, p2.y as f32), 
                Rgb([200, 200, 200])
            );
        }

        // 4. Dessine le centre (Point Rouge)
        draw_filled_circle_mut(
            &mut img, 
            (center.point.0 as i32, center.point.1 as i32), 
            2, 
            Rgb([255, 0, 0])
        );
    }

    img.save(filename).unwrap();
}

pub fn draw_plates(graph: &WorldGraph, _plates: &[Plate], filename: &str) {
    let mut img: RgbImage = ImageBuffer::new(graph.width as u32, graph.height as u32);

    // Palette de couleurs fixe pour les plaques
    let colors = vec![
        Rgb([230, 25, 75]), Rgb([60, 180, 75]), Rgb([255, 225, 25]), Rgb([0, 130, 200]),
        Rgb([245, 130, 48]), Rgb([145, 30, 180]), Rgb([70, 240, 240]), Rgb([240, 50, 230]),
        Rgb([210, 245, 60]), Rgb([250, 190, 190]), Rgb([0, 128, 128]), Rgb([230, 190, 255]),
        Rgb([170, 110, 40]), Rgb([255, 250, 200]), Rgb([128, 0, 0]), Rgb([170, 255, 195]),
    ];

    for center in &graph.centers {
        // Ignore les fantômes
        if center.point.0 < 0.0 || center.point.0 > graph.width || 
           center.point.1 < 0.0 || center.point.1 > graph.height {
            continue;
        }

        let color = colors[center.plate_id % colors.len()];
        
        // Trie les corners par angle autour du center
        let sorted_corners = sort_corners_by_angle(center, &center.corners, graph);
        
        let mut points: Vec<ImgPoint<i32>> = sorted_corners.iter()
            .map(|&c_idx| {
                let c = &graph.corners[c_idx];
                ImgPoint::new(c.point.0 as i32, c.point.1 as i32)
            })
            .collect();
        
        // Supprime les doublons consécutifs
        if points.len() > 1 {
            points.dedup();
        }
        
        // Supprime le dernier point s'il est identique au premier
        if points.len() > 2 && points[0] == points[points.len() - 1] {
            points.pop();
        }

        if points.len() >= 3 {
            draw_polygon_mut(&mut img, &points, color);
        }
    }

    img.save(filename).unwrap();
}

pub fn draw_elevation(graph: &WorldGraph, filename: &str) {
    draw_elevation_with_params(graph, filename, None, 0.0)
}

pub fn draw_elevation_with_params(graph: &WorldGraph, filename: &str, params: Option<&SimParams>, sea_level: f64) {
    let mut img: RgbImage = ImageBuffer::new(graph.width as u32, graph.height as u32);
    
    // Détermine si on utilise le mode grayscale
    let use_grayscale = params.map(|p| p.debug_grayscale).unwrap_or(false);
    let coastline_color = params.map(|p| p.coastline_blue_rgb).unwrap_or((0, 120, 255));
    
    // Calcule min/max elevation pour la normalisation grayscale
    let (min_elev, max_elev) = if use_grayscale {
        let elevations: Vec<f64> = graph.centers.iter()
            .filter(|c| !c.is_ghost && 
                    c.point.0 >= 0.0 && c.point.0 <= graph.width &&
                    c.point.1 >= 0.0 && c.point.1 <= graph.height)
            .map(|c| c.elevation)
            .collect();
        
        if elevations.is_empty() {
            (0.0, 0.0)
        } else {
            let min = elevations.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            let max = elevations.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            (min, max)
        }
    } else {
        (0.0, 0.0) // Pas utilisé en mode couleur
    };
    
    // Précalcule quels centers sont des côtes (pour le mode grayscale)
    let mut is_coast = vec![false; graph.centers.len()];
    if use_grayscale {
        for (center_idx, center) in graph.centers.iter().enumerate() {
            if center.is_ghost {
                continue;
            }
            
            let is_water = center.elevation <= sea_level;
            
            // Vérifie si au moins un voisin est de type différent
            for &neighbor_idx in &center.neighbors {
                if neighbor_idx < graph.centers.len() {
                    let neighbor = &graph.centers[neighbor_idx];
                    if !neighbor.is_ghost {
                        let neighbor_is_water = neighbor.elevation <= sea_level;
                        if neighbor_is_water != is_water {
                            is_coast[center_idx] = true;
                            break;
                        }
                    }
                }
            }
        }
    }

    for (center_idx, center) in graph.centers.iter().enumerate() {
        // Ignore les fantômes
        if center.point.0 < 0.0 || center.point.0 > graph.width || 
           center.point.1 < 0.0 || center.point.1 > graph.height {
            continue;
        }

        // Trie les corners par angle autour du center
        let sorted_corners = sort_corners_by_angle(center, &center.corners, graph);
        
        // Calcule l'élévation moyenne du polygone (utilise l'élévation du center)
        let elevation = center.elevation;

        // Détermine la couleur
        let color = if use_grayscale {
            // Mode grayscale : normalise l'élévation en luminance
            let t = if max_elev > min_elev {
                ((elevation - min_elev) / (max_elev - min_elev)).max(0.0).min(1.0)
            } else {
                0.5 // Fallback si toutes les élévations sont identiques
            };
            let g = (t * 255.0) as u8;
            
            // Si c'est une côte, utilise la couleur bleue
            if is_coast[center_idx] {
                Rgb([coastline_color.0, coastline_color.1, coastline_color.2])
            } else {
                Rgb([g, g, g])
            }
        } else {
            // Mode couleur original
            if elevation < 0.0 {
                // Océan : Bleu foncé à bleu clair
                let t = (elevation + 1.0).max(0.0).min(1.0);
                let r = 0;
                let g = if elevation > -0.3 {
                    (100.0 + (elevation + 0.3) * 300.0) as u8
                } else {
                    (50.0 + t * 100.0) as u8
                };
                let b = if elevation > -0.3 {
                    (150.0 + (elevation + 0.3) * 350.0) as u8
                } else {
                    (100.0 + t * 155.0) as u8
                };
                Rgb([r, g.min(255), b.min(255)])
            } else {
                // Terre : Vert à brun/blanc selon l'élévation
                let t = elevation.min(2.0) / 2.0;
                
                if t < 0.3 {
                    // Plaine : Vert
                    let r = (50.0 + t * 50.0) as u8;
                    let g = (150.0 - t * 50.0) as u8;
                    let b = (50.0 + t * 30.0) as u8;
                    Rgb([r, g, b])
                } else if t < 0.7 {
                    // Collines : Vert-brun
                    let r = (100.0 + (t - 0.3) * 200.0) as u8;
                    let g = (100.0 + (t - 0.3) * 100.0) as u8;
                    let b = (80.0 + (t - 0.3) * 50.0) as u8;
                    Rgb([r, g, b])
                } else {
                    // Montagnes : Brun à blanc
                    let r = (200.0 + (t - 0.7) * 55.0) as u8;
                    let g = (200.0 + (t - 0.7) * 55.0) as u8;
                    let b = (130.0 + (t - 0.7) * 125.0) as u8;
                    Rgb([r, g, b])
                }
            }
        };

        let mut points: Vec<ImgPoint<i32>> = sorted_corners.iter()
            .map(|&c_idx| {
                let c = &graph.corners[c_idx];
                ImgPoint::new(c.point.0 as i32, c.point.1 as i32)
            })
            .collect();
        
        // Supprime les doublons consécutifs
        if points.len() > 1 {
            points.dedup();
        }
        
        // Supprime le dernier point s'il est identique au premier
        if points.len() > 2 && points[0] == points[points.len() - 1] {
            points.pop();
        }

        if points.len() >= 3 {
            draw_polygon_mut(&mut img, &points, color);
        }
    }
    
    img.save(filename).unwrap();
}