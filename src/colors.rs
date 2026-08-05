//! Análise de separações de cor a partir do content stream (lopdf).
//! É o que permite TAC exato, detecção de spot colors, preto rico e
//! overprint — coisas que a estimativa por render RGB não alcança.

use lopdf::{Document, Object};
use std::collections::{HashMap, HashSet};

/// Cor lida do content stream, já em separações.
#[derive(Debug, Clone, PartialEq)]
pub enum Ink {
    /// Ciano, magenta, amarelo, preto — cada um 0.0–1.0.
    Cmyk(f64, f64, f64, f64),
    Gray(f64),
    Rgb(f64, f64, f64),
    /// Spot/Separation: nome da tinta e percentual aplicado.
    Spot(String, f64),
}

impl Ink {
    /// Cobertura total de tinta (%) da cor.
    pub fn tac(&self) -> f64 {
        match self {
            Ink::Cmyk(c, m, y, k) => (c + m + y + k) * 100.0,
            Ink::Gray(g) => (1.0 - g) * 100.0,
            // RGB é convertido na RIP; a conversão com GCR máximo é o
            // limite inferior honesto para essa cor.
            Ink::Rgb(r, g, b) => {
                let k = 1.0 - r.max(*g).max(*b);
                if k >= 1.0 {
                    100.0
                } else {
                    let c = (1.0 - r - k) / (1.0 - k);
                    let m = (1.0 - g - k) / (1.0 - k);
                    let y = (1.0 - b - k) / (1.0 - k);
                    (c + m + y + k) * 100.0
                }
            }
            Ink::Spot(_, pct) => pct * 100.0,
        }
    }

    /// Cor neutra escura composta por várias tintas (preto rico).
    pub fn is_rich_black(&self) -> bool {
        match self {
            Ink::Cmyk(c, m, y, k) => *k >= 0.6 && (c + m + y) >= 0.2,
            _ => false,
        }
    }

    /// Lab aproximado para comparação Delta-E.
    fn to_lab(&self) -> (f64, f64, f64) {
        let (r, g, b) = match self {
            Ink::Rgb(r, g, b) => (*r, *g, *b),
            Ink::Gray(v) => (*v, *v, *v),
            Ink::Cmyk(c, m, y, k) => {
                ((1.0 - c) * (1.0 - k), (1.0 - m) * (1.0 - k), (1.0 - y) * (1.0 - k))
            }
            Ink::Spot(_, pct) => (1.0 - pct, 1.0 - pct, 1.0 - pct),
        };
        rgb_to_lab(r, g, b)
    }
}

/// sRGB (0–1) para CIE Lab (D65).
fn rgb_to_lab(r: f64, g: f64, b: f64) -> (f64, f64, f64) {
    let lin = |c: f64| {
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let (r, g, b) = (lin(r), lin(g), lin(b));
    let x = (r * 0.4124 + g * 0.3576 + b * 0.1805) / 0.95047;
    let y = r * 0.2126 + g * 0.7152 + b * 0.0722;
    let z = (r * 0.0193 + g * 0.1192 + b * 0.9505) / 1.08883;
    let f = |t: f64| {
        if t > 0.008856 {
            t.cbrt()
        } else {
            7.787 * t + 16.0 / 116.0
        }
    };
    let (fx, fy, fz) = (f(x), f(y), f(z));
    (116.0 * fy - 16.0, 500.0 * (fx - fy), 200.0 * (fy - fz))
}

/// Delta-E CIE76 entre duas cores.
pub fn delta_e(a: &Ink, b: &Ink) -> f64 {
    let (l1, a1, b1) = a.to_lab();
    let (l2, a2, b2) = b.to_lab();
    ((l1 - l2).powi(2) + (a1 - a2).powi(2) + (b1 - b2).powi(2)).sqrt()
}

/// Resultado da análise de separações do documento.
#[derive(Debug, Default)]
pub struct ColorInfo {
    /// TAC máximo real por página (%), a partir das cores declaradas.
    pub tac_by_page: Vec<f64>,
    /// Nomes de tintas spot encontradas (Separation/DeviceN).
    pub spot_names: Vec<String>,
    /// Todas as cores distintas usadas.
    pub inks: Vec<Ink>,
    pub has_rich_black: bool,
    /// Overprint ligado em algum estado gráfico.
    pub overprint_on: bool,
    /// Menor largura de traço declarada (pontos), ignorando 0 (hairline puro).
    pub min_stroke: Option<f64>,
    /// Traço de largura 0 (hairline "mais fino possível" do PostScript).
    pub has_zero_width_stroke: bool,
    pub output_intent: Option<String>,
    /// Rendering intents declarados (operador ri e /RI no ExtGState).
    pub rendering_intents: Vec<String>,
    /// Fontes: nome -> (embutida, tem subset, tipo)
    pub fonts: HashMap<String, FontDetail>,
    /// Tamanhos de fonte usados (pontos).
    pub font_sizes: Vec<f64>,
}

#[derive(Debug, Clone)]
pub struct FontDetail {
    pub embedded: bool,
    pub subset: bool,
    pub font_type: String,
    /// Diferença entre glifos referenciados e disponíveis (Widths ausentes).
    pub missing_widths: bool,
}

pub fn analyze(path: &std::path::Path) -> Result<ColorInfo, String> {
    let pdf = Document::load(path).map_err(|e| e.to_string())?;
    let mut info = ColorInfo::default();
    let mut spots: HashSet<String> = HashSet::new();
    let mut seen_inks: Vec<Ink> = Vec::new();

    // Output intent do catálogo
    if let Ok(catalog) = pdf.catalog() {
        if let Ok(intents) = catalog.get(b"OutputIntents") {
            let resolved = resolve(&pdf, intents.clone());
            if let Object::Array(items) = resolved {
                if let Some(first) = items.first() {
                    if let Object::Dictionary(d) = resolve(&pdf, first.clone()) {
                        info.output_intent = d
                            .get(b"OutputConditionIdentifier")
                            .ok()
                            .and_then(|v| v.as_str().ok())
                            .map(|s| String::from_utf8_lossy(s).into_owned());
                    }
                }
            }
        }
    }

    for (_, page_id) in pdf.get_pages() {
        let content_bytes = pdf.get_page_content(page_id);
        let Ok(content) = lopdf::content::Content::decode(&content_bytes) else {
            info.tac_by_page.push(0.0);
            continue;
        };

        // Recursos da página: espaços de cor nomeados e estados gráficos
        let resources = pdf
            .get_page_resources(page_id)
            .ok()
            .and_then(|(dict, _)| dict.cloned())
            .unwrap_or_default();
        let colorspaces = dict_of(&pdf, &resources, b"ColorSpace");
        let extgstates = dict_of(&pdf, &resources, b"ExtGState");
        register_fonts(&pdf, &resources, &mut info);

        // Nome do espaço atual (para interpretar scn) — fill e stroke
        let mut cs_fill = String::new();
        let mut cs_stroke = String::new();
        let mut page_tac = 0.0f64;

        for op in content.operations.iter() {
            let nums: Vec<f64> = op.operands.iter().filter_map(as_num).collect();
            let mut record = |ink: Ink, info: &mut ColorInfo, seen: &mut Vec<Ink>| {
                let tac = ink.tac();
                if tac > page_tac {
                    page_tac = tac;
                }
                if ink.is_rich_black() {
                    info.has_rich_black = true;
                }
                if !seen.contains(&ink) {
                    seen.push(ink);
                }
            };
            match op.operator.as_str() {
                "k" | "K" if nums.len() == 4 => {
                    record(Ink::Cmyk(nums[0], nums[1], nums[2], nums[3]), &mut info, &mut seen_inks)
                }
                "g" | "G" if nums.len() == 1 => record(Ink::Gray(nums[0]), &mut info, &mut seen_inks),
                "rg" | "RG" if nums.len() == 3 => {
                    record(Ink::Rgb(nums[0], nums[1], nums[2]), &mut info, &mut seen_inks)
                }
                "cs" | "CS" => {
                    let name = op
                        .operands
                        .first()
                        .and_then(|o| o.as_name().ok())
                        .map(|n| String::from_utf8_lossy(n).into_owned())
                        .unwrap_or_default();
                    // registra spot declarada no espaço de cor
                    if let Some(spot) = spot_name(&pdf, &colorspaces, &name) {
                        spots.insert(spot);
                    }
                    if op.operator == "cs" {
                        cs_fill = name;
                    } else {
                        cs_stroke = name;
                    }
                }
                "scn" | "SCN" | "sc" | "SC" => {
                    let space = if op.operator.starts_with('s') { &cs_fill } else { &cs_stroke };
                    if let Some(spot) = spot_name(&pdf, &colorspaces, space) {
                        let pct = nums.first().copied().unwrap_or(1.0);
                        spots.insert(spot.clone());
                        record(Ink::Spot(spot, pct), &mut info, &mut seen_inks);
                    } else {
                        match nums.len() {
                            4 => record(
                                Ink::Cmyk(nums[0], nums[1], nums[2], nums[3]),
                                &mut info,
                                &mut seen_inks,
                            ),
                            3 => record(
                                Ink::Rgb(nums[0], nums[1], nums[2]),
                                &mut info,
                                &mut seen_inks,
                            ),
                            1 => record(Ink::Gray(nums[0]), &mut info, &mut seen_inks),
                            _ => {}
                        }
                    }
                }
                "w" => {
                    if let Some(width) = nums.first() {
                        if *width == 0.0 {
                            info.has_zero_width_stroke = true;
                        } else if info.min_stroke.is_none_or(|m| *width < m) {
                            info.min_stroke = Some(*width);
                        }
                    }
                }
                "ri" => {
                    if let Some(name) = op.operands.first().and_then(|o| o.as_name().ok()) {
                        push_unique(&mut info.rendering_intents, String::from_utf8_lossy(name).into_owned());
                    }
                }
                "Tf" => {
                    if let Some(size) = nums.first() {
                        if *size > 0.0 && !info.font_sizes.contains(size) {
                            info.font_sizes.push(*size);
                        }
                    }
                }
                "gs" => {
                    // estado gráfico nomeado: overprint e rendering intent
                    if let Some(name) = op.operands.first().and_then(|o| o.as_name().ok()) {
                        if let Some(gs) = extgstates.as_ref().and_then(|d| d.get(name).ok()) {
                            if let Object::Dictionary(d) = resolve(&pdf, gs.clone()) {
                                for key in [b"OP".as_slice(), b"op".as_slice()] {
                                    if matches!(d.get(key), Ok(Object::Boolean(true))) {
                                        info.overprint_on = true;
                                    }
                                }
                                if let Ok(ri) = d.get(b"RI").and_then(|v| v.as_name()) {
                                    push_unique(
                                        &mut info.rendering_intents,
                                        String::from_utf8_lossy(ri).into_owned(),
                                    );
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
        info.tac_by_page.push(page_tac);
    }

    // "All" e "None" são separações reservadas do PDF (registro e nada),
    // não tintas spot de verdade — reportá-las seria alarme falso.
    info.spot_names = spots.into_iter().filter(|s| s != "All" && s != "None").collect();
    info.spot_names.sort();
    info.inks = seen_inks;
    Ok(info)
}

fn push_unique(v: &mut Vec<String>, s: String) {
    if !v.contains(&s) {
        v.push(s);
    }
}

fn as_num(o: &Object) -> Option<f64> {
    match o {
        Object::Integer(n) => Some(*n as f64),
        Object::Real(n) => Some(*n as f64),
        _ => None,
    }
}

fn resolve(pdf: &Document, obj: Object) -> Object {
    match obj {
        Object::Reference(id) => pdf.get_object(id).cloned().unwrap_or(Object::Null),
        other => other,
    }
}

fn dict_of(pdf: &Document, resources: &lopdf::Dictionary, key: &[u8]) -> Option<lopdf::Dictionary> {
    resources.get(key).ok().map(|v| resolve(pdf, v.clone())).and_then(|o| match o {
        Object::Dictionary(d) => Some(d),
        _ => None,
    })
}

/// Se o espaço de cor nomeado for Separation/DeviceN, devolve o nome da tinta.
fn spot_name(pdf: &Document, colorspaces: &Option<lopdf::Dictionary>, name: &str) -> Option<String> {
    let cs = colorspaces.as_ref()?.get(name.as_bytes()).ok()?;
    let Object::Array(items) = resolve(pdf, cs.clone()) else { return None };
    let family = items.first()?.as_name().ok()?;
    match family {
        b"Separation" => {
            let ink = items.get(1)?.as_name().ok()?;
            Some(String::from_utf8_lossy(ink).into_owned())
        }
        b"DeviceN" => {
            let Object::Array(names) = resolve(pdf, items.get(1)?.clone()) else { return None };
            let joined: Vec<String> = names
                .iter()
                .filter_map(|n| n.as_name().ok())
                .map(|n| String::from_utf8_lossy(n).into_owned())
                .collect();
            Some(joined.join("+"))
        }
        _ => None,
    }
}

/// Coleta detalhes das fontes dos recursos da página.
fn register_fonts(pdf: &Document, resources: &lopdf::Dictionary, info: &mut ColorInfo) {
    let Some(fonts) = dict_of(pdf, resources, b"Font") else { return };
    for (_, font_ref) in fonts.iter() {
        let Object::Dictionary(font) = resolve(pdf, font_ref.clone()) else { continue };
        let base = font
            .get(b"BaseFont")
            .ok()
            .and_then(|v| v.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_default();
        if base.is_empty() {
            continue;
        }
        let font_type = font
            .get(b"Subtype")
            .ok()
            .and_then(|v| v.as_name().ok())
            .map(|n| String::from_utf8_lossy(n).into_owned())
            .unwrap_or_default();
        // Subset: prefixo de 6 letras maiúsculas + "+" (ex: ABCDEF+Arial)
        let subset = base.len() > 7
            && base.as_bytes()[6] == b'+'
            && base[..6].chars().all(|c| c.is_ascii_uppercase());

        // Embutida: FontFile/FontFile2/FontFile3 no descritor (direto ou
        // no descendente, para fontes CID)
        let mut descriptors = Vec::new();
        if let Ok(fd) = font.get(b"FontDescriptor") {
            descriptors.push(resolve(pdf, fd.clone()));
        }
        if let Ok(Object::Array(descendants)) = font.get(b"DescendantFonts").map(|v| resolve(pdf, v.clone())) {
            for d in descendants {
                if let Object::Dictionary(dd) = resolve(pdf, d) {
                    if let Ok(fd) = dd.get(b"FontDescriptor") {
                        descriptors.push(resolve(pdf, fd.clone()));
                    }
                }
            }
        }
        let embedded = descriptors.iter().any(|d| match d {
            Object::Dictionary(d) => {
                d.get(b"FontFile").is_ok() || d.get(b"FontFile2").is_ok() || d.get(b"FontFile3").is_ok()
            }
            _ => false,
        });
        let missing_widths = font.get(b"Widths").is_err() && font.get(b"DescendantFonts").is_err();

        info.fonts.insert(
            base.clone(),
            FontDetail { embedded, subset, font_type, missing_widths },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tac_por_tipo_de_cor() {
        assert_eq!(Ink::Cmyk(1.0, 1.0, 0.0, 0.5).tac(), 250.0);
        assert_eq!(Ink::Gray(0.0).tac(), 100.0); // preto puro em cinza
        assert_eq!(Ink::Spot("PANTONE 485".into(), 1.0).tac(), 100.0);
        assert!((Ink::Rgb(0.0, 0.0, 0.0).tac() - 100.0).abs() < 0.01);
    }

    #[test]
    fn preto_rico() {
        assert!(Ink::Cmyk(0.6, 0.4, 0.4, 1.0).is_rich_black());
        assert!(!Ink::Cmyk(0.0, 0.0, 0.0, 1.0).is_rich_black()); // preto chapado
        assert!(!Ink::Cmyk(0.1, 0.1, 0.1, 0.2).is_rich_black()); // claro demais
    }

    #[test]
    fn delta_e_entre_cores() {
        let preto = Ink::Cmyk(0.0, 0.0, 0.0, 1.0);
        assert!(delta_e(&preto, &preto) < 0.01);
        // preto vs branco: diferença máxima
        assert!(delta_e(&preto, &Ink::Cmyk(0.0, 0.0, 0.0, 0.0)) > 90.0);
        // dois cinzas próximos: diferença pequena
        assert!(delta_e(&Ink::Gray(0.5), &Ink::Gray(0.52)) < 3.0);
    }

    #[test]
    fn analise_de_fixture_com_cmyk() {
        // fixture prepress.pdf: "1 1 0 0.5 k" = TAC 250%, traço 0.1
        let info = analyze(std::path::Path::new("tests/fixtures/prepress.pdf")).unwrap();
        assert_eq!(info.tac_by_page.len(), 1);
        assert!((info.tac_by_page[0] - 250.0).abs() < 0.01, "TAC exato: {:?}", info.tac_by_page);
        assert!(info.min_stroke.is_some_and(|w| (w - 0.1).abs() < 0.001), "{:?}", info.min_stroke);
        assert!(!info.has_rich_black);
        assert!(info.spot_names.is_empty());
        assert!(info.fonts.contains_key("Helvetica"));
        assert!(info.font_sizes.contains(&12.0));
    }
}
