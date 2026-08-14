//! Pixel-level comparison of two rendered pages.
//!
//! `pdfl compare` answers "did the text or the structure change". This answers
//! a different question: "does it *look* the same", which is the one a print
//! shop asks about a corrected file. A logo nudged 2mm, a hairline that
//! vanished and a colour swapped for another of the same luminance all leave
//! the text identical.
//!
//! Everything here is a pure function over pixel buffers — no pdfium, no
//! files — so the algorithm is testable without rendering anything.

/// How the pixels are compared.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// Per-pixel colour distance, 0.0–1.0, above which two pixels differ.
    /// Lower is stricter.
    pub threshold: f64,
    /// Look for a global shift between the pages before comparing. A page
    /// nudged by one pixel is otherwise reported as different everywhere,
    /// which buries the change that matters.
    pub align: bool,
    /// How far the shift search may look, in pixels.
    pub max_offset: i32,
    /// Box-blur radius applied before comparing. Anti-aliasing puts different
    /// grey values on the same glyph edge in two renders; blurring makes those
    /// agree without hiding a real change.
    pub blur: u32,
    /// Rough diff percentage above which alignment is attempted at all. Below
    /// it the pages already line up and the search would be wasted work.
    pub align_above: f64,
}

impl Default for Options {
    fn default() -> Self {
        // The threshold and the alignment trigger come from pdfjob, where they
        // were settled against real print files.
        Options { threshold: 0.05, align: true, max_offset: 32, blur: 0, align_above: 3.0 }
    }
}

/// What one pixel turned out to be. The three cases are what makes the overlay
/// readable at a glance: ink that left, ink that arrived, and ink that only
/// changed colour.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Change {
    Same,
    /// Darker in A: content that was there and is not any more.
    Removed,
    /// Darker in B: content that was not there before.
    Added,
    /// Same weight, different colour.
    Recoloured,
}

impl Change {
    /// The overlay colour. Red and green are the diff convention; blue for a
    /// recolour, which is neither a loss nor an addition.
    fn rgba(self) -> [u8; 4] {
        match self {
            Change::Same => [0, 0, 0, 0],
            Change::Removed => [220, 38, 38, 255],
            Change::Added => [22, 163, 74, 255],
            Change::Recoloured => [37, 99, 235, 255],
        }
    }
}

/// A page rendered to RGBA.
pub struct Page {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

impl Page {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Self {
        Page { width, height, rgba }
    }

    fn at(&self, x: i32, y: i32) -> [u8; 4] {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            // Outside the page is white, not transparent: a page that is
            // simply larger should read as "this area was added", not as a
            // hole in the comparison.
            return [255, 255, 255, 255];
        }
        let i = ((y as u32 * self.width + x as u32) * 4) as usize;
        [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2], self.rgba[i + 3]]
    }
}

/// The outcome for one page pair.
pub struct PageDiff {
    pub page: i64,
    /// Percentage of compared pixels that differ, 0–100.
    pub diff_percent: f64,
    /// Bounding boxes of the changed areas, in pixels of the rendered page.
    pub regions: Vec<Region>,
    /// The shift that had to be applied to line the pages up, if any.
    pub shift: (i32, i32),
    pub width: u32,
    pub height: u32,
    /// RGBA overlay: transparent where the pages agree, coloured where they do
    /// not. Made to be drawn *over* the page, which is what lets the viewer
    /// show the change in place instead of side by side.
    pub overlay: Vec<u8>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Compares two rendered pages.
pub fn compare(a: &Page, b: &Page, opts: Options, page: i64) -> PageDiff {
    let (a, b) = (maybe_blur(a, opts.blur), maybe_blur(b, opts.blur));

    let shift = if opts.align { find_shift(&a, &b, opts) } else { (0, 0) };
    let (dx, dy) = shift;

    let (w, h) = (a.width.max(b.width), a.height.max(b.height));
    let mut overlay = vec![0u8; (w as usize) * (h as usize) * 4];
    let mut mask = vec![false; (w as usize) * (h as usize)];
    let (mut compared, mut differing) = (0u64, 0u64);

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let pa = a.at(x, y);
            let pb = b.at(x - dx, y - dy);
            let change = classify(pa, pb, opts.threshold);
            compared += 1;
            if change != Change::Same {
                differing += 1;
                let i = ((y as u32 * w + x as u32) * 4) as usize;
                overlay[i..i + 4].copy_from_slice(&change.rgba());
                mask[(y as u32 * w + x as u32) as usize] = true;
            }
        }
    }

    let diff_percent =
        if compared == 0 { 0.0 } else { differing as f64 / compared as f64 * 100.0 };

    PageDiff {
        page,
        diff_percent,
        regions: find_regions(&mask, w, h),
        shift,
        width: w,
        height: h,
        overlay,
    }
}

/// Which of the three kinds of change, if any, two pixels represent.
fn classify(a: [u8; 4], b: [u8; 4], threshold: f64) -> Change {
    if colour_distance(a, b) <= threshold {
        return Change::Same;
    }
    let (la, lb) = (luminance(a), luminance(b));
    // A tolerance on the luminance comparison, otherwise a change that is
    // mostly a recolour gets called added or removed on rounding noise.
    const SAME_WEIGHT: f64 = 8.0;
    if (la - lb).abs() <= SAME_WEIGHT {
        Change::Recoloured
    } else if la < lb {
        Change::Removed
    } else {
        Change::Added
    }
}

fn luminance(p: [u8; 4]) -> f64 {
    // Alpha matters: an unpainted pixel is white on a page, and treating it as
    // black would make every margin look like removed content.
    let a = p[3] as f64 / 255.0;
    let lum = 0.299 * p[0] as f64 + 0.587 * p[1] as f64 + 0.114 * p[2] as f64;
    lum * a + 255.0 * (1.0 - a)
}

/// Normalised 0.0–1.0 distance between two colours, alpha included.
fn colour_distance(a: [u8; 4], b: [u8; 4]) -> f64 {
    // Composited over white first, so "transparent" and "white" compare equal:
    // one renderer's blank margin is the other's white rectangle.
    let over_white = |p: [u8; 4]| {
        let al = p[3] as f64 / 255.0;
        [
            p[0] as f64 * al + 255.0 * (1.0 - al),
            p[1] as f64 * al + 255.0 * (1.0 - al),
            p[2] as f64 * al + 255.0 * (1.0 - al),
        ]
    };
    let (ca, cb) = (over_white(a), over_white(b));
    let (dr, dg, db) = (ca[0] - cb[0], ca[1] - cb[1], ca[2] - cb[2]);
    // 441.67 = the longest possible distance in RGB space, so the result is 0–1.
    (dr * dr + dg * dg + db * db).sqrt() / 441.67
}

fn maybe_blur(page: &Page, radius: u32) -> Page {
    if radius == 0 {
        return Page { width: page.width, height: page.height, rgba: page.rgba.clone() };
    }
    box_blur(page, radius)
}

/// Separable box blur. Two one-dimensional passes instead of one square
/// window: the same result for a fraction of the work.
fn box_blur(page: &Page, radius: u32) -> Page {
    let (w, h) = (page.width, page.height);
    let r = radius as i32;
    let window = (2 * r + 1) as u32;
    let mut horizontal = vec![0u8; page.rgba.len()];

    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut sum = [0u32; 4];
            for k in -r..=r {
                let px = (x + k).clamp(0, w as i32 - 1);
                let p = page.at(px, y);
                for c in 0..4 {
                    sum[c] += p[c] as u32;
                }
            }
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            for c in 0..4 {
                horizontal[i + c] = (sum[c] / window) as u8;
            }
        }
    }

    let mid = Page { width: w, height: h, rgba: horizontal };
    let mut out = vec![0u8; page.rgba.len()];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            let mut sum = [0u32; 4];
            for k in -r..=r {
                let py = (y + k).clamp(0, h as i32 - 1);
                let p = mid.at(x, py);
                for c in 0..4 {
                    sum[c] += p[c] as u32;
                }
            }
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            for c in 0..4 {
                out[i + c] = (sum[c] / window) as u8;
            }
        }
    }
    Page { width: w, height: h, rgba: out }
}

/// Finds the global shift between two pages, coarse pass then fine.
///
/// Searching every offset at full resolution would cost more than the
/// comparison itself, so this looks on a 1/8-scale greyscale copy first and
/// only refines the winner.
fn find_shift(a: &Page, b: &Page, opts: Options) -> (i32, i32) {
    const COARSE: u32 = 8;
    const FINE: u32 = 2;

    let (ga, gb) = (grey(a, COARSE), grey(b, COARSE));
    if rough_diff(&ga, &gb, opts.threshold) < opts.align_above {
        return (0, 0);
    }

    let coarse_range = (opts.max_offset / COARSE as i32).max(2);
    let (cx, cy) = best_shift(&ga, &gb, 0, 0, coarse_range);

    let (fa, fb) = (grey(a, FINE), grey(b, FINE));
    let centre = (cx * COARSE as i32 / FINE as i32, cy * COARSE as i32 / FINE as i32);
    let (fx, fy) = best_shift(&fa, &fb, centre.0, centre.1, (COARSE / FINE) as i32);

    // Negated: the search asks "where in B is A", the comparison asks the
    // reverse.
    (-fx * FINE as i32, -fy * FINE as i32)
}

struct Grey {
    width: u32,
    height: u32,
    px: Vec<u8>,
}

impl Grey {
    fn at(&self, x: i32, y: i32) -> Option<u8> {
        if x < 0 || y < 0 || x >= self.width as i32 || y >= self.height as i32 {
            return None;
        }
        Some(self.px[(y as u32 * self.width + x as u32) as usize])
    }
}

/// Downsamples to greyscale by averaging `scale`×`scale` boxes.
fn grey(page: &Page, scale: u32) -> Grey {
    let w = page.width.div_ceil(scale);
    let h = page.height.div_ceil(scale);
    let mut px = vec![0u8; (w * h) as usize];
    for gy in 0..h {
        for gx in 0..w {
            let (mut sum, mut n) = (0u32, 0u32);
            for sy in 0..scale {
                for sx in 0..scale {
                    let (x, y) = ((gx * scale + sx) as i32, (gy * scale + sy) as i32);
                    if x >= page.width as i32 || y >= page.height as i32 {
                        continue;
                    }
                    sum += luminance(page.at(x, y)) as u32;
                    n += 1;
                }
            }
            if let Some(mean) = sum.checked_div(n) {
                px[(gy * w + gx) as usize] = mean as u8;
            }
        }
    }
    Grey { width: w, height: h, px }
}

/// Diff percentage at shift zero, on the downsampled copies.
fn rough_diff(a: &Grey, b: &Grey, threshold: f64) -> f64 {
    let (mut total, mut diff) = (0u64, 0u64);
    for y in 0..a.height as i32 {
        for x in 0..a.width as i32 {
            let (Some(va), Some(vb)) = (a.at(x, y), b.at(x, y)) else { continue };
            if (va as f64 - vb as f64).abs() / 255.0 > threshold {
                diff += 1;
            }
            total += 1;
        }
    }
    if total == 0 {
        0.0
    } else {
        diff as f64 / total as f64 * 100.0
    }
}

/// The offset around `centre` that makes the two images agree best.
fn best_shift(a: &Grey, b: &Grey, centre_x: i32, centre_y: i32, range: i32) -> (i32, i32) {
    let mut best = (f64::MAX, centre_x, centre_y);
    for dy in (centre_y - range)..=(centre_y + range) {
        for dx in (centre_x - range)..=(centre_x + range) {
            let score = mean_abs_diff(a, b, dx, dy);
            if score < best.0 {
                best = (score, dx, dy);
            }
        }
    }
    (best.1, best.2)
}

fn mean_abs_diff(a: &Grey, b: &Grey, dx: i32, dy: i32) -> f64 {
    let (mut sum, mut n) = (0u64, 0u64);
    for y in 0..a.height as i32 {
        for x in 0..a.width as i32 {
            let (Some(va), Some(vb)) = (a.at(x, y), b.at(x + dx, y + dy)) else { continue };
            sum += va.abs_diff(vb) as u64;
            n += 1;
        }
    }
    if n == 0 {
        f64::MAX
    } else {
        sum as f64 / n as f64
    }
}

/// Groups changed pixels into boxes on a 32-pixel grid.
///
/// Per-pixel coordinates would be useless to a person and enormous in a
/// report; a grid of blocks says "look here" at the resolution someone can
/// actually act on.
fn find_regions(mask: &[bool], w: u32, h: u32) -> Vec<Region> {
    const BLOCK: u32 = 32;
    let mut regions = Vec::new();
    let mut by = 0;
    while by < h {
        let mut bx = 0;
        while bx < w {
            let (end_x, end_y) = ((bx + BLOCK).min(w), (by + BLOCK).min(h));
            let mut touched = false;
            'block: for y in by..end_y {
                for x in bx..end_x {
                    if mask[(y * w + x) as usize] {
                        touched = true;
                        break 'block;
                    }
                }
            }
            if touched {
                regions.push(Region { x: bx, y: by, width: end_x - bx, height: end_y - by });
            }
            bx += BLOCK;
        }
        by += BLOCK;
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A solid page of one colour.
    fn solid(w: u32, h: u32, colour: [u8; 4]) -> Page {
        Page::new(w, h, colour.iter().copied().cycle().take((w * h * 4) as usize).collect())
    }

    fn with_rect(w: u32, h: u32, rect: (u32, u32, u32, u32), ink: [u8; 4]) -> Page {
        let mut page = solid(w, h, [255, 255, 255, 255]);
        let (rx, ry, rw, rh) = rect;
        for y in ry..(ry + rh).min(h) {
            for x in rx..(rx + rw).min(w) {
                let i = ((y * w + x) * 4) as usize;
                page.rgba[i..i + 4].copy_from_slice(&ink);
            }
        }
        page
    }

    #[test]
    fn identical_pages_have_no_difference() {
        let a = with_rect(64, 64, (10, 10, 20, 20), [0, 0, 0, 255]);
        let b = with_rect(64, 64, (10, 10, 20, 20), [0, 0, 0, 255]);
        let d = compare(&a, &b, Options::default(), 1);
        assert_eq!(d.diff_percent, 0.0);
        assert!(d.regions.is_empty());
        assert!(d.overlay.iter().all(|&b| b == 0), "the overlay must be fully transparent");
    }

    /// The three kinds of change are what make the overlay readable, so each
    /// one has to come out as its own colour.
    #[test]
    fn ink_that_vanished_is_red_and_ink_that_arrived_is_green() {
        let white = solid(32, 32, [255, 255, 255, 255]);
        let black = solid(32, 32, [0, 0, 0, 255]);

        let removed = compare(&black, &white, Options { align: false, ..Default::default() }, 1);
        assert_eq!(&removed.overlay[0..4], &Change::Removed.rgba());

        let added = compare(&white, &black, Options { align: false, ..Default::default() }, 1);
        assert_eq!(&added.overlay[0..4], &Change::Added.rgba());
    }

    #[test]
    fn a_colour_swap_at_the_same_weight_is_neither_added_nor_removed() {
        // Red and green of the same weight: 0.299×200 = 59.8 and
        // 0.587×102 = 59.9, plainly different to the eye and identical to a
        // luminance test.
        let a = solid(16, 16, [200, 0, 0, 255]);
        let b = solid(16, 16, [0, 102, 0, 255]);
        let la = luminance([200, 0, 0, 255]);
        let lb = luminance([0, 102, 0, 255]);
        assert!((la - lb).abs() <= 8.0, "test premise: {la} vs {lb}");

        let d = compare(&a, &b, Options { align: false, ..Default::default() }, 1);
        assert_eq!(&d.overlay[0..4], &Change::Recoloured.rgba());
    }

    /// A blank margin and a painted-white margin are the same page to a
    /// person, so they must not be reported as a difference.
    #[test]
    fn transparent_and_white_compare_equal() {
        let transparent = solid(16, 16, [0, 0, 0, 0]);
        let white = solid(16, 16, [255, 255, 255, 255]);
        let d = compare(&transparent, &white, Options { align: false, ..Default::default() }, 1);
        assert_eq!(d.diff_percent, 0.0);
    }

    #[test]
    fn the_changed_area_is_reported_as_a_region() {
        let a = solid(128, 128, [255, 255, 255, 255]);
        let b = with_rect(128, 128, (40, 40, 16, 16), [0, 0, 0, 255]);
        let d = compare(&a, &b, Options { align: false, ..Default::default() }, 1);

        assert!(d.diff_percent > 0.0);
        assert!(!d.regions.is_empty());
        // Every reported block has to actually contain the change.
        for r in &d.regions {
            assert!(r.x < 64 && r.y < 64, "unexpected region {r:?}");
        }
    }

    /// The whole point of aligning: a page nudged by a pixel is the same page,
    /// and without this it reports as different along every edge.
    #[test]
    fn a_shifted_page_is_recognised_as_the_same_page() {
        let a = with_rect(128, 128, (30, 30, 40, 40), [0, 0, 0, 255]);
        let b = with_rect(128, 128, (34, 36, 40, 40), [0, 0, 0, 255]);

        let unaligned = compare(&a, &b, Options { align: false, ..Default::default() }, 1);
        let aligned = compare(&a, &b, Options::default(), 1);

        assert!(
            aligned.diff_percent < unaligned.diff_percent,
            "aligning made it worse: {} vs {}",
            aligned.diff_percent,
            unaligned.diff_percent
        );
        assert_ne!(aligned.shift, (0, 0), "the shift should have been found");
    }

    /// Pages of different sizes still compare: the missing area reads as a
    /// change rather than crashing or being silently skipped.
    #[test]
    fn pages_of_different_sizes_compare_over_the_union() {
        let small = solid(32, 32, [255, 255, 255, 255]);
        let large = solid(64, 64, [255, 255, 255, 255]);
        let d = compare(&small, &large, Options { align: false, ..Default::default() }, 1);
        assert_eq!((d.width, d.height), (64, 64));
        // Both are white, and outside-the-page reads as white, so no change.
        assert_eq!(d.diff_percent, 0.0);
    }

    #[test]
    fn blur_absorbs_a_single_stray_pixel() {
        let a = solid(64, 64, [255, 255, 255, 255]);
        let mut b = solid(64, 64, [255, 255, 255, 255]);
        let i = ((32 * 64 + 32) * 4) as usize;
        b.rgba[i..i + 4].copy_from_slice(&[0, 0, 0, 255]);

        let sharp = compare(&a, &b, Options { align: false, blur: 0, ..Default::default() }, 1);
        let blurred = compare(&a, &b, Options { align: false, blur: 2, ..Default::default() }, 1);
        assert!(sharp.diff_percent > 0.0);
        assert!(
            blurred.diff_percent < sharp.diff_percent,
            "blur should have softened it: {} vs {}",
            blurred.diff_percent,
            sharp.diff_percent
        );
    }
}
