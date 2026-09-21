# w4rld

Génération procédurale de monde en Rust : maillage de Voronoï, plaques tectoniques, surrection et
subsidence, isostasie, érosion thermique, niveau de la mer, pluie avec effet orographique, érosion
hydraulique, et un GIF animé de toute l'évolution.

![Évolution du monde en 85 étapes](docs/evolution-500px.gif)

Projet du soir, écrit en deux jours (janvier 2026), pour pratiquer Rust sur un vrai sujet et retrouver
le plaisir des mondes générés à la Dwarf Fortress. Il est raconté sur [Binary Imothep](https://medium.com/binary-imothep),
série *Jeux & simulation* :

- **Épisode 1** — « Générer un monde en 2 254 lignes de Rust » : la version d'origine, avec ses défauts.
  Figée au tag [`blog/jeux-simulation-1`](../../tree/blog/jeux-simulation-1).
- **Épisode 2** — la réparation : ce que contient `main` aujourd'hui (voir « Ce qui a été corrigé »).

## Le pipeline

1. **Maillage** : N points aléatoires + 4 points fantômes lointains → Delaunay (`delaunator`) → cellules de Voronoï.
2. **Plaques** : graines espacées (maximisation de la distance minimale) → parcours en largeur à sources multiples. 60 % de plaques océaniques.
3. **Élévation de base** : −0,5 (océan) / +0,1 (continent).
4. **Tectonique** (40 cycles) : vitesse relative projetée sur la normale de chaque arête inter-plaques → compression (surrection diffuse, surrection de cœur continent-continent avec fatigue, subduction, affaissement) ou extension (dorsale, rift). Deltas diffusés puis appliqués, **rappel isostatique** vers le niveau de base de la plaque, 3 passes d'érosion thermique.
5. **Niveau de la mer** : quantile pour obtenir 40 % de terres émergées.
6. **Pluie et érosion hydraulique** (40 cycles) : distance à l'océan, humidité exponentielle, bonus orographique (vent ouest → est), écoulement vers le plus bas voisin, érosion des terres émergées et dépôt.
7. **GIF** : une image par étape, légende en police bitmap 5 × 7 maison.

Tous les paramètres sont dans [`world_gen/src/params.rs`](world_gen/src/params.rs) ; le pipeline est dans
[`world_gen/src/pipeline.rs`](world_gen/src/pipeline.rs) et se simule sans dessiner.

## Lancer

```bash
cargo build --release
./target/release/world_gen --seed 2026            # sorties dans out/
./target/release/world_gen --seed 2026 --legacy --out out-legacy   # les défauts de l'épisode 1, même monde
./target/release/world_gen --no-images            # simulation seule, pour mesurer
./target/release/world_gen --help
```

Même seed = même monde : le programme imprime une **empreinte** (FNV-1a des élévations finales) qui doit
être identique d'une exécution à l'autre. Sorties : 85 PNG, `world_evolution.gif` (~16 Mo), `stats.csv`
(une ligne par cycle : min, max, moyenne, écart-type, arêtes convergentes et divergentes, arêtes
d'orogenèse de cœur, coins à 0,0, coins sous l'eau, ratio de terres).

Sur un portable, en release, 10 000 cellules : **simulation 0,4 s**, rendu des PNG 4 s, GIF 12 s, soit
17 s au total. La version de l'épisode 1 mettait 97 s, dont 98 % dans la quantification des couleurs du GIF.

## Tests

```bash
cargo test --release
```

Huit tests d'intégration verrouillent les défauts de l'épisode 1 : même seed → même empreinte ; seeds
différents → mondes différents ; le niveau de la mer donne bien 40 % de terres ; l'érosion ne remonte
jamais le fond marin (et le fait en mode `legacy`) ; chaque plaque est connexe ; l'orogenèse de cœur se
déclenche puis s'éteint (et ne se déclenche jamais en mode `legacy`) ; le relief sature avec l'isostasie
(et continue de monter sans).

## Ce qui a été corrigé (épisode 2)

Même monde (seed 2026), à gauche la version de l'épisode 1, à droite la version corrigée :

![Comparaison épisode 1 / épisode 2](docs/comparaison-legacy-corrige.png)

1. **Seed et empreinte.** `thread_rng` remplacé par un `StdRng` injecté dans le maillage et les plaques ;
   empreinte FNV-1a imprimée et testée. Sans cela, aucune comparaison n'était possible.
2. **L'orogenèse de cœur se déclenche.** Le seuil de compression est désormais une fraction de la
   compression maximale possible (0,55 × 0,5 = 0,275) au lieu d'un absolu de 0,55 inatteignable.
   Le critère « continental » combine la nature de la plaque et l'élévation : sur l'élévation seule, des
   cellules océaniques soulevées par diffusion passaient pour des continents et l'orogenèse ne
   s'éteignait jamais. Mesuré : 8 arêtes actives pendant 16 cycles, puis 0.
3. **Le relief sature.** Rappel isostatique `−k (h − base)` par cycle, k = 0,06. L'altitude maximale
   culmine à 1,58 au cycle 12 puis redescend à 1,27 au cycle 40 quand l'orogenèse s'éteint ; sans
   isostasie elle montait encore (2,24 au cycle 40, seulement freinée par l'érosion thermique).
4. **L'érosion ne remonte plus le fond marin.** L'érosion fluviale ignore les coins immergés. Avant :
   3 311 coins exactement à 0,0 après la pluie et 8 461 sous l'eau ; après : 0 et 12 039.
5. **Le GIF en 12 s au lieu de 98.** `Frame::from_rgba` quantifie la palette à la vitesse 1 ; à 10, la
   différence est invisible sur ces rendus.
6. Nettoyage : crate `world_core` vide supprimée, champs et fonction morts retirés, boucle dupliquée
   dans la pluie retirée, sorties dans `out/`, rendu couleur par défaut, zéro avertissement.

Le mode `--legacy` conserve volontairement les défauts 2 à 4 pour pouvoir les mesurer sur le même monde.

## Note sur la fabrication

Ce code a été écrit avec un assistant IA, sous ma direction : je relis, je corrige, je demande des
modifications, mais je laisse l'IA produire le gros du travail. C'est ce qui me permet d'aller vite et
d'essayer des choses nouvelles, ce que j'adore faire. Les articles qui décrivent ce dépôt sont publiés sur
[Binary Imothep](https://medium.com/binary-imothep) avec la même note.

## Licence

MIT, voir [`LICENSE`](LICENSE).
