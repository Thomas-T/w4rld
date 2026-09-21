use std::collections::HashMap;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;
use image::Rgba;

/// Crée un GIF animé à partir d'une liste de fichiers PNG
pub fn create_gif_from_pngs(png_files: &[String], output_path: &str, delay: u16) -> Result<(), Box<dyn std::error::Error>> {
    let mut frames = Vec::new();
    
    for png_file in png_files {
        if !Path::new(png_file).exists() {
            eprintln!("Avertissement: fichier {} introuvable, ignoré", png_file);
            continue;
        }
        
        let mut img = image::open(png_file)?.to_rgba8();
        let (width, height) = img.dimensions();
        
        // Extrait le nom du fichier (sans le chemin)
        let filename = Path::new(png_file)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(png_file);
        
        // Dessine un rectangle semi-transparent pour le fond du texte (en bas à gauche)
        let text_y = (height as i32) - 20; // 20 pixels du bas
        let text_x = 10; // 10 pixels de la gauche
        let text_width = (filename.len() * 7).min(500); // Estimation de la largeur
        
        // Dessine un rectangle semi-transparent pour le fond
        for y in text_y..(height as i32) {
            for x in text_x..(text_x + text_width as i32) {
                if x >= 0 && x < width as i32 && y >= 0 && y < height as i32 {
                    let pixel = img.get_pixel(x as u32, y as u32);
                    // Mélange avec un fond noir semi-transparent
                    let new_pixel = Rgba([
                        (pixel[0] as f32 * 0.3) as u8,
                        (pixel[1] as f32 * 0.3) as u8,
                        (pixel[2] as f32 * 0.3) as u8,
                        255,
                    ]);
                    img.put_pixel(x as u32, y as u32, new_pixel);
                }
            }
        }
        
        // Dessine le texte en blanc
        draw_text_on_image(&mut img, filename, text_x, text_y, Rgba([255, 255, 255, 255]));
        
        // Convertit en format GIF (palette de couleurs)
        // from_rgba nécessite un slice mutable, on doit convertir le buffer
        let mut rgba_data = img.into_raw();
        // Épisode 2 : `from_rgba` quantifie la palette à la vitesse 1 (la plus lente, ~1 s par
        // image 1000×1000) : c'était 98 des 103 secondes du programme. À 10, la différence visuelle
        // est imperceptible sur ces rendus et l'encodage passe sous les 10 secondes.
        let mut frame = gif::Frame::from_rgba_speed(width as u16, height as u16, &mut rgba_data, 10);
        frame.delay = delay;
        frames.push(frame);
    }
    
    if frames.is_empty() {
        return Err("Aucune frame valide trouvée".into());
    }
    
    let file = File::create(output_path)?;
    let mut encoder = gif::Encoder::new(BufWriter::new(file), frames[0].width, frames[0].height, &[])?;
    encoder.set_repeat(gif::Repeat::Infinite)?;
    
    for frame in frames {
        encoder.write_frame(&frame)?;
    }
    
    Ok(())
}

/// Dessine du texte simple sur une image (sans vraie police, approche bitmap)
fn draw_text_on_image(img: &mut image::RgbaImage, text: &str, x: i32, y: i32, color: Rgba<u8>) {
    // Police bitmap simple 5x7 pixels par caractère
    let char_width = 6;
    let _char_height = 7;
    let mut current_x = x;
    
    // Table de caractères bitmap simple (5x7)
    let font_data = create_simple_font();
    
    // Caractère par défaut (rectangle)
    let default_char = [
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
    ];
    
    for ch in text.chars() {
        if current_x + char_width > img.width() as i32 {
            break;
        }
        
        let glyph = font_data.get(&ch.to_ascii_uppercase())
            .unwrap_or(&default_char);
        
        for (row_idx, row) in glyph.iter().enumerate() {
            for (col_idx, &pixel) in row.iter().enumerate() {
                if pixel == 1 {
                    let px = current_x + col_idx as i32;
                    let py = y + row_idx as i32;
                    if px >= 0 && px < img.width() as i32 && py >= 0 && py < img.height() as i32 {
                        img.put_pixel(px as u32, py as u32, color);
                    }
                }
            }
        }
        
        current_x += char_width;
    }
}

/// Crée une table de caractères bitmap simple
fn create_simple_font() -> HashMap<char, [[u8; 5]; 7]> {
    let mut font = HashMap::new();
    
    // Définit quelques caractères de base (A-Z, 0-9, quelques symboles)
    // Format: chaque ligne est un tableau de 5 bits (0 ou 1)
    
    // 'A'
    font.insert('A', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'B'
    font.insert('B', [
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 0],
    ]);
    
    // 'C'
    font.insert('C', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // 'D'
    font.insert('D', [
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 0],
    ]);
    
    // 'E'
    font.insert('E', [
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
    ]);
    
    // 'F'
    font.insert('F', [
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
    ]);
    
    // 'G'
    font.insert('G', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 0],
        [1, 0, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // 'H'
    font.insert('H', [
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'I'
    font.insert('I', [
        [1, 1, 1, 1, 1],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [1, 1, 1, 1, 1],
    ]);
    
    // 'J'
    font.insert('J', [
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // 'K'
    font.insert('K', [
        [1, 0, 0, 0, 1],
        [1, 0, 0, 1, 0],
        [1, 0, 1, 0, 0],
        [1, 1, 0, 0, 0],
        [1, 0, 1, 0, 0],
        [1, 0, 0, 1, 0],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'L'
    font.insert('L', [
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
    ]);
    
    // 'M'
    font.insert('M', [
        [1, 0, 0, 0, 1],
        [1, 1, 0, 1, 1],
        [1, 0, 1, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'N'
    font.insert('N', [
        [1, 0, 0, 0, 1],
        [1, 1, 0, 0, 1],
        [1, 0, 1, 0, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'O'
    font.insert('O', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // 'P'
    font.insert('P', [
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
    ]);
    
    // 'Q'
    font.insert('Q', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 1, 0, 1],
        [1, 0, 0, 1, 0],
        [0, 1, 1, 0, 1],
    ]);
    
    // 'R'
    font.insert('R', [
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 1, 1, 1, 0],
        [1, 0, 1, 0, 0],
        [1, 0, 0, 1, 0],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'S'
    font.insert('S', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 0],
        [0, 1, 1, 1, 0],
        [0, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // 'T'
    font.insert('T', [
        [1, 1, 1, 1, 1],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
    ]);
    
    // 'U'
    font.insert('U', [
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // 'V'
    font.insert('V', [
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 0, 1, 0],
        [0, 1, 0, 1, 0],
        [0, 0, 1, 0, 0],
    ]);
    
    // 'W'
    font.insert('W', [
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [1, 0, 1, 0, 1],
        [1, 1, 0, 1, 1],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'X'
    font.insert('X', [
        [1, 0, 0, 0, 1],
        [0, 1, 0, 1, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 1, 0, 1, 0],
        [1, 0, 0, 0, 1],
    ]);
    
    // 'Y'
    font.insert('Y', [
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 0, 1, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
    ]);
    
    // 'Z'
    font.insert('Z', [
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 1, 0],
        [0, 0, 1, 0, 0],
        [0, 1, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
    ]);
    
    // Chiffres
    // '0'
    font.insert('0', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 1, 1],
        [1, 0, 1, 0, 1],
        [1, 1, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // '1'
    font.insert('1', [
        [0, 0, 1, 0, 0],
        [0, 1, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 0, 1, 0, 0],
        [0, 1, 1, 1, 0],
    ]);
    
    // '2'
    font.insert('2', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 1, 1, 0],
        [0, 1, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
    ]);
    
    // '3'
    font.insert('3', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
        [0, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // '4'
    font.insert('4', [
        [0, 0, 0, 1, 0],
        [0, 0, 1, 1, 0],
        [0, 1, 0, 1, 0],
        [1, 0, 0, 1, 0],
        [1, 1, 1, 1, 1],
        [0, 0, 0, 1, 0],
        [0, 0, 0, 1, 0],
    ]);
    
    // '5'
    font.insert('5', [
        [1, 1, 1, 1, 1],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 0],
        [0, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // '6'
    font.insert('6', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // '7'
    font.insert('7', [
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 1, 0],
        [0, 0, 1, 0, 0],
        [0, 1, 0, 0, 0],
        [1, 0, 0, 0, 0],
        [1, 0, 0, 0, 0],
    ]);
    
    // '8'
    font.insert('8', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // '9'
    font.insert('9', [
        [0, 1, 1, 1, 0],
        [1, 0, 0, 0, 1],
        [1, 0, 0, 0, 1],
        [0, 1, 1, 1, 1],
        [0, 0, 0, 0, 1],
        [0, 0, 0, 0, 1],
        [0, 1, 1, 1, 0],
    ]);
    
    // Symboles
    // '.'
    font.insert('.', [
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 1, 0, 0],
    ]);
    
    // '_'
    font.insert('_', [
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
    ]);
    
    // '-'
    font.insert('-', [
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [1, 1, 1, 1, 1],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
        [0, 0, 0, 0, 0],
    ]);
    
    font
}

