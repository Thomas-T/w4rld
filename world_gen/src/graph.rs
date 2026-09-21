#[derive(Debug, Clone)]
pub struct Center {
    pub index: usize,
    pub point: (f64, f64),  // Position X,Y
    pub corners: Vec<usize>, // Coins du polygone
    pub neighbors: Vec<usize>, // Autres Centers voisins
    pub borders: Vec<usize>,   // Arêtes frontières
    
    // Données géologiques
    pub plate_id: usize,     // À quelle plaque tectonique j'appartiens
    pub is_ghost: bool,       // Point fantôme (hors carte, utilisé pour borner le Voronoï)
    pub elevation: f64,       // Élévation du terrain
    pub is_oceanic: bool,     // Type de plaque (océanique ou continentale)
    pub moisture: f64,        // Humidité (pour la simulation)
    pub orogeny_age: f64,     // Âge de l'orogenèse active (0 = inactive, >0 = active)
}

#[derive(Debug, Clone)]
pub struct Corner {
    pub index: usize,
    pub point: (f64, f64),
    pub touches: Vec<usize>, // Centers touchés
    pub adjacent: Vec<usize>, // Corners voisins
    pub elevation: f64,       // Élévation du terrain
    pub downslope: Option<usize>, // Corner vers lequel l'eau s'écoule
    pub river_flow: f64,      // Quantité d'eau qui passe par ce corner
    pub moisture: f64,        // Humidité (pour la simulation)
}

#[derive(Debug, Clone)]
pub struct Edge {
    pub index: usize,
    pub d0: usize, pub d1: usize, // Centers connectés (Delaunay)
    pub v0: usize, pub v1: usize, // Corners connectés (Voronoï)
}

pub struct WorldGraph {
    pub width: f64,
    pub height: f64,
    pub centers: Vec<Center>,
    pub corners: Vec<Corner>,
    pub edges: Vec<Edge>,
}

impl WorldGraph {
    pub fn new(width: f64, height: f64) -> Self {
        Self { 
            width, 
            height, 
            centers: vec![], 
            corners: vec![], 
            edges: vec![] 
        }
    }
}

