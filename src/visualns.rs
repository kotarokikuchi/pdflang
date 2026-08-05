//! Namespace `visual::` — images and visual comparison.
//! Visual analysis renders the pages in greyscale on demand; the result is
//! cached per (document, width).

use crate::interpreter::{DocData, ImageData, RuntimeError, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// A rendered page: (width, height, greyscale pixels).
type GrayPage = (u32, u32, Vec<u8>);

thread_local! {
    /// cache: path -> (page number -> render)
    static RENDERS: RefCell<HashMap<String, HashMap<i64, GrayPage>>> = RefCell::new(HashMap::new());
}

/// Default width for analysis renders (a cost/accuracy balance).
const ANALYSIS_WIDTH: u16 = 600;

fn render_page(path: &std::path::Path, page: i64) -> Result<GrayPage, RuntimeError> {
    let key = path.to_string_lossy().into_owned();
    if let Some(cached) = RENDERS.with(|c| c.borrow().get(&key).and_then(|m| m.get(&page).cloned())) {
        return Ok(cached);
    }
    let rendered = crate::pdf::render_pages_gray(path, &[page], ANALYSIS_WIDTH)
        .map_err(|e| err(format!("failed to render page {page}: {e:#}")))?;
    let (_, w, h, px) = rendered
        .into_iter()
        .next()
        .ok_or_else(|| err(format!("page {page} could not be rendered")))?;
    let value = (w, h, px);
    RENDERS.with(|c| {
        c.borrow_mut().entry(key).or_default().insert(page, value.clone());
    });
    Ok(value)
}

pub fn call(doc: &Rc<DocData>, name: &str, args: &[Value]) -> Result<Value, RuntimeError> {
    let images: Vec<&Rc<ImageData>> = doc.pages.iter().flat_map(|p| p.images.iter()).collect();
    match name {
        "detect_images" => Ok(Value::Bool(!images.is_empty())),
        "count_images" => Ok(Value::Int(images.len() as i64)),
        "get_image_resolution" => {
            let img = image_arg(&images, args, name)?;
            Ok(Value::Float(img.dpi_x.min(img.dpi_y)))
        }
        "get_image_size" => {
            let img = image_arg(&images, args, name)?;
            Ok(Value::List(Rc::new(vec![Value::Int(img.width), Value::Int(img.height)])))
        }
        "detect_image_color_space" => {
            // Without an argument: the list of distinct color spaces present.
            // With argument n: the color space of the nth image.
            if args.is_empty() {
                let mut spaces: Vec<String> = Vec::new();
                for img in &images {
                    if !spaces.contains(&img.color_space) {
                        spaces.push(img.color_space.clone());
                    }
                }
                Ok(Value::List(Rc::new(spaces.into_iter().map(Value::Str).collect())))
            } else {
                Ok(Value::Str(image_arg(&images, args, name)?.color_space.clone()))
            }
        }
        "detect_low_resolution" => {
            // true = there is an image below the minimum DPI (default 300).
            let min_dpi = match args.first() {
                Some(Value::Int(n)) => *n as f64,
                Some(Value::Float(n)) => *n,
                None => 300.0,
                Some(other) => {
                    return Err(err(format!(
                        "visual::detect_low_resolution expects a number (minimum DPI), got {}",
                        other.type_name()
                    )))
                }
            };
            Ok(Value::Bool(images.iter().any(|i| i.dpi_x.min(i.dpi_y) < min_dpi)))
        }
        // ---- visual comparison ----
        "calculate_perceptual_hash" => {
            // 64-bit pHash (8x8 DCT over a 32x32 render), in hexadecimal
            let page = page_arg(doc, args, name)?;
            let (w, h, px) = render_page(&doc.path, page)?;
            Ok(Value::Str(phash_hex(&px, w, h)))
        }
        "measure_ssim" => {
            // measure_ssim(pagina_a, "outro.pdf" [, pagina_b])
            let (a, b) = two_pages(doc, args, name)?;
            Ok(Value::Float(round3(ssim(&a, &b))))
        }
        "pixel_diff" => {
            // % of pixels differing beyond the threshold (default 10/255)
            let (a, b) = two_pages(doc, args, name)?;
            let tolerance = match args.get(3) {
                Some(Value::Int(n)) => *n as u8,
                Some(Value::Float(n)) => *n as u8,
                _ => 10,
            };
            Ok(Value::Float(round3(pixel_diff_ratio(&a, &b, tolerance) * 100.0)))
        }
        "compare_images" | "diff_pages" => {
            // Visual similarity 0–100 between this doc's page and another's
            let (a, b) = two_pages(doc, args, name)?;
            Ok(Value::Float(round3(ssim(&a, &b) * 100.0)))
        }
        "detect_image_replacement" => {
            // true = the pages' pHashes differ beyond the tolerated Hamming
            // distance (default 10 of 64 bits)
            let (a, b) = two_pages(doc, args, name)?;
            let max_dist = match args.get(3) {
                Some(Value::Int(n)) => *n as u32,
                _ => 10,
            };
            let ha = phash_bits(&a.2, a.0, a.1);
            let hb = phash_bits(&b.2, b.0, b.1);
            Ok(Value::Bool((ha ^ hb).count_ones() > max_dist))
        }
        // ---- quality analysis ----
        "detect_image_artifacts" => {
            // JPEG-style blocking: edge energy at multiples of 8 px above the
            // rest. > 1.6 indicates a visible artifact.
            let page = page_arg(doc, args, name)?;
            let (w, h, px) = render_page(&doc.path, page)?;
            Ok(Value::Bool(blockiness(&px, w, h) > 1.6))
        }
        "estimate_image_quality" => {
            // 0–100: penalizes blocking and lack of detail
            let page = page_arg(doc, args, name)?;
            let (w, h, px) = render_page(&doc.path, page)?;
            let block = blockiness(&px, w, h);
            let score = (100.0 - ((block - 1.0).max(0.0) * 60.0)).clamp(0.0, 100.0);
            Ok(Value::Float(round3(score)))
        }
        "detect_posterization" => {
            // Few distinct grey levels on a page with a gradient
            let page = page_arg(doc, args, name)?;
            let (_, _, px) = render_page(&doc.path, page)?;
            let mut seen = [false; 256];
            for &p in &px {
                seen[p as usize] = true;
            }
            let levels = seen.iter().filter(|&&s| s).count();
            let range = px.iter().max().copied().unwrap_or(0) as i32
                - px.iter().min().copied().unwrap_or(0) as i32;
            Ok(Value::Bool(range > 60 && levels < 24))
        }
        "detect_banding" => {
            // A gradient with jumps: on smooth lines, repeated abrupt variation
            let page = page_arg(doc, args, name)?;
            let (w, h, px) = render_page(&doc.path, page)?;
            Ok(Value::Bool(has_banding(&px, w, h)))
        }
        _ => Err(err(format!("unknown function: visual::{name}"))),
    }
}

// ---- visual analysis helpers ----

fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// A page of this document (default 1).
fn page_arg(doc: &Rc<DocData>, args: &[Value], name: &str) -> Result<i64, RuntimeError> {
    let n = match args.first() {
        Some(Value::Int(n)) => *n,
        Some(Value::Page(p)) => p.index + 1,
        None => 1,
        Some(other) => {
            return Err(err(format!("visual::{name} expects the page number, got {}", other.type_name())))
        }
    };
    if n < 1 || n as usize > doc.pages.len() {
        return Err(err(format!("page {n} does not exist (the PDF has {})", doc.pages.len())));
    }
    Ok(n)
}

/// `f(page_a, "other.pdf" [, page_b])` → renders aligned to the same size.
fn two_pages(
    doc: &Rc<DocData>,
    args: &[Value],
    name: &str,
) -> Result<(GrayPage, GrayPage), RuntimeError> {
    let page_a = page_arg(doc, args, name)?;
    let other = match args.get(1) {
        Some(Value::Str(p)) => std::path::PathBuf::from(p),
        _ => return Err(err(format!("visual::{name} expects the path of the other PDF as the 2nd argument"))),
    };
    let page_b = match args.get(2) {
        Some(Value::Int(n)) => *n,
        _ => page_a,
    };
    let a = render_page(&doc.path, page_a)?;
    let b = render_page(&other, page_b)?;
    Ok(resize_to_match(a, b))
}

/// Matches the dimensions by sampling (nearest neighbor) so they can be compared.
fn resize_to_match(a: GrayPage, b: GrayPage) -> (GrayPage, GrayPage) {
    if a.0 == b.0 && a.1 == b.1 {
        return (a, b);
    }
    let (w, h) = (a.0.min(b.0).max(1), a.1.min(b.1).max(1));
    (resize(&a, w, h), resize(&b, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Generates a smooth horizontal gradient.
    fn gradiente(w: u32, h: u32) -> GrayPage {
        let px = (0..h).flat_map(|_| (0..w).map(|x| (x * 255 / w.max(1)) as u8)).collect();
        (w, h, px)
    }

    /// A gradient quantized into few levels (posterized/banding).
    fn posterizado(w: u32, h: u32, niveis: u8) -> GrayPage {
        let step = 255 / niveis as u32;
        let px = (0..h)
            .flat_map(|_| (0..w).map(|x| (((x * 255 / w.max(1)) / step) * step) as u8))
            .collect();
        (w, h, px)
    }

    #[test]
    fn phash_same_and_different() {
        let a = gradiente(64, 64);
        let b = gradiente(64, 64);
        let ha = phash_bits(&a.2, a.0, a.1);
        assert_eq!(ha, phash_bits(&b.2, b.0, b.1), "mesma imagem = mesmo hash");
        // invertido: hash bem diferente
        let inv: Vec<u8> = a.2.iter().map(|p| 255 - p).collect();
        let hi = phash_bits(&inv, a.0, a.1);
        assert!((ha ^ hi).count_ones() > 10, "imagens opostas devem diferir");
        assert_eq!(phash_hex(&a.2, a.0, a.1).len(), 16);
    }

    #[test]
    fn ssim_and_pixel_diff() {
        let a = gradiente(64, 64);
        assert!((ssim(&a, &a) - 1.0).abs() < 1e-9, "idênticas = 1.0");
        assert_eq!(pixel_diff_ratio(&a, &a, 10), 0.0);

        let mut b = a.clone();
        for p in b.2.iter_mut().take(64 * 32) {
            *p = 0; // metade da imagem preta
        }
        assert!(ssim(&a, &b) < 0.9, "metade alterada deve baixar o SSIM");
        assert!(pixel_diff_ratio(&a, &b, 10) > 0.3);
    }

    #[test]
    fn resize_matches_dimensions() {
        let (a, b) = resize_to_match(gradiente(100, 50), gradiente(60, 30));
        assert_eq!((a.0, a.1), (60, 30));
        assert_eq!((b.0, b.1), (60, 30));
        assert_eq!(a.2.len(), 60 * 30);
    }

    #[test]
    fn posterization_and_banding() {
        let suave = gradiente(128, 128);
        let ruim = posterizado(128, 128, 8);
        let niveis = |g: &GrayPage| {
            let mut seen = [false; 256];
            g.2.iter().for_each(|&p| seen[p as usize] = true);
            seen.iter().filter(|&&s| s).count()
        };
        assert!(niveis(&suave) > 24 && niveis(&ruim) < 24);
        assert!(has_banding(&ruim.2, ruim.0, ruim.1), "degradê quantizado = banding");
        assert!(!has_banding(&suave.2, suave.0, suave.1), "degradê suave não é banding");
    }

    #[test]
    fn blockiness_detects_blocks() {
        // smooth image: no blocking
        let liso: GrayPage = (64, 64, vec![128; 64 * 64]);
        assert!((blockiness(&liso.2, 64, 64) - 1.0).abs() < 0.01);
        // steps every 8 px: high blocking
        let blocos: Vec<u8> =
            (0..64).flat_map(|_| (0..64).map(|x: u32| ((x / 8) * 32) as u8)).collect();
        assert!(blockiness(&blocos, 64, 64) > 1.6);
    }
}

fn resize(src: &GrayPage, w: u32, h: u32) -> GrayPage {
    let (sw, sh, px) = src;
    let mut out = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        let sy = (y as u64 * *sh as u64 / h as u64).min(*sh as u64 - 1) as u32;
        for x in 0..w {
            let sx = (x as u64 * *sw as u64 / w as u64).min(*sw as u64 - 1) as u32;
            out.push(px[(sy * sw + sx) as usize]);
        }
    }
    (w, h, out)
}

/// 64-bit pHash: 32x32 DCT-II, low-frequency 8x8 block vs the median.
fn phash_bits(px: &[u8], w: u32, h: u32) -> u64 {
    const N: usize = 32;
    let small = resize(&(w, h, px.to_vec()), N as u32, N as u32).2;
    // separable 2D DCT
    let cos_table: Vec<Vec<f64>> = (0..N)
        .map(|u| {
            (0..N)
                .map(|x| ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / (2.0 * N as f64)).cos())
                .collect()
        })
        .collect();
    let mut rows = vec![0.0f64; N * N];
    for y in 0..N {
        for u in 0..N {
            let mut sum = 0.0;
            for x in 0..N {
                sum += small[y * N + x] as f64 * cos_table[u][x];
            }
            rows[y * N + u] = sum;
        }
    }
    let mut dct = vec![0.0f64; N * N];
    for u in 0..N {
        for v in 0..N {
            let mut sum = 0.0;
            for y in 0..N {
                sum += rows[y * N + u] * cos_table[v][y];
            }
            dct[v * N + u] = sum;
        }
    }
    // low-frequency 8x8 block, without the DC term
    let mut block: Vec<f64> = Vec::with_capacity(64);
    for v in 0..8 {
        for u in 0..8 {
            block.push(dct[v * N + u]);
        }
    }
    let mut sorted: Vec<f64> = block[1..].to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = sorted[sorted.len() / 2];
    let mut bits = 0u64;
    for (i, value) in block.iter().enumerate() {
        if *value > median {
            bits |= 1 << i;
        }
    }
    bits
}

fn phash_hex(px: &[u8], w: u32, h: u32) -> String {
    format!("{:016x}", phash_bits(px, w, h))
}

/// Global SSIM using means, variances and covariance (0–1).
fn ssim(a: &GrayPage, b: &GrayPage) -> f64 {
    let (pa, pb) = (&a.2, &b.2);
    let n = pa.len().min(pb.len());
    if n == 0 {
        return 1.0;
    }
    let (mut sa, mut sb) = (0.0f64, 0.0f64);
    for i in 0..n {
        sa += pa[i] as f64;
        sb += pb[i] as f64;
    }
    let (ma, mb) = (sa / n as f64, sb / n as f64);
    let (mut va, mut vb, mut cov) = (0.0f64, 0.0f64, 0.0f64);
    for i in 0..n {
        let (da, db) = (pa[i] as f64 - ma, pb[i] as f64 - mb);
        va += da * da;
        vb += db * db;
        cov += da * db;
    }
    let (va, vb, cov) = (va / n as f64, vb / n as f64, cov / n as f64);
    let (c1, c2) = (6.5025, 58.5225); // (0.01*255)^2, (0.03*255)^2
    ((2.0 * ma * mb + c1) * (2.0 * cov + c2)) / ((ma * ma + mb * mb + c1) * (va + vb + c2))
}

fn pixel_diff_ratio(a: &GrayPage, b: &GrayPage, tolerance: u8) -> f64 {
    let (pa, pb) = (&a.2, &b.2);
    let n = pa.len().min(pb.len());
    if n == 0 {
        return 0.0;
    }
    let diff = (0..n).filter(|&i| pa[i].abs_diff(pb[i]) > tolerance).count();
    diff as f64 / n as f64
}

/// Ratio between edge energy at 8x8 block boundaries and everywhere else.
fn blockiness(px: &[u8], w: u32, h: u32) -> f64 {
    if w < 16 || h < 16 {
        return 1.0;
    }
    let (mut on_edge, mut on_edge_n, mut off_edge, mut off_edge_n) = (0.0f64, 0u64, 0.0f64, 0u64);
    for y in 0..h {
        for x in 1..w {
            let d = (px[(y * w + x) as usize] as f64 - px[(y * w + x - 1) as usize] as f64).abs();
            if x % 8 == 0 {
                on_edge += d;
                on_edge_n += 1;
            } else {
                off_edge += d;
                off_edge_n += 1;
            }
        }
    }
    if on_edge_n == 0 || off_edge_n == 0 {
        return 1.0;
    }
    let (a, b) = (on_edge / on_edge_n as f64, off_edge / off_edge_n as f64);
    if a < 0.01 {
        return 1.0; // imagem lisa: nada nos limites de bloco = sem blocagem
    }
    // Floor on the denominator: steps ONLY at block boundaries are the extreme
    // case of blocking, not its absence (with b == 0 the ratio is undefined).
    a / b.max(0.05)
}

/// Banding: a gradient that jumps instead of varying smoothly. Analyzes rows
/// and columns — the gradient's direction is not known in advance.
fn has_banding(px: &[u8], w: u32, h: u32) -> bool {
    if w < 32 || h < 32 {
        return false;
    }
    // Banding = a gradient progressing in one direction in wide steps.
    // Text also has jumps, but alternating in direction and without plateaus —
    // hence we require monotonicity and few steps relative to the length.
    let banding_in = |linha: &[u8]| {
        let deltas: Vec<(usize, i32)> = linha
            .windows(2)
            .enumerate()
            .map(|(i, p)| (i, p[1] as i32 - p[0] as i32))
            .filter(|(_, d)| *d != 0)
            .collect();
        if deltas.len() < 4 {
            return false;
        }
        // Wide steps: transitions are rare relative to the line's length
        if deltas.len() > linha.len() / 8 {
            return false; // too much variation = texture/text, not banding
        }
        let jumps: Vec<&(usize, i32)> = deltas.iter().filter(|(_, d)| d.abs() >= 4).collect();
        if jumps.len() < 3 {
            return false;
        }
        // Monotonic progression: at least 80% of the jumps in the same direction
        let positivos = jumps.iter().filter(|(_, d)| *d > 0).count();
        let dominante = positivos.max(jumps.len() - positivos);
        if dominante * 5 < jumps.len() * 4 {
            return false;
        }
        // Plateaus of similar size between the jumps (regular steps)
        let espacos: Vec<usize> = jumps.windows(2).map(|w| w[1].0 - w[0].0).collect();
        let media = espacos.iter().sum::<usize>() as f64 / espacos.len() as f64;
        media >= 4.0
    };
    let mut suspeitas = 0;
    // columns (vertical gradient)
    for x in (w / 4..w * 3 / 4).step_by((w / 16).max(1) as usize) {
        let col: Vec<u8> = (0..h).map(|y| px[(y * w + x) as usize]).collect();
        if banding_in(&col) {
            suspeitas += 1;
        }
    }
    // rows (horizontal gradient)
    for y in (h / 4..h * 3 / 4).step_by((h / 16).max(1) as usize) {
        let row = &px[(y * w) as usize..((y + 1) * w) as usize];
        if banding_in(row) {
            suspeitas += 1;
        }
    }
    suspeitas >= 3
}

fn err(message: String) -> RuntimeError {
    RuntimeError { message }
}

/// Optional argument: the image number (1-based). Without one, the first.
fn image_arg<'a>(
    images: &'a [&'a Rc<ImageData>],
    args: &[Value],
    name: &str,
) -> Result<&'a Rc<ImageData>, RuntimeError> {
    let n = match args.first() {
        Some(Value::Int(n)) => *n,
        None => 1,
        Some(other) => {
            return Err(err(format!("visual::{name} expects the image number, got {}", other.type_name())))
        }
    };
    if n < 1 || n as usize > images.len() {
        return Err(err(format!("image {n} does not exist (the PDF has {})", images.len())));
    }
    Ok(images[(n - 1) as usize])
}
