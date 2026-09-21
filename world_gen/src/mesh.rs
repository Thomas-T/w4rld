use crate::graph::{Center, Corner, Edge, WorldGraph};
use delaunator::{triangulate, Point};
use rand::Rng;

/// Construit un graphe Voronoï à partir d'un maillage de Delaunay
pub fn build_graph(width: f64, height: f64, num_points: usize, rng: &mut impl Rng) -> WorldGraph {
    // Génère des points aléatoires (générateur injecté : même seed, même monde)
    let mut points: Vec<Point> = Vec::new();
    
    for _ in 0..num_points {
        let x = rng.gen_range(0.0..width);
        let y = rng.gen_range(0.0..height);
        points.push(Point { x, y });
    }

    // 2. AJOUT DES POINTS FANTÔMES (C'est le secret !)
    // On les place très loin pour qu'ils n'interfèrent pas avec le centre,
    // mais qu'ils ferment proprement les polygones des bords.
    let margin = width.max(height) * 5.0; 
    points.push(Point { x: -margin, y: -margin });             // Haut-Gauche
    points.push(Point { x: width + margin, y: -margin });      // Haut-Droite
    points.push(Point { x: width + margin, y: height + margin }); // Bas-Droite
    points.push(Point { x: -margin, y: height + margin });     // Bas-Gauche    
    
    // Triangulation de Delaunay
    let triangulation = triangulate(&points);
    
    // Convertit en notre structure WorldGraph
    let mut graph = WorldGraph::new(width, height);
    
    // Les points de Delaunay deviennent les Centers
    for (i, point) in points.iter().enumerate() {
        // Les 4 derniers points sont les points fantômes (marges)
        let is_ghost = i >= num_points;
        graph.centers.push(Center {
            point: (point.x, point.y),
            corners: vec![],
            neighbors: vec![],
            plate_id: 0,
            is_ghost,
            elevation: 0.0,
            is_oceanic: false,
            moisture: 0.0,
            orogeny_age: 0.0,
            base_level: 0.0,
        });
    }
    
    // Construit les Corners (centres circonscrits des triangles)
    let triangles = &triangulation.triangles;
    let num_triangles = triangles.len() / 3;
    
    for i in 0..num_triangles {
        let i0 = triangles[i * 3];
        let i1 = triangles[i * 3 + 1];
        let i2 = triangles[i * 3 + 2];
        
        let p0 = &points[i0];
        let p1 = &points[i1];
        let p2 = &points[i2];
        
        // Calcule le centre circonscrit du triangle
        let circumcenter = circumcenter(p0, p1, p2);
        
        let corner_index = graph.corners.len();
        graph.corners.push(Corner {
            point: (circumcenter.x, circumcenter.y),
            touches: vec![i0, i1, i2],
            adjacent: vec![],
            elevation: 0.0,
            downslope: None,
            river_flow: 0.0,
            moisture: 0.0,
        });
        
        // Ajoute ce corner aux centers correspondants
        graph.centers[i0].corners.push(corner_index);
        graph.centers[i1].corners.push(corner_index);
        graph.centers[i2].corners.push(corner_index);
    }
    
    // Construit les Edges et les relations neighbors
    let halfedges = &triangulation.halfedges;
    
    for (edge_idx, &halfedge) in halfedges.iter().enumerate() {
        if halfedge == delaunator::EMPTY || edge_idx > halfedge {
            continue; // On traite chaque arête une seule fois
        }
        
        let start = edge_idx;
        let end = halfedge;
        
        // Trouve les triangles adjacents
        let tri_start = start / 3;
        let tri_end = end / 3;
        
        // Les centers connectés sont les deux extrémités de l'arête correspondant au half-edge
        // Le half-edge start pointe vers le sommet triangles[start]
        // Le half-edge suivant pointe vers triangles[next_halfedge(start)]
        let d0 = triangles[start];
        let d1 = triangles[next_halfedge(start)];
        
        // Les corners connectés sont les centres circonscrits des triangles
        let v0 = tri_start;
        let v1 = tri_end;
        
        graph.edges.push(Edge {
            d0,
            d1,
            v0,
            v1,
        });
        
        // Ajoute les relations neighbors
        if !graph.centers[d0].neighbors.contains(&d1) {
            graph.centers[d0].neighbors.push(d1);
        }
        if !graph.centers[d1].neighbors.contains(&d0) {
            graph.centers[d1].neighbors.push(d0);
        }
    }
    
    // Construit les relations adjacent pour les corners
    for edge in &graph.edges {
        if !graph.corners[edge.v0].adjacent.contains(&edge.v1) {
            graph.corners[edge.v0].adjacent.push(edge.v1);
        }
        if !graph.corners[edge.v1].adjacent.contains(&edge.v0) {
            graph.corners[edge.v1].adjacent.push(edge.v0);
        }
    }
    
    graph
}

/// Retourne le half-edge suivant dans le même triangle
fn next_halfedge(e: usize) -> usize {
    if e % 3 == 2 { e - 2 } else { e + 1 }
}

/// Calcule le centre circonscrit d'un triangle
fn circumcenter(p0: &Point, p1: &Point, p2: &Point) -> Point {
    let d = 2.0 * (p0.x * (p1.y - p2.y) + p1.x * (p2.y - p0.y) + p2.x * (p0.y - p1.y));
    
    let ux = ((p0.x * p0.x + p0.y * p0.y) * (p1.y - p2.y) +
              (p1.x * p1.x + p1.y * p1.y) * (p2.y - p0.y) +
              (p2.x * p2.x + p2.y * p2.y) * (p0.y - p1.y)) / d;
    
    let uy = ((p0.x * p0.x + p0.y * p0.y) * (p2.x - p1.x) +
              (p1.x * p1.x + p1.y * p1.y) * (p0.x - p2.x) +
              (p2.x * p2.x + p2.y * p2.y) * (p1.x - p0.x)) / d;
    
    Point { x: ux, y: uy }
}

