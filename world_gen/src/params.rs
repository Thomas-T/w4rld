/// Paramètres centralisés de la simulation
#[derive(Debug, Clone)]
pub struct SimParams {
    // Tectonique
    pub tectonic_cycles: usize,
    pub dt: f64,
    pub plate_speed_min: f64,
    pub plate_speed_max: f64,
    pub uplift_k: f64,          // facteur global uplift
    pub subsidence_k: f64,      // facteur global subsidence
    pub ridge_k: f64,           // divergence (ridge/rift) amplitude
    pub trench_k: f64,          // trench (subduction côté océan)
    pub smoothing_k: f64,       // intensité lissage par step
    pub smoothing_radius: f64,  // rayon de lissage (falloff)
    pub smoothing_falloff: f32,      // <-- optionnel (1.0 = linéaire simple)
    // Orogeny dynamics
    pub compression_threshold: f64,  // seuil de compression pour activer core uplift
    pub max_orogeny_age: f64,        // durée de vie maximale d'une orogenèse (cycles)
    pub fatigue_tau: f64,            // constante de temps pour la fatigue tectonique
    pub core_uplift_cap: f64,        // limite de sécurité sur l'uplift par cycle
    pub continent_threshold: f64,    // seuil d'élévation pour considérer un center comme continental
    pub core_mult: f64,               // multiplicateur pour le core uplift orogénique
    // Erosion thermique
    pub thermal_passes_per_cycle: usize,
    pub talus: f64,             // seuil de différence d'élévation
    pub thermal_k: f64,         // taux de transfert
    
    // Sea level
    pub land_ratio: f64,        // ex 0.40
    
    // Hydrologie / pluie
    pub rain_cycles: usize,
    pub rain_amount: f64,       // précip globale par cycle
    pub moisture_decay: f64,    // k dans exp(-dist/k)
    pub ocean_distance_unit: f64, // distance ajoutée par edge en BFS (ex 1.0)
    pub river_erosion_k: f64,
    pub deposition_k: f64,
    
    // Debug visuel
    pub debug_grayscale: bool,           // Mode grayscale pour la heightmap
    pub coastline_blue_rgb: (u8, u8, u8), // Couleur du liseré côtier
    pub coastline_width: f64,             // Largeur du liseré (pour référence future)
}

impl Default for SimParams {
    fn default() -> Self {
        Self {

           // --- Tectonique ---
            // 20–60 cycles donnent généralement des continents bien structurés sans sur-amplification
            tectonic_cycles: 40,
            // dt pour garder stabilité quand tu ajustes cycles
            dt: 1.0,

            // vitesses faibles (important !) : tu veux des forces modestes par step
            // (avec dt appliqué partout, tu peux monter un peu, mais commence bas)
            plate_speed_min: 0.05,
            plate_speed_max: 0.25,

            // amplitudes (à ajuster selon ton échelle d'élévation)
            // but : produire ~[-1.5, +1.5] avant sea level (ordre de grandeur)
            uplift_k: 0.35,       // montagnes en convergence (continent-continent)
            subsidence_k: 0.12,   // subsidence (plutôt océanique)
            ridge_k: 0.18,        // dorsales en divergence (relief doux)
            trench_k: 0.25,       // fosses (subduction) plus marquées que ridge

            // relaxation tectonique légère (diffusion sur deltas, pas sur élévations)
            smoothing_k: 0.25,
            smoothing_radius: 2.0,
            // falloff : >1 rend la courbe plus "centrée près de la frontière"
            smoothing_falloff: 1.6,
            
            // --- Orogeny dynamics ---
            compression_threshold: 0.55,  // seuil pour activer core uplift (réduit le nombre de zones actives)
            max_orogeny_age: 15.0,       // durée de vie maximale d'une orogenèse (cycles)
            fatigue_tau: 7.0,           // constante de temps pour la fatigue (cycles)
            core_uplift_cap: 0.12,        // limite de sécurité sur l'uplift par cycle
            continent_threshold: -0.05,  // seuil d'élévation pour considérer un center comme continental
            core_mult: 4.0,               // multiplicateur pour le core uplift orogénique

            // --- Thermal erosion ---
            // 2–4 passes par cycle suffisent souvent
            thermal_passes_per_cycle: 3,
            // talus : plus haut => montagnes plus “raides” (moins d'éboulis)
            talus: 0.55,
            // quantité de matière transférée quand pente > talus
            thermal_k: 0.25,

            // --- Sea level ---
            land_ratio: 0.40,

            // --- Hydrologie ---
            // 20–80 cycles selon ton algo ; commence modéré
            rain_cycles: 40,
            // pluie globale par cycle (peut être faible si ton erosion est forte)
            rain_amount: 0.06,

            // distance à l’océan (BFS hops) -> humidité décroît
            // plus grand = continents plus humides loin des côtes
            moisture_decay: 18.0,
            ocean_distance_unit: 1.0,

            // érosion fluviale : commence bas sinon tu vas tout raviner très vite
            river_erosion_k: 0.015,
            // dépôt : stabilise les plaines / deltas
            deposition_k: 0.010,
            
            // --- Debug visuel ---
            debug_grayscale: true,
            coastline_blue_rgb: (0, 120, 255),
            coastline_width: 1.0,

        }
    }
}

