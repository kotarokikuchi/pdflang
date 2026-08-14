//! Writes the folder `pdfl pixelcompare --viewer` produces: the rendered
//! pages, the difference overlays, and a small application to look at them.
//!
//! A percentage tells someone *that* two files differ. Deciding whether the
//! difference matters means seeing it, and seeing it means putting one page on
//! top of the other — which a static report cannot do. Hence a viewer: wipe
//! between the two, flip between them, or fade the overlay in.
//!
//! The page is one HTML file with its CSS and JavaScript inline, next to PNGs.
//! No bundler, no CDN, no server: opening `index.html` from the filesystem has
//! to work, because the folder gets zipped and mailed to whoever has to
//! approve the file.

use crate::pixelcmp::PageDiff;
use anyhow::{Context, Result};
use std::path::Path;

/// One page's three images, already encoded as PNG.
pub struct PageAssets {
    pub page: i64,
    pub before: Vec<u8>,
    pub after: Vec<u8>,
    pub overlay: Vec<u8>,
}

/// Encodes an RGBA buffer as a PNG.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>> {
    use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
    let mut out = Vec::new();
    PngEncoder::new(&mut out)
        .write_image(rgba, width, height, ExtendedColorType::Rgba8)
        .context("could not encode the page as PNG")?;
    Ok(out)
}

/// Writes the viewer folder: one PNG trio per page plus `index.html`.
pub fn write(
    dir: &Path,
    before_name: &str,
    after_name: &str,
    diffs: &[PageDiff],
    assets: &[PageAssets],
) -> Result<()> {
    std::fs::create_dir_all(dir)
        .with_context(|| format!("could not create {}", dir.display()))?;

    for a in assets {
        for (suffix, bytes) in
            [("before", &a.before), ("after", &a.after), ("diff", &a.overlay)]
        {
            let name = format!("page-{:03}-{suffix}.png", a.page);
            std::fs::write(dir.join(&name), bytes)
                .with_context(|| format!("could not write {name}"))?;
        }
    }

    let html = index_html(before_name, after_name, diffs);
    std::fs::write(dir.join("index.html"), html)
        .with_context(|| format!("could not write {}", dir.join("index.html").display()))?;
    Ok(())
}

/// The page data the script needs, as JSON.
fn pages_json(diffs: &[PageDiff]) -> String {
    let entries: Vec<String> = diffs
        .iter()
        .map(|d| {
            let regions: Vec<String> = d
                .regions
                .iter()
                .map(|r| format!("{{\"x\":{},\"y\":{},\"w\":{},\"h\":{}}}", r.x, r.y, r.width, r.height))
                .collect();
            format!(
                "{{\"page\":{},\"diff\":{:.4},\"w\":{},\"h\":{},\"shift\":[{},{}],\"regions\":[{}]}}",
                d.page,
                d.diff_percent,
                d.width,
                d.height,
                d.shift.0,
                d.shift.1,
                regions.join(",")
            )
        })
        .collect();
    format!("[{}]", entries.join(","))
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// A string as a JavaScript literal, safe to sit inside a `<script>` block.
///
/// JSON quoting alone is not enough. The HTML parser ends the script at the
/// first `</script` whatever the JavaScript grammar thinks, and `<script` and
/// `<!--` inside the block push it into its escaped states — so a file named
/// `</script><img onerror=...>.pdf` would break out. Escaping every `<` as
/// `<` leaves the parser nothing to match and the string value unchanged.
fn js_string(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".into()).replace('<', "\\u003C")
}

fn index_html(before_name: &str, after_name: &str, diffs: &[PageDiff]) -> String {
    let changed = diffs.iter().filter(|d| d.diff_percent > 0.0).count();
    let worst = diffs.iter().map(|d| d.diff_percent).fold(0.0f64, f64::max);
    format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Pixel comparison — {before} vs {after}</title>
<style>
  :root {{
    --bg: #f6f7f9; --panel: #fff; --ink: #1a1d21; --muted: #667085;
    --line: #e4e7ec; --accent: #2563eb;
    --removed: #dc2626; --added: #16a34a; --recoloured: #2563eb;
  }}
  @media (prefers-color-scheme: dark) {{
    :root {{
      --bg: #14171a; --panel: #1c2024; --ink: #e8eaed; --muted: #98a2b3;
      --line: #2c3238;
    }}
  }}
  * {{ box-sizing: border-box; }}
  body {{
    margin: 0; background: var(--bg); color: var(--ink);
    font: 14px/1.5 system-ui, -apple-system, Segoe UI, sans-serif;
  }}
  header {{
    padding: 14px 20px; border-bottom: 1px solid var(--line);
    background: var(--panel); position: sticky; top: 0; z-index: 5;
  }}
  h1 {{ font-size: 15px; margin: 0 0 4px; font-weight: 600; }}
  .files {{ color: var(--muted); font-size: 13px; }}
  .files b {{ color: var(--ink); font-weight: 600; }}
  .bar {{
    display: flex; gap: 18px; align-items: center; flex-wrap: wrap;
    padding: 10px 20px; border-bottom: 1px solid var(--line); background: var(--panel);
    position: sticky; top: 58px; z-index: 4;
  }}
  .group {{ display: flex; gap: 6px; align-items: center; }}
  .group > label {{ color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: .04em; }}
  button {{
    font: inherit; padding: 5px 11px; border-radius: 6px; cursor: pointer;
    border: 1px solid var(--line); background: transparent; color: var(--ink);
  }}
  button[aria-pressed="true"] {{ background: var(--accent); border-color: var(--accent); color: #fff; }}
  select, input[type=range] {{ font: inherit; accent-color: var(--accent); }}
  select {{ padding: 4px 8px; border-radius: 6px; border: 1px solid var(--line); background: var(--panel); color: var(--ink); }}
  main {{ padding: 20px; display: flex; justify-content: center; }}
  .stage {{
    position: relative; background: var(--panel); border: 1px solid var(--line);
    border-radius: 8px; overflow: hidden; box-shadow: 0 1px 3px rgba(0,0,0,.06);
    /* Chequerboard, so a transparent page is visibly transparent. */
    background-image:
      linear-gradient(45deg, rgba(128,128,128,.09) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(128,128,128,.09) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, rgba(128,128,128,.09) 75%),
      linear-gradient(-45deg, transparent 75%, rgba(128,128,128,.09) 75%);
    background-size: 16px 16px;
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  }}
  /* An <img> is draggable by default: without this the browser starts its own
     ghost-image drag on the first move and the wipe freezes where it began.
     Events belong to the stage, never to the pictures on it. */
  .stage img {{
    display: block; max-width: 100%; height: auto;
    pointer-events: none; user-select: none; -webkit-user-drag: none;
  }}
  /* touch-action: the browser would otherwise claim a horizontal drag as a
     scroll gesture and cancel the pointer halfway. */
  .stage {{ touch-action: none; user-select: none; }}
  .layer {{ position: absolute; inset: 0; }}
  .layer img {{ width: 100%; height: 100%; }}
  /* The wipe: the top layer is clipped to the left of the handle. */
  #after-layer {{ clip-path: inset(0 0 0 var(--split)); }}
  #overlay-layer {{ pointer-events: none; opacity: 0; transition: opacity .12s; }}
  #handle {{
    position: absolute; top: 0; bottom: 0; width: 2px; background: var(--accent);
    cursor: ew-resize; z-index: 3;
  }}
  #handle::after {{
    content: ""; position: absolute; top: 50%; left: 50%;
    width: 26px; height: 26px; margin: -13px 0 0 -13px; border-radius: 50%;
    background: var(--accent); box-shadow: 0 1px 4px rgba(0,0,0,.35);
  }}
  .pages {{ display: flex; gap: 6px; flex-wrap: wrap; padding: 0 20px 20px; justify-content: center; }}
  .chip {{
    padding: 4px 10px; border-radius: 999px; border: 1px solid var(--line);
    background: var(--panel); cursor: pointer; font-size: 13px;
  }}
  .chip[aria-current="true"] {{ border-color: var(--accent); color: var(--accent); font-weight: 600; }}
  .chip.identical {{ color: var(--muted); }}
  .chip .pct {{ font-variant-numeric: tabular-nums; opacity: .75; margin-left: 5px; font-size: 12px; }}
  footer {{ padding: 0 20px 28px; color: var(--muted); font-size: 12.5px; text-align: center; }}
  .key {{ display: inline-flex; gap: 14px; flex-wrap: wrap; justify-content: center; margin-top: 6px; }}
  .key span {{ display: inline-flex; gap: 5px; align-items: center; }}
  .dot {{ width: 10px; height: 10px; border-radius: 2px; display: inline-block; }}
  kbd {{
    font: 11px/1 ui-monospace, monospace; padding: 2px 5px; border: 1px solid var(--line);
    border-bottom-width: 2px; border-radius: 4px; background: var(--panel);
  }}
</style>
</head>
<body>
<header>
  <h1>Pixel comparison</h1>
  <div class="files"><b>{before}</b> → <b>{after}</b> · {changed} of {total} page(s) differ · worst {worst:.2}%</div>
</header>

<div class="bar">
  <div class="group">
    <label>Mode</label>
    <button id="m-wipe" aria-pressed="true">Wipe</button>
    <button id="m-flip" aria-pressed="false">Flip</button>
    <button id="m-fade" aria-pressed="false">Fade</button>
  </div>
  <div class="group" id="fade-group" hidden>
    <label for="fade">Blend</label>
    <input id="fade" type="range" min="0" max="100" value="50">
  </div>
  <div class="group">
    <label for="ov">Differences</label>
    <input id="ov" type="range" min="0" max="100" value="100" title="Overlay opacity">
  </div>
  <div class="group">
    <label for="zoom">Zoom</label>
    <select id="zoom">
      <option value="fit" selected>Fit</option>
      <option value="1">100%</option>
      <option value="1.5">150%</option>
      <option value="2">200%</option>
    </select>
  </div>
  <div class="group" id="showing"></div>
</div>

<main>
  <div class="stage" id="stage">
    <img id="base" alt="">
    <div class="layer" id="after-layer"><img id="after" alt=""></div>
    <div class="layer" id="overlay-layer"><img id="overlay" alt=""></div>
    <div id="handle"></div>
  </div>
</main>

<div class="pages" id="pages"></div>

<footer>
  <div>
    <kbd>←</kbd> <kbd>→</kbd> page · <kbd>space</kbd> flip · <kbd>d</kbd> differences
  </div>
  <div class="key">
    <span><i class="dot" style="background:var(--removed)"></i> gone from the new file</span>
    <span><i class="dot" style="background:var(--added)"></i> new in it</span>
    <span><i class="dot" style="background:var(--recoloured)"></i> same weight, other colour</span>
  </div>
</footer>

<script>
const PAGES = {pages_json};
const BEFORE = {before_js};
const AFTER = {after_js};

const el = id => document.getElementById(id);
const stage = el("stage"), handle = el("handle");
let index = 0, mode = "wipe", split = 50, flipShowsAfter = false;

function pad(n) {{ return String(n).padStart(3, "0"); }}

function render() {{
  const p = PAGES[index];
  if (!p) return;
  el("base").src = `page-${{pad(p.page)}}-before.png`;
  el("after").src = `page-${{pad(p.page)}}-after.png`;
  el("overlay").src = `page-${{pad(p.page)}}-diff.png`;
  el("base").alt = `${{BEFORE}}, page ${{p.page}}`;
  el("after").alt = `${{AFTER}}, page ${{p.page}}`;
  el("overlay").alt = `differences on page ${{p.page}}`;
  stage.style.aspectRatio = `${{p.w}} / ${{p.h}}`;
  applyZoom();
  applyMode();
  el("showing").textContent =
    p.diff === 0 ? "identical" : `${{p.diff.toFixed(2)}}% of pixels differ`;
  for (const chip of document.querySelectorAll(".chip")) {{
    chip.setAttribute("aria-current", String(Number(chip.dataset.i) === index));
  }}
}}

function applyMode() {{
  const wipe = mode === "wipe";
  handle.hidden = !wipe;
  el("fade-group").hidden = mode !== "fade";
  const layer = el("after-layer");
  if (wipe) {{
    layer.style.clipPath = `inset(0 0 0 ${{split}}%)`;
    layer.style.opacity = 1;
    handle.style.left = `calc(${{split}}% - 1px)`;
  }} else if (mode === "flip") {{
    layer.style.clipPath = "none";
    layer.style.opacity = flipShowsAfter ? 1 : 0;
  }} else {{
    layer.style.clipPath = "none";
    layer.style.opacity = el("fade").value / 100;
  }}
  for (const b of ["wipe", "flip", "fade"]) {{
    el("m-" + b).setAttribute("aria-pressed", String(mode === b));
  }}
}}

function applyZoom() {{
  const z = el("zoom").value;
  const p = PAGES[index];
  stage.style.width = z === "fit" ? "min(100%, 900px)" : `${{p.w * Number(z)}}px`;
}}

// Dragging anywhere on the stage moves the wipe: grabbing a 2px handle with a
// mouse is a chore, and the whole point is a quick back-and-forth.
function dragTo(clientX) {{
  const r = stage.getBoundingClientRect();
  split = Math.min(100, Math.max(0, ((clientX - r.left) / r.width) * 100));
  applyMode();
}}
let dragging = false;
stage.addEventListener("pointerdown", e => {{
  if (mode !== "wipe") return;
  dragging = true;
  stage.setPointerCapture(e.pointerId);
  dragTo(e.clientX);
}});
stage.addEventListener("pointermove", e => {{ if (dragging) dragTo(e.clientX); }});
for (const end of ["pointerup", "pointercancel"]) {{
  // pointercancel as well as pointerup: a gesture the browser takes over
  // never sends an up, and the wipe would stay stuck to the pointer.
  stage.addEventListener(end, e => {{
    dragging = false;
    if (stage.hasPointerCapture(e.pointerId)) stage.releasePointerCapture(e.pointerId);
  }});
}}

// In flip mode, holding the pointer down shows the other file — the fastest
// way to spot what moved.
stage.addEventListener("pointerdown", () => {{
  if (mode === "flip") {{ flipShowsAfter = true; applyMode(); }}
}});
stage.addEventListener("pointerup", () => {{
  if (mode === "flip") {{ flipShowsAfter = false; applyMode(); }}
}});

for (const b of ["wipe", "flip", "fade"]) {{
  el("m-" + b).addEventListener("click", () => {{ mode = b; applyMode(); }});
}}
el("fade").addEventListener("input", applyMode);
el("ov").addEventListener("input", () => {{
  el("overlay-layer").style.opacity = el("ov").value / 100;
}});
el("zoom").addEventListener("change", applyZoom);

document.addEventListener("keydown", e => {{
  if (e.key === "ArrowRight" && index < PAGES.length - 1) {{ index++; render(); }}
  else if (e.key === "ArrowLeft" && index > 0) {{ index--; render(); }}
  else if (e.key === " ") {{
    e.preventDefault();
    if (mode !== "flip") {{ mode = "flip"; }}
    flipShowsAfter = !flipShowsAfter;
    applyMode();
  }} else if (e.key.toLowerCase() === "d") {{
    const o = el("ov");
    o.value = o.value > 0 ? 0 : 100;
    o.dispatchEvent(new Event("input"));
  }}
}});

const list = el("pages");
PAGES.forEach((p, i) => {{
  const b = document.createElement("button");
  b.className = "chip" + (p.diff === 0 ? " identical" : "");
  b.dataset.i = i;
  b.innerHTML = `Page ${{p.page}}<span class="pct">${{p.diff === 0 ? "—" : p.diff.toFixed(2) + "%"}}</span>`;
  b.addEventListener("click", () => {{ index = i; render(); }});
  list.appendChild(b);
}});

el("overlay-layer").style.opacity = 1;
render();
</script>
</body>
</html>
"##,
        before = escape(before_name),
        after = escape(after_name),
        before_js = js_string(before_name),
        after_js = js_string(after_name),
        changed = changed,
        total = diffs.len(),
        worst = worst,
        pages_json = pages_json(diffs),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pixelcmp::Region;

    fn diff(page: i64, pct: f64) -> PageDiff {
        PageDiff {
            page,
            diff_percent: pct,
            regions: vec![Region { x: 0, y: 32, width: 32, height: 32 }],
            shift: (1, -2),
            width: 200,
            height: 300,
            overlay: Vec::new(),
        }
    }

    #[test]
    fn the_page_data_reaches_the_script_as_json() {
        let json = pages_json(&[diff(1, 12.5)]);
        let parsed: serde_json::Value = serde_json::from_str(&json).expect("valid JSON");
        assert_eq!(parsed[0]["page"], 1);
        assert_eq!(parsed[0]["diff"], 12.5);
        assert_eq!(parsed[0]["shift"][0], 1);
        assert_eq!(parsed[0]["regions"][0]["y"], 32);
    }

    /// A file name is not trusted markup: it comes from the command line and
    /// lands both in the HTML and inside a `<script>` block.
    #[test]
    fn a_hostile_file_name_cannot_break_out_of_the_page() {
        let hostile = r#"a"><script>alert(1)</script>.pdf"#;
        let html = index_html(hostile, "b.pdf", &[diff(1, 1.0)]);

        // The HTML side: no live tag survives.
        assert!(!html.contains("<script>alert(1)"), "the injected tag survived");
        assert!(html.contains("&lt;script&gt;alert(1)"), "expected it escaped instead");

        // The JavaScript side: the only `</script` in the file is the real one
        // closing the block. JSON quoting alone would have left the one from
        // the file name intact, ending the script early.
        assert_eq!(html.matches("</script").count(), 1, "a second </script would end the block");
    }

    #[test]
    fn js_string_keeps_the_value_while_defusing_the_tag() {
        let quoted = js_string("</script><!--x");
        assert!(!quoted.contains('<'), "{quoted}");
        // Still the same string once parsed: `<` is just `<`.
        let back: String = serde_json::from_str(&quoted).unwrap();
        assert_eq!(back, "</script><!--x");
    }

    #[test]
    fn the_folder_gets_one_html_and_three_pngs_per_page() {
        let dir = std::env::temp_dir().join(format!("pdfl-viewer-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let assets = vec![PageAssets {
            page: 1,
            before: b"png-a".to_vec(),
            after: b"png-b".to_vec(),
            overlay: b"png-d".to_vec(),
        }];
        write(&dir, "a.pdf", "b.pdf", &[diff(1, 1.0)], &assets).unwrap();

        assert!(dir.join("index.html").is_file());
        assert!(dir.join("page-001-before.png").is_file());
        assert!(dir.join("page-001-after.png").is_file());
        assert!(dir.join("page-001-diff.png").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_png_round_trips_through_the_encoder() {
        let rgba = vec![255u8, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
        let png = encode_png(2, 2, &rgba).unwrap();
        assert_eq!(&png[1..4], b"PNG");
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (2, 2));
        assert_eq!(decoded.as_raw(), &rgba);
    }
}
