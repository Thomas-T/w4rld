use std::fs;
use std::path::PathBuf;

use world_gen::drawer::{draw_elevation_with_params, draw_mesh, draw_plates};
use world_gen::gif_export::create_gif_from_pngs;
use world_gen::params::SimParams;
use world_gen::pipeline::{run, CycleStats};

const USAGE: &str = "\
w4rld — génération procédurale de monde

USAGE : world_gen [OPTIONS]

OPTIONS
  --seed N            graine du générateur (défaut : 2026). Même seed = même monde.
  --out DIR           dossier de sortie (défaut : out/)
  --legacy            reproduit les défauts de l'épisode 1 (seuil absolu, érosion sous l'eau, pas d'isostasie)
  --grayscale         rendu debug en niveaux de gris avec liseré côtier
  --points N          nombre de cellules (défaut : 10000)
  --plates N          nombre de plaques (défaut : 50)
  --tectonic-cycles N (défaut : 40)
  --rain-cycles N     (défaut : 40)
  --no-images         simule sans dessiner (mesure de performance)
  --no-gif            dessine les PNG mais pas le GIF
  -h, --help
";

struct Cli {
    seed: u64,
    out: PathBuf,
    params: SimParams,
    images: bool,
    gif: bool,
}

fn parse_cli() -> Cli {
    let mut cli = Cli { seed: 2026, out: PathBuf::from("out"), params: SimParams::default(), images: true, gif: true };
    let mut legacy = false;
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    let next = |i: &mut usize, flag: &str| -> String {
        *i += 1;
        args.get(*i).cloned().unwrap_or_else(|| {
            eprintln!("option {} : valeur manquante", flag);
            std::process::exit(2)
        })
    };
    while i < args.len() {
        match args[i].as_str() {
            "--seed" => cli.seed = next(&mut i, "--seed").parse().expect("--seed attend un entier"),
            "--out" => cli.out = PathBuf::from(next(&mut i, "--out")),
            "--legacy" => legacy = true,
            "--grayscale" => cli.params.grayscale = true,
            "--points" => cli.params.num_points = next(&mut i, "--points").parse().expect("--points attend un entier"),
            "--plates" => cli.params.num_plates = next(&mut i, "--plates").parse().expect("--plates attend un entier"),
            "--tectonic-cycles" => cli.params.tectonic_cycles = next(&mut i, "--tectonic-cycles").parse().expect("entier attendu"),
            "--rain-cycles" => cli.params.rain_cycles = next(&mut i, "--rain-cycles").parse().expect("entier attendu"),
            "--no-images" => cli.images = false,
            "--no-gif" => cli.gif = false,
            "-h" | "--help" => {
                print!("{}", USAGE);
                std::process::exit(0)
            }
            other => {
                eprintln!("option inconnue : {}\n\n{}", other, USAGE);
                std::process::exit(2)
            }
        }
        i += 1;
    }
    if legacy {
        let grayscale = cli.params.grayscale;
        let mut p = SimParams::legacy();
        p.grayscale = grayscale;
        p.num_points = cli.params.num_points;
        p.num_plates = cli.params.num_plates;
        p.tectonic_cycles = cli.params.tectonic_cycles;
        p.rain_cycles = cli.params.rain_cycles;
        cli.params = p;
    }
    cli
}

fn main() {
    let cli = parse_cli();
    fs::create_dir_all(&cli.out).expect("impossible de créer le dossier de sortie");
    let params = cli.params.clone();

    println!(
        "w4rld — seed {} · {} cellules · {} plaques · {} cycles tectoniques · {} cycles de pluie{}",
        cli.seed, params.num_points, params.num_plates, params.tectonic_cycles, params.rain_cycles,
        if params.legacy { " · MODE LEGACY (défauts de l'épisode 1)" } else { "" }
    );
    println!(
        "seuil d'orogenèse de cœur : {:.3} (compression max possible {:.3}) · isostasie k = {}",
        params.core_threshold_abs(), 2.0 * params.plate_speed_max, params.isostasy_k
    );

    let mut png_files: Vec<String> = Vec::new();
    let out_dir = cli.out.clone();
    let images = cli.images;
    let render_params = params.clone();
    let outcome = run(&params, cli.seed, |name, graph, plates| {
        if !images {
            return;
        }
        let path = out_dir.join(format!("{}.png", name));
        let path_str = path.to_string_lossy().to_string();
        match name {
            "step1_mesh" => draw_mesh(graph, &path_str),
            "step2_plates" => draw_plates(graph, plates, &path_str),
            _ => draw_elevation_with_params(graph, &path_str, Some(&render_params), 0.0),
        }
        png_files.push(path_str);
    });

    // Log lisible, une ligne par étape.
    for s in &outcome.stats {
        match s.phase {
            "tectonic" => println!(
                "  tecto {:>2} : min {:+.3} max {:+.3} moy {:+.3} σ {:.3} | conv {} div {} cœur {}",
                s.cycle, s.min, s.max, s.mean, s.std, s.convergent, s.divergent, s.core_edges
            ),
            "hydro" => println!(
                "  pluie {:>2} : min {:+.3} max {:+.3} | coins à 0,0 : {} | sous l'eau : {} | terres {:.1} %",
                s.cycle, s.min, s.max, s.zero_corners, s.underwater_corners, 100.0 * s.land_ratio
            ),
            other => println!(
                "  {:<9} : min {:+.3} max {:+.3} moy {:+.3} σ {:.3} | terres {:.1} %",
                other, s.min, s.max, s.mean, s.std, 100.0 * s.land_ratio
            ),
        }
    }

    // CSV des statistiques.
    let csv_path = cli.out.join("stats.csv");
    let mut csv = String::from(CycleStats::CSV_HEADER);
    for s in &outcome.stats {
        csv.push('\n');
        csv.push_str(&s.csv_row());
    }
    fs::write(&csv_path, csv).expect("écriture de stats.csv");

    let mut gif_secs = 0.0;
    if cli.images && cli.gif {
        let t = std::time::Instant::now();
        let gif_path = cli.out.join("world_evolution.gif").to_string_lossy().to_string();
        match create_gif_from_pngs(&png_files, &gif_path, 10) {
            Ok(_) => println!("GIF : {}", gif_path),
            Err(e) => eprintln!("GIF : erreur {}", e),
        }
        gif_secs = t.elapsed().as_secs_f64();
    }

    let t = &outcome.timings;
    println!();
    println!("empreinte  : {:016x}", outcome.fingerprint);
    println!(
        "temps      : maillage {:.2}s · plaques {:.2}s · tectonique {:.2}s · hydrologie {:.2}s · rendu PNG {:.2}s · GIF {:.2}s · total {:.2}s",
        t.mesh.as_secs_f64(), t.plates.as_secs_f64(), t.tectonics.as_secs_f64(),
        t.hydrology.as_secs_f64(), t.render.as_secs_f64(), gif_secs, t.total.as_secs_f64() + gif_secs
    );
    let last = outcome.stats.last().expect("au moins une mesure");
    println!(
        "final      : terres {:.1} % · coins à 0,0 : {} · sous l'eau : {} · {} PNG dans {} · stats.csv",
        100.0 * last.land_ratio, last.zero_corners, last.underwater_corners, png_files.len(), cli.out.display()
    );
}
