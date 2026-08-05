# 2. Document types

[← The language](01-language.md) · [Index](README.md) · [Next: `text::` →](03-text.md)

Every script automatically receives the `doc` variable, representing the PDF
under analysis. From it you reach pages, fonts and images.

---

## 2.1 `doc` — the document

### Properties

| Property | Type | What it is |
|---|---|---|
| `doc.page_count` | number | Number of pages |
| `doc.title` | text | Title from metadata (empty when absent) |
| `doc.author` | text | Author from metadata (empty when absent) |
| `doc.filename` | text | Name of the analyzed file |
| `doc.pages` | list | All pages |
| `doc.fonts` | list | All fonts in use |
| `doc.images` | list | All images across all pages |

```pdfl
check "Document properties" {
  print("file:", doc.filename)
  print("pages:", doc.page_count)
  print("title:", doc.title)

  // The collections are plain lists — every list method works on them
  require doc.pages.length == doc.page_count
  require doc.fonts.length > 0
  print("images in the whole document:", doc.images.length)
}
```

### Methods

#### `doc.extract_text()`

All the text in the document, with pages separated by a newline.

```pdfl
check "Document text" {
  text = doc.extract_text()
  assert text.trim() != "", "PDF has no extractable text (images only?)"
  require text.contains("Agreement")
  print("total characters:", text.length)
}
```

---

## 2.2 `page` — the page

Pages come from `doc.pages` (inside blocks) or from the `page` variable (inside a
`rule`).

### Properties

| Property | Type | What it is |
|---|---|---|
| `page.number` | number | Page number, starting at **1** |
| `page.index` | number | Page index, starting at **0** |
| `page.width` | number | Width in points |
| `page.height` | number | Height in points |
| `page.images` | list | Images on this page |
| `page.tac` | number | Estimated maximum ink coverage (%) |
| `page.ink_coverage` | number | Estimated average ink coverage (%) |
| `page.min_stroke_width` | number/null | Thinnest stroke (pt); `null` when there are none |
| `page.has_media_box` | boolean | MediaBox is defined |
| `page.has_crop_box` | boolean | CropBox is defined |
| `page.has_trim_box` | boolean | TrimBox is defined |
| `page.has_bleed_box` | boolean | BleedBox is defined |
| `page.has_art_box` | boolean | ArtBox is defined |

```pdfl
check "Page format" {
  doc.pages.each { |page|
    // number is what users see; index is for internal arithmetic
    assert page.width > 100mm,
      "page #{page.number} is too narrow: #{page.width}pt"

    // Boxes: essential for printing
    assert page.has_trim_box,
      "page #{page.number} has no TrimBox (trim area)"
    assert page.has_bleed_box,
      "page #{page.number} has no BleedBox (bleed area)"
  }
}

check "Ink and strokes" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "page #{page.number}: #{page.tac}% ink (limit 300%)"

    // min_stroke_width can be null (page with no strokes) —
    // null is falsy, so this test is safe:
    assert !page.min_stroke_width || page.min_stroke_width >= 0.25,
      "page #{page.number} has a hairline stroke"
  }
}
```

### Methods

#### `page.extract_text()`

Text from this page only.

```pdfl
check "Blank pages" {
  blank = doc.pages.filter { |p| p.extract_text().trim() == "" }
  assert blank.length == 0,
    "#{blank.length} blank page(s): #{blank.map { |p| p.number }.join(", ")}"
}
```

---

## 2.3 `font` — the font

Fonts come from `doc.fonts`.

| Property | Type | What it is |
|---|---|---|
| `font.name` | text | Font name |
| `font.is_embedded` | boolean | Whether it is embedded in the file |

```pdfl
check "Embedded fonts" {
  // A non-embedded font gets substituted by the reader — the text changes shape
  doc.fonts.each { |font|
    assert font.is_embedded,
      "font '#{font.name}' is not embedded in the PDF"
  }
}

check "Font report" {
  print("fonts in use:", doc.fonts.map { |f| f.name }.join(", "))
  missing = doc.fonts.filter { |f| !f.is_embedded }
  print("not embedded:", missing.length)
}
```

---

## 2.4 `image` — the image

Images come from `doc.images` (all) or `page.images` (one page).

| Property | Type | What it is |
|---|---|---|
| `image.width` | number | Width in **pixels** |
| `image.height` | number | Height in **pixels** |
| `image.dpi` | number | Effective resolution (the lower of dpi_x and dpi_y) |
| `image.dpi_x` | number | Effective horizontal resolution |
| `image.dpi_y` | number | Effective vertical resolution |
| `image.color_space` | text | `DeviceRGB`, `DeviceCMYK`, `Indexed`... |
| `image.page_number` | number | Page it appears on (1-based) |
| `image.bits_per_pixel` | number | Bits per pixel |

> **DPI is effective**, computed as pixels ÷ printed size on the page — not the
> nominal value stored in metadata. That is the number that matters for print
> quality: a 1000 px image stretched to 20 cm has low DPI no matter what its
> metadata claims.

```pdfl
profile "images-for-offset" {
  const MIN_DPI = 300

  check "Resolution" {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image #{img.width}x#{img.height}px on page #{img.page_number}: #{img.dpi} DPI (minimum #{MIN_DPI})"
    }
  }

  check "Color space" {
    // Offset printing works in CMYK; RGB needs conversion
    doc.images.each { |img|
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number} — convert to CMYK"
    }
  }

  check "Images per page" {
    doc.pages.each { |page|
      // page.images only holds the images on that page
      print("page", page.number, "has", page.images.length, "image(s)")
    }
  }
}
```

---

## 2.5 `region` — an area of the page

Regions delimit rectangular areas so you can validate specific parts of a page:
footer, header, barcode area, prescription band on a medicine label.

### Creating one

```pdfl
// region(x, y, width, height [, "name"])
// The origin (0,0) is the BOTTOM-left corner, as in the PDF spec.
header = region(0, 742, 595, 100, "header")
footer = region(0, 0, 595, 60, "footer")
band = region(20mm, 250mm, 60mm, 15mm, "red band")
```

### Properties

| Property | What it is |
|---|---|
| `region.name` | Name given at creation (empty if omitted) |
| `region.x` / `region.y` | Bottom-left corner |
| `region.width` / `region.height` | Dimensions |
| `region.right` / `region.top` | Right and top edges (computed) |
| `region.area` | Area in square points |

### Methods

| Method | What it does |
|---|---|
| `region.contains_point(x, y)` | Is the point inside? |
| `region.intersects(other)` | Do the two regions overlap? |
| `region.expand(pt)` | New, larger region on every side |
| `region.inset(pt)` | New, smaller region on every side |
| `region.export_coordinates()` | `[x0, y0, x1, y1]` |

```pdfl
check "Working with regions" {
  footer = region(0, 0, 595, 60, "footer")

  require footer.name == "footer"
  require footer.top == 60.0
  require footer.right == 595.0
  require footer.area == 35700.0

  // Is a point in the footer?
  require footer.contains_point(300, 30)
  require !footer.contains_point(300, 500)

  // Overlap: useful to detect elements invading reserved areas
  header = region(0, 780, 595, 62)
  require !footer.intersects(header)

  // expand/inset return NEW regions (the original is unchanged)
  slack = footer.expand(5mm)      // 5mm larger on each side
  safe = footer.inset(3mm)        // 3mm smaller on each side
  require slack.area > footer.area
  require safe.area < footer.area
}
```

### Using regions in validations

```pdfl
profile "medicine-label" {

  check "Prescription band" {
    // The band must sit at the top and carry the legal text
    band = region(0, 700, 595, 142, "band")
    content = text::extract_from_region(1, band)
    assert content.contains("PRESCRIPTION ONLY"),
      "band is missing the mandatory text"
  }

  check "Ink in the fold area" {
    // Too much ink on the fold causes finishing problems
    fold = region(290, 0, 15, 842, "center fold")
    measured = prepress::calculate_tac_by_region(1, fold)
    assert measured.first() < 240,
      "too much ink on the fold: #{measured.first()}%"
  }

  check "Barcode in the right place" {
    code_area = region(400, 20, 180, 80, "barcode area")
    assert codes::validate_barcode_position(code_area),
      "barcode outside the reserved area"
  }
}
```

---

[← The language](01-language.md) · [Index](README.md) · [Next: `text::` →](03-text.md)
