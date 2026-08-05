# 6. `prepress::` namespace — prepress

[← `visual::`](05-visual.md) · [Index](README.md) · [Next: `codes::` →](07-codes.md)

30 functions covering what a print shop must verify before going to plate: ink
coverage, color separations, fonts, strokes and page boxes.

---

## 6.1 Ink coverage (TAC)

TAC (Total Area Coverage) is the sum of all four inks at a single point. Going
over the press limit causes smearing, slow drying and set-off between sheets. The
usual limit for offset on coated stock is 300%.

There are **two ways** to measure it, and the difference matters:

### `prepress::calculate_exact_tac([page])` — the trustworthy number

Reads the colors **declared in the file** (the PDF color operators). This is the
real value.

```pdfl
check "Ink limit" {
  tac = prepress::calculate_exact_tac()
  assert tac <= 300,
    "ink coverage of #{tac}% exceeds the 300% limit"

  // Per page, to locate the problem
  doc.pages.each { |page|
    value = prepress::calculate_exact_tac(page.number)
    assert value <= 300,
      "page #{page.number}: #{value}% ink"
  }
}
```

### `prepress::calculate_tac([page])` — the estimate

Computed by rendering the page in RGB. It is a **lower bound**: dark neutral
colors (rich black) collapse toward 100% in the estimate.

```pdfl
check "Comparing both methods" {
  print("exact (declared in the file):", prepress::calculate_exact_tac(), "%")
  print("estimated (by rendering):", prepress::calculate_tac(), "%")
  // In real files under test: exact 324% vs estimated 299%
  // — only the exact number reveals the file is over the limit.
}
```

**Always use `calculate_exact_tac` to validate an ink limit.** The estimate is
for a quick read when colors are not declared.

### `prepress::validate_tac_limits([limit])`

True when every page is within the limit (default 300). Uses the render-based
estimate.

```pdfl
check "Limit by paper profile" {
  // newsprint takes far less ink than coated stock
  assert prepress::validate_tac_limits(240),
    "exceeds the 240% limit for newsprint"
}
```

### `prepress::calculate_ink_coverage([page])`

**Average** ink coverage (%) — an indicator of consumption, not of smear risk.

```pdfl
check "Ink consumption" {
  average = prepress::calculate_ink_coverage()
  print("average coverage:", average, "%")
  // Heavily covered pages make the print run more expensive
  assert average < 200, "high average coverage: #{average}%"
}
```

### `prepress::calculate_tac_by_region(page, region)`

`[max_tac, average_coverage]` within a specific area.

```pdfl
check "Ink on the fold" {
  // Too much ink on the fold cracks during finishing
  fold = region(290, 0, 15, 842, "center fold")
  measured = prepress::calculate_tac_by_region(1, fold)

  assert measured.first() < 240,
    "TAC of #{measured.first()}% on the fold (max 240%)"
  print("average on the fold:", measured.last(), "%")
}
```

---

## 6.2 Colors and separations

### `prepress::detect_spot_colors()`

Lists special inks (Pantone, varnish, die lines) declared as `Separation` or
`DeviceN`.

> The reserved separations `All` and `None` are excluded — `All` is a
> registration mark, not an ink.

```pdfl
check "Special colors" {
  spots = prepress::detect_spot_colors()
  print("special inks:", spots.join(", "))

  // A job quoted as 4-color cannot carry extra inks
  assert spots.length == 0,
    "file uses an unquoted special ink: #{spots.join(", ")}"
}

check "Varnish expected" {
  // When the special ink IS expected
  spots = prepress::detect_spot_colors()
  assert spots.contains("Varnish"),
    "the spot varnish layer is missing"
}
```

### `prepress::detect_color_mode()`

Returns `"CMYK"`, `"RGB"`, `"Mixed"`, `"None"` or `"Other"`, based on the images.

```pdfl
check "Document color mode" {
  mode = prepress::detect_color_mode()
  assert mode == "CMYK" || mode == "None",
    "document is #{mode} — offset printing requires CMYK"
}
```

### `prepress::validate_color_space(space)`

True when **all** images are in the given space.

```pdfl
check "Everything in CMYK" {
  assert prepress::validate_color_space("DeviceCMYK"),
    "there are images outside CMYK"
}
```

### `prepress::compare_colors_delta_e(color_a, color_b)`

Perceptual difference between two colors (Delta-E CIE76). Colors are lists:
4 values = CMYK, 3 = RGB, 1 = gray.

Rule of thumb: ΔE below 1 is imperceptible; up to 3 is acceptable in print; above
5 is visibly different.

```pdfl
check "Brand color" {
  // The approved corporate blue
  brand = [1.0, 0.6, 0.0, 0.1]
  used = [1.0, 0.62, 0.0, 0.12]

  difference = prepress::compare_colors_delta_e(brand, used)
  assert difference < 3.0,
    "brand color out of tolerance (ΔE #{difference})"
}
```

### `prepress::detect_rich_black()`

True when black is built from several inks (K ≥ 60% with C+M+Y ≥ 20%).

```pdfl
check "Correct black for text" {
  // Small text in rich black shows registration wobble
  assert !prepress::detect_rich_black(),
    "rich black found — use flat black (0/0/0/100) for text"
}
```

### `prepress::validate_overprint_settings()`

True when **no** overprint is enabled.

```pdfl
check "Overprint" {
  // Accidental overprint makes elements disappear on press
  assert prepress::validate_overprint_settings(),
    "overprint is enabled — verify that it is intentional"
}
```

### `prepress::validate_output_intent([name])`

Without an argument: true when an Output Intent is declared. With a name: true
when the intent contains that text.

```pdfl
check "Output profile" {
  assert prepress::validate_output_intent(),
    "PDF has no Output Intent — the shop cannot know the target color profile"

  // Requiring a specific profile
  assert prepress::validate_output_intent("Coated FOGRA39"),
    "Output Intent differs from the shop standard"
}
```

### `prepress::check_rendering_intent([expected])`

Without an argument: lists the declared intents. With one: true when all of them
match.

```pdfl
check "Rendering intent" {
  print("intents in the file:", prepress::check_rendering_intent().join(", "))

  assert prepress::check_rendering_intent("RelativeColorimetric"),
    "rendering intent differs from the production standard"
}
```

---

## 6.3 Strokes and thin lines

Very thin lines disappear in print or come out uneven.

### `prepress::detect_hairlines([limit])`

True when a stroke falls below the limit (default 0.25 pt).

```pdfl
check "No hairlines" {
  assert !prepress::detect_hairlines(0.25),
    "there are strokes below 0.25 pt — they will disappear in print"
}
```

### `prepress::detect_hairlines_exact()`

True when a stroke has **zero width** — the classic PostScript hairline, which
the press renders at the device minimum (unpredictable).

```pdfl
check "Zero-width stroke" {
  assert !prepress::detect_hairlines_exact(),
    "there is a stroke with 0 width — set a real thickness"
}
```

### `prepress::detect_fine_lines([limit])`

Like `detect_hairlines`, with a wider limit (default 1 pt).

```pdfl
check "Thin lines over color" {
  // Over a background, lines below 1pt vanish
  assert !prepress::detect_fine_lines(1.0),
    "there are lines below 1 pt"
}
```

### `prepress::validate_minimum_stroke_width(minimum)`

True when no stroke falls below the required minimum.

```pdfl
check "Minimum thickness in the contract" {
  assert prepress::validate_minimum_stroke_width(0.5),
    "the shop contract requires strokes of at least 0.5 pt"
}
```

---

## 6.4 Fonts

### `prepress::list_fonts()`

Names of the fonts in use.

```pdfl
check "Font inventory" {
  fonts = prepress::list_fonts()
  print("fonts:", fonts.join(", "))
  assert fonts.length <= 8,
    "#{fonts.length} different fonts — inconsistent design?"
}
```

### `prepress::validate_font_embedding()`

True when every font is embedded.

```pdfl
check "Embedded fonts" {
  assert prepress::validate_font_embedding(),
    "there are non-embedded fonts — the text will change at the RIP"
}
```

### `prepress::detect_text_substitution()`

Lists the **non-embedded** fonts, which the reader will substitute.

```pdfl
check "Which fonts are missing" {
  missing = prepress::detect_text_substitution()
  assert missing.length == 0,
    "fonts not embedded: #{missing.join(", ")}"
}
```

### `prepress::detect_missing_glyphs()`

Lists fonts with no widths table — the reader has to guess the metrics, which
misaligns the text.

```pdfl
check "Complete metrics" {
  problems = prepress::detect_missing_glyphs()
  assert problems.length == 0,
    "fonts without a widths table: #{problems.join(", ")}"
}
```

### `prepress::subset_fonts()`

True when every embedded font is subset (only the glyphs actually used), which
keeps the file lean.

```pdfl
check "Fonts are subset" {
  assert prepress::subset_fonts(),
    "a full font is embedded — the file is larger than it needs to be"
}
```

### `prepress::check_font_licensing()`

Lists fonts that carry licensing risk: Type3 or non-embedded.

```pdfl
check "Licensing" {
  risky = prepress::check_font_licensing()
  assert risky.length == 0,
    "fonts with licensing risk: #{risky.join(", ")}"
}
```

### `prepress::validate_font_size([minimum])`

True when no text falls below the minimum size (default 6 pt).

```pdfl
check "Legibility" {
  // Regulators require minimum body size on package inserts;
  // contracts often have similar requirements
  assert prepress::validate_font_size(6),
    "there is text below 6 pt — illegible once printed"
}
```

---

## 6.5 Pages and boxes

PDF boxes define the working areas: **MediaBox** (sheet), **BleedBox** (bleed),
**TrimBox** (final trim), **CropBox** (viewing), **ArtBox** (content).

### `prepress::get_page_size([page])`

`[width, height]` in points.

```pdfl
check "Format" {
  size = prepress::get_page_size(1)
  print("page 1:", size.first(), "x", size.last(), "pt")
  assert abs(size.first() - 595.0) < 5, "width is outside A4"
}
```

### `prepress::get_page_boxes([page])`

List of the defined boxes, formatted as text.

```pdfl
check "Boxes on the first page" {
  prepress::get_page_boxes(1).each { |box| print(box) }
  // Sample output:
  //   MediaBox: [0, 0, 467.2, 665.6]
  //   TrimBox: [35.2, 35.2, 432, 630.4]
}
```

### `validate_media_box()`, `validate_trim_box()`, `validate_bleed_box()`

True when the box exists on **every** page.

```pdfl
check "Boxes required for printing" {
  require prepress::validate_media_box()
  assert prepress::validate_trim_box(),
    "no TrimBox — the shop cannot know where to trim"
  assert prepress::validate_bleed_box(),
    "no BleedBox — no bleed area is defined"
}
```

### `prepress::check_page_geometry([margin])`

True when the BleedBox exceeds the TrimBox by the given margin on **all sides**,
on every page. Default: 3 mm.

```pdfl
check "Sufficient bleed" {
  // Use unit literals: readable, and conversion is automatic
  assert prepress::check_page_geometry(3mm),
    "bleed smaller than 3 mm on some page"

  // Packaging shops usually demand more
  assert prepress::check_page_geometry(5mm),
    "this shop requires 5 mm of bleed"
}
```

---

## 6.6 Complete example

```pdfl
// offset_magazine.pdfl — full preflight for offset printing
// Usage: pdfl run offset_magazine.pdfl magazine.pdf --output html --output-file report.html
profile "offset-magazine" {

  const TAC_LIMIT = 300%
  const BLEED = 3mm
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress", "colors"] {
    // Always the exact figure when validating a limit
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMIT,
        "page #{page.number}: #{tac}% ink (limit #{TAC_LIMIT}%)"
    }
    print("average coverage:", prepress::calculate_ink_coverage(), "%")
  }

  check "Colors" tags: ["prepress", "colors"] {
    assert prepress::detect_color_mode() != "RGB", "document is in RGB"
    spots = prepress::detect_spot_colors()
    assert spots.length == 0, "unquoted special ink: #{spots.join(", ")}"
    assert !prepress::detect_rich_black(), "rich black in text"
    assert prepress::validate_output_intent(), "no Output Intent"
  }

  check "Fonts" tags: ["fonts"] {
    missing = prepress::detect_text_substitution()
    assert missing.length == 0, "fonts not embedded: #{missing.join(", ")}"
    assert prepress::validate_font_size(6), "text below 6 pt"
    print("fonts:", prepress::list_fonts().join(", "))
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25), "strokes below 0.25 pt"
    assert !prepress::detect_hairlines_exact(), "stroke with 0 width"
  }

  check "Geometry" tags: ["prepress", "boxes"] {
    require prepress::validate_trim_box()
    require prepress::validate_bleed_box()
    assert prepress::check_page_geometry(BLEED),
      "bleed smaller than 3 mm"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI"
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number}"
    }
  }
}
```

---

[← `visual::`](05-visual.md) · [Index](README.md) · [Next: `codes::` →](07-codes.md)
