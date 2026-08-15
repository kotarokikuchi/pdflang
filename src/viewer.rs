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
/// `on_page` is called once per page written: three PNGs a page at 300 dpi is
/// gigabytes for a long document, slow enough to be worth showing.
pub fn write(
    dir: &Path,
    before_name: &str,
    after_name: &str,
    diffs: &[PageDiff],
    assets: &[PageAssets],
    on_page: &mut dyn FnMut(),
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
        on_page();
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
  html, body {{ height: 100%; }}
  /* The three panes are sized against the window, so the page itself never
     scrolls: what is on screen is the whole of it. */
  body {{
    margin: 0; background: var(--bg); color: var(--ink); overflow: hidden;
    font: 13px/1.45 system-ui, -apple-system, Segoe UI, sans-serif;
    display: flex; flex-direction: column;
  }}

  /* One bar, one line where the window allows it: every pixel it takes is a
     pixel the pages do not get. */
  .bar {{
    flex: none; display: flex; align-items: center; gap: 14px; flex-wrap: wrap;
    padding: 6px 12px; background: var(--panel); border-bottom: 1px solid var(--line);
  }}
  .files {{ font-size: 12.5px; color: var(--muted); white-space: nowrap; }}
  .files b {{ color: var(--ink); font-weight: 600; }}
  .sep {{ width: 1px; align-self: stretch; background: var(--line); }}
  .group {{ display: flex; gap: 5px; align-items: center; }}
  .group > label {{
    color: var(--muted); font-size: 11px; text-transform: uppercase; letter-spacing: .04em;
  }}
  button {{
    font: inherit; padding: 3px 9px; border-radius: 6px; cursor: pointer;
    border: 1px solid var(--line); background: transparent; color: var(--ink);
  }}
  button[aria-pressed="true"] {{ background: var(--accent); border-color: var(--accent); color: #fff; }}
  button[disabled] {{ opacity: .45; cursor: not-allowed; }}
  #showing, #zoomlevel {{ font-size: 12.5px; color: var(--muted); white-space: nowrap; font-variant-numeric: tabular-nums; }}
  .key {{ display: flex; gap: 10px; font-size: 11.5px; color: var(--muted); white-space: nowrap; }}
  .key span {{ display: inline-flex; gap: 4px; align-items: center; }}
  .dot {{ width: 9px; height: 9px; border-radius: 2px; display: inline-block; }}

  /* One scrolling row, never a block that grows down the window. */
  .strip {{
    flex: none; display: flex; gap: 5px; padding: 5px 12px; overflow-x: auto;
    background: var(--panel); border-bottom: 1px solid var(--line);
  }}
  .chip {{
    flex: none; padding: 2px 9px; border-radius: 999px; border: 1px solid var(--line);
    background: transparent; cursor: pointer; font-size: 12px; white-space: nowrap;
  }}
  .chip[aria-current="true"] {{ border-color: var(--accent); color: var(--accent); font-weight: 600; }}
  .chip.identical {{ color: var(--muted); }}
  .chip[hidden] {{ display: none; }}
  .chip .pct {{ font-variant-numeric: tabular-nums; opacity: .75; margin-left: 5px; font-size: 11px; }}

  /* min-height: 0 on both, or a grid child refuses to shrink below its content
     and the panes push the window into scrolling. */
  main {{
    flex: 1 1 auto; min-height: 0; display: grid; gap: 8px; padding: 8px;
    grid-template-columns: repeat(3, minmax(0, 1fr));
  }}
  .pane {{ display: flex; flex-direction: column; min-height: 0; min-width: 0; }}
  .pane h2 {{
    flex: none; margin: 0 0 5px; font-size: 11px; font-weight: 600; color: var(--muted);
    text-transform: uppercase; letter-spacing: .04em;
    display: flex; gap: 6px; align-items: baseline;
  }}
  .pane h2 em {{
    font-style: normal; font-weight: 400; text-transform: none; letter-spacing: 0;
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
  }}
  /* container-type: size makes cqw/cqh below resolve against this box.
     overflow: hidden is what keeps a zoomed page inside its own pane. */
  .fit {{
    flex: 1 1 auto; min-height: 0; display: flex; container-type: size;
    overflow: hidden;
  }}
  /* Fitting a page into a box in both directions, without distorting it.
     `width: 100%` with `max-height` does not do this: when the height clamps,
     the width stays and the page is stretched — a wide, short window turned
     A4 into a letterbox. Taking the smaller of "as wide as the box" and "as
     wide as this box's height allows" keeps the page's own proportions, which
     is also what makes a wipe at 50% land at 50% of the page rather than of
     some empty area beside it. The plain `width: 100%` first is the fallback
     for a browser without container query units. */
  .stage {{
    margin: auto; position: relative; aspect-ratio: var(--ar, 1);
    width: 100%;
    width: min(100cqw, calc(100cqh * var(--arn, 1)));
    max-width: 100%; max-height: 100%;
    background: var(--panel); border: 1px solid var(--line); border-radius: 6px;
    overflow: hidden;
    /* Chequerboard, so a transparent page is visibly transparent. */
    background-image:
      linear-gradient(45deg, rgba(128,128,128,.09) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(128,128,128,.09) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, rgba(128,128,128,.09) 75%),
      linear-gradient(-45deg, transparent 75%, rgba(128,128,128,.09) 75%);
    background-size: 16px 16px;
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
  }}
  /* object-fit: when the two files disagree about a page's size — a page
     turned landscape, a different paper — the box is the union of both, and
     without this each image is stretched to fill it. Contained, each keeps its
     own proportions inside the shared box, which is also what keeps the two
     halves of the wipe measuring the same thing. */
  /* Zoom is paint, not layout: the box keeps the size the fit gave it and is
     drawn larger, so nothing about the fitting maths has to know about it. */
  .stage {{ transform: scale(var(--z, 1)); transform-origin: var(--ox, 50%) var(--oy, 50%); }}
  .stage img {{
    position: absolute; inset: 0; width: 100%; height: 100%; display: block;
    object-fit: contain;
    /* An <img> is draggable by default: without this the browser starts its own
       ghost-image drag on the first move and the wipe freezes where it began. */
    pointer-events: none; user-select: none; -webkit-user-drag: none;
  }}
  .layer {{ position: absolute; inset: 0; }}
  /* touch-action: the browser would otherwise claim a horizontal drag as a
     scroll gesture and cancel the pointer halfway. */
  .stage {{ touch-action: none; user-select: none; cursor: ew-resize; }}
  /* Both cuts at once: the new file shows to the right of the upright bar and
     below the flat one, so where they cross is the corner of the reveal. */
  #wipe-after {{ clip-path: inset(var(--splity, 0%) 0 0 var(--splitx, 50%)); }}
  /* The same pair of bars in all three panes, in the same place. In the
     difference pane they cut between the two files; in the other two they are
     rulers on the same column and the same line of the page, so what the wipe
     is cutting can be found in either original without measuring by eye. */
  /* Divided by the zoom, so a bar stays a hairline and the dot stays a dot
     however far in the page is scaled — the scale is on their parent. */
  .handle, .hbar {{ position: absolute; background: var(--accent); z-index: 3; }}
  .handle {{ top: 0; bottom: 0; width: calc(2px / var(--z, 1)); }}
  .hbar {{ left: 0; right: 0; height: calc(2px / var(--z, 1)); }}
  /* The dot rides the upright bar at the height of the pointer, which puts it
     exactly where the two bars cross. */
  .handle::after {{
    content: ""; position: absolute; top: var(--doty, 50%); left: 50%;
    width: calc(22px / var(--z, 1)); height: calc(22px / var(--z, 1));
    margin: calc(-11px / var(--z, 1)) 0 0 calc(-11px / var(--z, 1));
    border-radius: 50%;
    background: var(--accent); box-shadow: 0 1px 4px rgba(0,0,0,.35);
    /* Translucent like the bars in the other two panes: the dot sits on the
       part of the page being looked at, and an opaque disc hides exactly what
       it is pointing at. */
    opacity: .75;
  }}
  /* Only the pane that actually cuts carries the grab dot at full size; the
     other two show thinner lines, so which pane does what stays obvious. */
  .stage:not(#wipe) .handle {{ width: calc(1px / var(--z, 1)); opacity: .75; }}
  .stage:not(#wipe) .hbar {{ height: calc(1px / var(--z, 1)); opacity: .75; }}
  .stage:not(#wipe) .handle::after {{
    width: calc(12px / var(--z, 1)); height: calc(12px / var(--z, 1));
    margin: calc(-6px / var(--z, 1)) 0 0 calc(-6px / var(--z, 1));
  }}
  #empty {{ flex: none; padding: 10px 12px; text-align: center; color: var(--muted); }}

  /* Too narrow for three across: stack them and let the window scroll, rather
     than shrinking each page to a stamp. */
  @media (max-width: 760px) {{
    body {{ overflow: auto; }}
    main {{ grid-template-columns: 1fr; }}
    .fit {{ min-height: 60vh; }}
  }}
</style>
</head>
<body>

<div class="bar">
  <div class="files"><b>{before}</b> → <b>{after}</b></div>
  <div class="sep"></div>
  <div class="files">{changed} of {total} page(s) differ · worst {worst:.2}%</div>
  <div class="sep"></div>
  <div class="group">
    <label>Pages</label>
    <button id="f-changed" aria-pressed="true">Changed only</button>
    <button id="f-all" aria-pressed="false">All</button>
  </div>
  <div class="group">
    <button id="prev" title="Previous page (left arrow)">←</button>
    <button id="next" title="Next page (right arrow)">→</button>
    <span id="showing"></span>
  </div>
  <div class="group">
    <button id="reset" title="Back to the whole page, bars where they started">Reset view</button>
    <span id="zoomlevel"></span>
  </div>
  <div class="sep"></div>
  <div class="key">
    <span><i class="dot" style="background:var(--removed)"></i> gone</span>
    <span><i class="dot" style="background:var(--added)"></i> new</span>
    <span><i class="dot" style="background:var(--recoloured)"></i> recoloured</span>
  </div>
</div>

<div class="strip" id="pages"></div>
<div id="empty" hidden>Every page compared is identical.</div>

<main>
  <section class="pane">
    <h2>Original <em>{before}</em></h2>
    <div class="fit"><div class="stage" id="s-before"><img id="p-before" alt=""><div class="handle"></div><div class="hbar"></div></div></div>
  </section>
  <section class="pane">
    <h2>New <em>{after}</em></h2>
    <div class="fit"><div class="stage" id="s-after"><img id="p-after" alt=""><div class="handle"></div><div class="hbar"></div></div></div>
  </section>
  <section class="pane">
    <h2>Difference <em>drag to wipe</em></h2>
    <div class="fit">
      <div class="stage" id="wipe">
        <img id="wipe-before" alt="">
        <div class="layer" id="wipe-after"><img id="wipe-after-img" alt=""></div>
        <img id="wipe-diff" alt="">
        <div class="handle"></div>
        <div class="hbar"></div>
      </div>
    </div>
  </section>
</main>

<script>
const PAGES = {pages_json};
const BEFORE = {before_js};
const AFTER = {after_js};

const el = id => document.getElementById(id);
// Opens on the pages that differ: on a long document those are the reason
// anyone opened this at all. Falls back to every page when none differ.
let filter = PAGES.some(p => p.diff > 0) ? "changed" : "all";
let index = 0, splitX = 50, splitY = 0, zoom = 1, ox = 50, oy = 50;

function pad(n) {{ return String(n).padStart(3, "0"); }}

/// The indices the filter admits.
function shown() {{
  return PAGES.map((p, i) => i).filter(i => filter === "all" || PAGES[i].diff > 0);
}}

function applyFilter() {{
  const visible = shown();
  for (const chip of document.querySelectorAll(".chip")) {{
    chip.hidden = !visible.includes(Number(chip.dataset.i));
  }}
  el("f-all").setAttribute("aria-pressed", String(filter === "all"));
  el("f-changed").setAttribute("aria-pressed", String(filter === "changed"));
  // Filtering to nothing would leave a blank strip and no way back, so say so
  // and keep the page on screen.
  el("empty").hidden = visible.length > 0;
  // The page being looked at may be one the filter just hid: move to the
  // nearest one it admits rather than showing a page no chip points at.
  if (visible.length > 0 && !visible.includes(index)) {{
    index = visible.reduce((best, i) =>
      Math.abs(i - index) < Math.abs(best - index) ? i : best, visible[0]);
  }}
  render();
}}

function render() {{
  const p = PAGES[index];
  if (!p) return;
  const before = `page-${{pad(p.page)}}-before.png`;
  const after = `page-${{pad(p.page)}}-after.png`;
  const diff = `page-${{pad(p.page)}}-diff.png`;
  el("p-before").src = before;
  el("p-after").src = after;
  el("wipe-before").src = before;
  el("wipe-after-img").src = after;
  el("wipe-diff").src = diff;
  el("p-before").alt = `${{BEFORE}}, page ${{p.page}}`;
  el("p-after").alt = `${{AFTER}}, page ${{p.page}}`;
  el("wipe-diff").alt = `differences on page ${{p.page}}`;
  for (const id of STAGES) {{
    el(id).style.setProperty("--ar", `${{p.w}} / ${{p.h}}`);
    el(id).style.setProperty("--arn", String(p.w / p.h));
  }}
  applyWipe();

  const visible = shown();
  const at = visible.indexOf(index);
  el("prev").disabled = at <= 0;
  el("next").disabled = at < 0 || at >= visible.length - 1;
  el("showing").textContent =
    `page ${{p.page}} · ` + (p.diff === 0 ? "identical" : `${{p.diff.toFixed(2)}}% of pixels differ`);
  for (const chip of document.querySelectorAll(".chip")) {{
    chip.setAttribute("aria-current", String(Number(chip.dataset.i) === index));
  }}
}}

const STAGES = ["s-before", "s-after", "wipe"];

/// One position, three panes. Both splits are percentages of the page rather
/// than of any pane, and every pane holds the page at the same proportions, so
/// one pair of numbers lands on the same spot of the document in all three.
/// The zoom, and the point it grows around. Applied to all three panes from one
/// pair of numbers, so they stay on the same part of the page.
const MAX_ZOOM = 8;
function applyView() {{
  for (const id of STAGES) {{
    const st = el(id).style;
    st.setProperty("--z", String(zoom));
    st.setProperty("--ox", `${{ox}}%`);
    st.setProperty("--oy", `${{oy}}%`);
  }}
  el("zoomlevel").textContent = zoom > 1.001 ? `${{Math.round(zoom * 100)}}%` : "";
  el("reset").disabled = zoom <= 1.001 && splitX === 50 && splitY === 0;
}}

function applyWipe() {{
  const cut = el("wipe-after").style;
  cut.setProperty("--splitx", `${{splitX}}%`);
  cut.setProperty("--splity", `${{splitY}}%`);
  for (const bar of document.querySelectorAll(".handle")) {{
    bar.style.left = `calc(${{splitX}}% - 1px)`;
    // The dot sits where the pointer is holding the bar, which is also where
    // the two cuts meet: the corner of what is being revealed.
    bar.style.setProperty("--doty", `${{splitY}}%`);
  }}
  for (const bar of document.querySelectorAll(".hbar")) {{
    bar.style.top = `calc(${{splitY}}% - 1px)`;
  }}
  applyView();
}}

// The upright bar is dragged; the flat one follows the pointer wherever it is,
// pressed or not. Dragging anywhere on any pane moves all three: grabbing a 2px
// bar with a mouse is a chore, and the point is a quick back-and-forth.
function moveTo(stage, clientX, clientY, withX) {{
  const r = stage.getBoundingClientRect();
  if (withX) {{
    splitX = Math.min(100, Math.max(0, ((clientX - r.left) / r.width) * 100));
  }}
  splitY = Math.min(100, Math.max(0, ((clientY - r.top) / r.height) * 100));
  applyWipe();
}}
let dragging = null;
for (const id of STAGES) {{
  const stage = el(id);
  stage.addEventListener("pointerdown", e => {{
    dragging = stage;
    stage.setPointerCapture(e.pointerId);
    moveTo(stage, e.clientX, e.clientY, true);
  }});
  stage.addEventListener("pointermove", e => {{
    // Held: both bars follow. Merely hovering: only the flat one, so passing
    // over a pane on the way somewhere else does not move the cut sideways.
    if (dragging === stage) moveTo(stage, e.clientX, e.clientY, true);
    else if (!dragging) moveTo(stage, e.clientX, e.clientY, false);
  }});
  for (const end of ["pointerup", "pointercancel"]) {{
    // pointercancel as well as pointerup: a gesture the browser takes over
    // never sends an up, and the bar would stay stuck to the pointer.
    stage.addEventListener(end, e => {{
      dragging = null;
      if (stage.hasPointerCapture(e.pointerId)) stage.releasePointerCapture(e.pointerId);
    }});
  }}
  // The wheel zooms rather than scrolls. passive: false because the default
  // has to be stopped — otherwise the gesture scrolls the page strip behind.
  stage.addEventListener("wheel", e => {{
    e.preventDefault();
    const r = stage.getBoundingClientRect();
    // Grow around the pointer, so what is under it stays under it. Read on
    // every notch rather than remembered: a wheel gesture holds still, and
    // this way the zoom follows the pointer if it does move.
    ox = Math.min(100, Math.max(0, ((e.clientX - r.left) / r.width) * 100));
    oy = Math.min(100, Math.max(0, ((e.clientY - r.top) / r.height) * 100));
    const step = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    zoom = Math.min(MAX_ZOOM, Math.max(1, zoom * step));
    applyView();
  }}, {{ passive: false }});
}}

// Back to the whole page with the bars where they started: a zoomed-in corner
// with a wipe halfway across it is easy to reach and tedious to undo by hand.
el("reset").addEventListener("click", () => {{
  zoom = 1; ox = 50; oy = 50; splitX = 50; splitY = 0;
  applyWipe();
}});

for (const f of ["all", "changed"]) {{
  el("f-" + f).addEventListener("click", () => {{ filter = f; applyFilter(); }});
}}

/// Moves `delta` places through the pages the filter admits, so the arrows
/// skip what the chips are hiding rather than landing on it.
function step(delta) {{
  const visible = shown();
  const at = visible.indexOf(index);
  const next = visible[(at < 0 ? 0 : at) + delta];
  if (next !== undefined) {{
    index = next;
    render();
  }}
}}
el("prev").addEventListener("click", () => step(-1));
el("next").addEventListener("click", () => step(1));

document.addEventListener("keydown", e => {{
  if (e.key === "ArrowRight") {{ step(1); }}
  else if (e.key === "ArrowLeft") {{ step(-1); }}
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

// Nothing to filter down to: offer the button but say why it does nothing,
// rather than letting someone press it and get an empty strip.
if (!PAGES.some(p => p.diff > 0)) {{
  el("f-changed").disabled = true;
  el("f-changed").title = "No page differs";
}}

index = shown()[0] ?? 0;
applyFilter();
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

    /// Three panes, and the wipe as the only thing to operate. The controls
    /// that were removed are asserted absent by id: bringing one back without
    /// deciding to is the failure this catches.
    #[test]
    fn three_panes_and_the_wipe_are_the_whole_interface() {
        let html = index_html("a.pdf", "b.pdf", &[diff(1, 4.0)]);
        for id in ["p-before", "p-after", "wipe-before", "wipe-after-img", "wipe-diff"] {
            assert!(html.contains(&format!(r#"id="{id}""#)), "missing {id}");
        }
        // One bar per pane, and one `split` driving all of them: a bar that
        // only one pane carries is the thing this replaced.
        assert_eq!(
            html.matches(r#"<div class="handle">"#).count(),
            3,
            "each pane carries the wipe bar"
        );
        assert_eq!(
            html.matches(r#"<div class="hbar">"#).count(),
            3,
            "each pane carries the flat bar too"
        );
        assert!(
            html.contains(r#"for (const bar of document.querySelectorAll(".handle"))"#)
                && html.contains(r#"for (const bar of document.querySelectorAll(".hbar"))"#),
            "the bars are not moved together"
        );
        // Both cuts come from the same pair of numbers the bars are drawn from.
        assert!(
            html.contains("inset(var(--splity, 0%) 0 0 var(--splitx, 50%))"),
            "the reveal is not cut by both bars"
        );
        // The dot rides the upright bar rather than sitting at its middle.
        assert!(html.contains("top: var(--doty, 50%)"), "the dot is pinned to the centre");
        for gone in ["m-flip", "m-fade", "fade-group", r#"id="fade""#, r#"id="ov""#, r#"id="zoom""#] {
            assert!(!html.contains(gone), "{gone} should have been removed");
        }
        // The three stages are sized from the page's own proportions, which is
        // what keeps a wipe at 50% at 50% of the page.
        assert!(html.contains("--arn"), "the panes lost their aspect sizing");
    }

    /// Zoom is one number for three panes, and the bars must not thicken with
    /// it — they are children of what is being scaled.
    #[test]
    fn the_wheel_zooms_all_three_and_the_bars_keep_their_weight() {
        let html = index_html("a.pdf", "b.pdf", &[diff(1, 4.0)]);
        assert!(html.contains(r#"stage.addEventListener("wheel""#), "the wheel does nothing");
        // Without preventDefault the gesture scrolls the strip behind instead.
        assert!(html.contains("{ passive: false }"), "the wheel keeps its default");
        assert!(html.contains("transform: scale(var(--z, 1))"), "nothing is scaled");
        for counter in [
            "width: calc(2px / var(--z, 1))",
            "height: calc(2px / var(--z, 1))",
            "width: calc(22px / var(--z, 1))",
        ] {
            assert!(html.contains(counter), "the bars scale with the page: {counter}");
        }
        // The floor is the fitted page: zooming out past it only shrinks the
        // page inside a pane already sized to hold it.
        assert!(html.contains("Math.max(1, zoom * step)"), "zoom has no floor at fit");
        assert!(html.contains(r#"id="reset""#), "no way back to the starting view");
    }

    /// The dot sits on the part of the page being looked at.
    #[test]
    fn the_dot_does_not_hide_what_it_points_at() {
        let html = index_html("a.pdf", "b.pdf", &[diff(1, 4.0)]);
        let dot = html
            .split(".handle::after {")
            .nth(1)
            .expect("the dot is styled");
        assert!(dot.contains("opacity: .75"), "the dot is opaque");
    }

    /// On a long document the changed pages are the reason anyone opened this.
    #[test]
    fn it_opens_on_the_pages_that_differ() {
        let html = index_html("a.pdf", "b.pdf", &[diff(1, 0.0), diff(2, 3.0)]);
        assert!(
            html.contains(r#"PAGES.some(p => p.diff > 0) ? "changed" : "all""#),
            "the initial filter is not derived from what changed"
        );
        assert!(html.contains("index = shown()[0]"), "it does not start on the first shown page");
        // The button matches the state it starts in, rather than saying "All"
        // while the strip shows something else.
        assert!(html.contains(r#"<button id="f-changed" aria-pressed="true">"#));
        assert!(html.contains(r#"<button id="f-all" aria-pressed="false">"#));
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
        write(&dir, "a.pdf", "b.pdf", &[diff(1, 1.0)], &assets, &mut || {}).unwrap();

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
