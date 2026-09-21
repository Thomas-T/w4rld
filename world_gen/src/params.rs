/// Paramètres centralisés de la simulation.
///
/// Version 2 (branche fix/tectonique) : ajout du monde (taille, points, plaques), de l'isostasie,
/// d'un seuil d'orogenèse exprimé en fraction de la compression maximale, et d'un mode `legacy`
/// qui reproduit exactement les défauts décrits dans l'épisode 1 du blog, pour comparaison.
#[derive(Debug, Clone)]
pub struct SimParams {
    // --- Monde ---
    pub width: f64,
    pub height: f64,
    pub num_points: usize,
    pub num_plates: usize,

    // --- Tectonique ---
    pub tectonic_cycles: usize,
    pub dt: f64,
    pub plate_speed_min: f64,
    pub plate_speed_max: f64,
    pub uplift_k: f64,       // surrection diffuse en convergence
    pub subsidence_k: f64,   // affaissement
    pub ridge_k: f64,        // dorsale (divergence océanique)
    pub trench_k: f64,       // fosse (subduction, côté océan)
    pub smoothing_k: f64,    // intensité de la diffusion des deltas
    pub smoothing_radius: f64, // nombre de passes de diffusion (arrondi, max 3)

    // --- Orogenèse « de cœur » ---
    /// Seuil d'activation, en FRACTION de la compression maximale possible (2 × plate_speed_max).
    /// Épisode 1 : ce seuil était absolu (0,55) alors que la compression ne dépasse jamais 0,5,
    /// donc l'orogenèse de cœur ne se déclenchait jamais. En mode `legacy`, il redevient absolu.
    pub compression_threshold: f64,
    pub max_orogeny_age: f64,   // durée de vie d'une orogenèse (cycles)
    pub fatigue_tau: f64,       // constante de temps de la fatigue (cycles)
    pub core_uplift_cap: f64,   // plafond de surrection de cœur par cycle
    pub continent_threshold: f64, // élévation au-dessus de laquelle un centre est « continental »
    pub core_mult: f64,         // multiplicateur de la surrection de cœur

    // --- Isostasie (nouveau) ---
    /// Rappel de l'élévation vers le niveau de base de la plaque, par cycle. 0 = désactivé
    /// (comportement de l'épisode 1 : le relief monte en ligne droite sans jamais saturer).
    pub isostasy_k: f64,
    pub base_oceanic: f64,
    pub base_continental: f64,

    // --- Érosion thermique ---
    pub thermal_passes_per_cycle: usize,
    pub talus: f64,
    pub thermal_k: f64,

    // --- Niveau de la mer ---
    pub land_ratio: f64,

    // --- Hydrologie ---
    pub rain_cycles: usize,
    pub rain_amount: f64,
    pub moisture_decay: f64,
    pub ocean_distance_unit: f64,
    pub river_erosion_k: f64,
    pub deposition_k: f64,

    // --- Rendu ---
    pub grayscale: bool,                  // niveaux de gris + liseré côtier (mode debug)
    pub coastline_blue_rgb: (u8, u8, u8),

    // --- Compatibilité ---
    /// Reproduit les trois défauts mesurables de l'épisode 1 : seuil d'orogenèse absolu,
    /// érosion appliquée sous l'eau (qui remonte le fond marin à 0,0), pas d'isostasie.
    pub legacy: bool,
}

impl Default for SimParams {
    fn default() -> Self {
        Self {
            width: 1000.0,
            height: 1000.0,
            num_points: 10_000,
            num_plates: 50,

            tectonic_cycles: 40,
            dt: 1.0,
            plate_speed_min: 0.05,
            plate_speed_max: 0.25,
            uplift_k: 0.35,
            subsidence_k: 0.12,
            ridge_k: 0.18,
            trench_k: 0.25,
            smoothing_k: 0.25,
            smoothing_radius: 2.0,

            compression_threshold: 0.55, // fraction : 0,55 × (2 × 0,25) = 0,275 en absolu
            max_orogeny_age: 15.0,
            fatigue_tau: 7.0,
            core_uplift_cap: 0.12,
            continent_threshold: -0.05,
            core_mult: 4.0,

            isostasy_k: 0.06, // temps caractéristique ≈ 17 cycles : le relief sature avant le 40e
            base_oceanic: -0.5,
            base_continental: 0.1,

            thermal_passes_per_cycle: 3,
            talus: 0.55,
            thermal_k: 0.25,

            land_ratio: 0.40,

            rain_cycles: 40,
            rain_amount: 0.06,
            moisture_decay: 18.0,
            ocean_distance_unit: 1.0,
            river_erosion_k: 0.015,
            deposition_k: 0.010,

            grayscale: false,
            coastline_blue_rgb: (0, 120, 255),

            legacy: false,
        }
    }
}

impl SimParams {
    /// Paramètres qui reproduisent le comportement de l'épisode 1 (tag `blog/jeux-simulation-1`).
    pub fn legacy() -> Self {
        Self { legacy: true, isostasy_k: 0.0, ..Self::default() }
    }

    /// Seuil absolu d'activation de l'orogenèse de cœur.
    pub fn core_threshold_abs(&self) -> f64 {
        if self.legacy {
            self.compression_threshold
        } else {
            self.compression_threshold * 2.0 * self.plate_speed_max
        }
    }
}
