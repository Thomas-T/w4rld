//! w4rld : génération procédurale de monde (Voronoï, plaques, tectonique, pluie, érosion).
//!
//! La bibliothèque expose le pipeline complet (`pipeline::run`) pour le binaire et les tests ;
//! le rendu en images est délégué à un callback, ce qui permet de simuler sans dessiner.

pub mod drawer;
pub mod gif_export;
pub mod graph;
pub mod hydrology;
pub mod mesh;
pub mod params;
pub mod pipeline;
pub mod tectonics;
