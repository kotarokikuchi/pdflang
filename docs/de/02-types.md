# 2. Typen des Dokuments

[← Die Sprache](01-language.md) · [Inhalt](README.md) · [Weiter: `text::` →](03-text.md)

Jedes Skript erhält automatisch die Variable `doc`, die das analysierte PDF
darstellt. Von ihr aus erreicht man Seiten, Schriften und Bilder.

---

## 2.1 `doc` — das Dokument

| Eigenschaft | Typ | Inhalt |
|---|---|---|
| `doc.page_count` | Zahl | Anzahl der Seiten |
| `doc.title` | Text | Titel aus den Metadaten (leer, wenn er fehlt) |
| `doc.author` | Text | Autor aus den Metadaten (leer, wenn er fehlt) |
| `doc.filename` | Text | Name der analysierten Datei |
| `doc.pages` | Liste | Alle Seiten |
| `doc.fonts` | Liste | Alle verwendeten Schriften |
| `doc.images` | Liste | Alle Bilder über alle Seiten hinweg |

Methode: `doc.extract_text()` — der Text des ganzen Dokuments, Seiten durch
Zeilenumbrüche getrennt.

```pdfl
check "Document properties" {
  print("file:", doc.filename)
  print("pages:", doc.page_count)

  // Diese Sammlungen sind gewöhnliche Listen — alle Methoden funktionieren
  require doc.pages.length == doc.page_count
  require doc.fonts.length > 0

  text = doc.extract_text()
  assert text.trim() != "", "PDF has no extractable text (images only?)"
  print("total characters:", text.length)
}
```

---

## 2.2 `page` — die Seite

Seiten kommen aus `doc.pages` (in einem Block) oder aus der Variable `page` (in
einer `rule`).

| Eigenschaft | Typ | Inhalt |
|---|---|---|
| `page.number` | Zahl | Seitennummer, ab **1** |
| `page.index` | Zahl | Index, ab **0** |
| `page.width` / `page.height` | Zahl | Breite / Höhe in Punkt |
| `page.images` | Liste | Bilder dieser Seite |
| `page.tac` | Zahl | Geschätzter maximaler Gesamtfarbauftrag (%) |
| `page.ink_coverage` | Zahl | Geschätzter mittlerer Farbauftrag (%) |
| `page.min_stroke_width` | Zahl/null | Dünnste Linie (pt); `null`, wenn es keine Linie gibt |
| `page.has_media_box` usw. | Wahrheitswert | `has_crop_box`, `has_trim_box`, `has_bleed_box`, `has_art_box` |

Methode: `page.extract_text()` — nur der Text dieser Seite.

```pdfl
check "Page format" {
  doc.pages.each { |page|
    // number ist die Nummer für Menschen, index dient internen Rechnungen
    assert page.width > 100mm,
      "page #{page.number} is too narrow: #{page.width}pt"

    // Die Rahmen: für den Druck unverzichtbar
    assert page.has_trim_box, "page #{page.number} has no TrimBox (trim area)"
    assert page.has_bleed_box, "page #{page.number} has no BleedBox (bleed area)"

    assert page.tac <= 300,
      "page #{page.number}: #{page.tac}% ink (limit 300%)"

    // min_stroke_width kann null sein (keine Linie auf der Seite).
    // null ist falsch, deshalb ist das hier sicher:
    assert !page.min_stroke_width || page.min_stroke_width >= 0.25,
      "page #{page.number} has a hairline stroke"
  }
}

check "Blank pages" {
  leere = doc.pages.filter { |p| p.extract_text().trim() == "" }
  assert leere.length == 0,
    "#{leere.length} blank page(s): #{leere.map { |p| p.number }.join(", ")}"
}
```

---

## 2.3 `font` — die Schrift

Kommt aus `doc.fonts`. Eigenschaften: `font.name` (Name) und `font.is_embedded`
(eingebettet oder nicht).

```pdfl
check "Embedded fonts" {
  // Eine nicht eingebettete Schrift ersetzt der Betrachter — der Text ändert sich
  doc.fonts.each { |font|
    assert font.is_embedded,
      "font '#{font.name}' is not embedded in the PDF"
  }
  print("fonts in use:", doc.fonts.map { |f| f.name }.join(", "))
}
```

---

## 2.4 `image` — das Bild

Kommt aus `doc.images` (alle) oder `page.images` (die einer Seite).

| Eigenschaft | Inhalt |
|---|---|
| `image.width` / `image.height` | Breite / Höhe in **Pixeln** |
| `image.dpi` | Effektive Auflösung (der kleinere Wert von dpi_x und dpi_y) |
| `image.dpi_x` / `image.dpi_y` | Effektive Auflösung waagerecht / senkrecht |
| `image.color_space` | `DeviceRGB`, `DeviceCMYK`, `Indexed` … |
| `image.page_number` | Seite, auf der es steht (ab 1) |
| `image.bits_per_pixel` | Bittiefe |

> **Die DPI-Angabe ist effektiv**, berechnet als „Pixel ÷ gedruckte Größe auf der
> Seite“, nicht der Nennwert aus den Metadaten. Diese Zahl entscheidet über die
> Druckqualität: Ein Bild mit 1000 px, auf 20 cm gezogen, hat eine niedrige
> Auflösung, ganz gleich was seine Metadaten behaupten.

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
    // Offset arbeitet in CMYK; RGB muss umgewandelt werden
    doc.images.each { |img|
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number} — convert to CMYK"
    }
  }

  check "Images per page" {
    doc.pages.each { |page|
      print("page", page.number, "has", page.images.length, "image(s)")
    }
  }
}
```

---

## 2.5 `region` — ein Bereich der Seite

Eine Region grenzt einen Teil der Seite durch ein Rechteck ab. Damit prüft man
Fußzeilen, Kopfzeilen, den Platz eines Strichcodes oder ein
Pflichtangaben-Band.

Erzeugen: `region(x, y, breite, höhe [, "name"])`, der Ursprung (0,0) liegt wie
im PDF unten links.

| Eigenschaft | Inhalt | | Methode | Zweck |
|---|---|---|---|---|
| `region.name` | Bei der Erzeugung vergebener Name | | `contains_point(x, y)` | Liegt der Punkt darin? |
| `region.x` / `region.y` | Untere linke Ecke | | `intersects(andere)` | Überlappen sich beide Regionen? |
| `region.width` / `region.height` | Maße | | `expand(pt)` | Neue, allseitig vergrößerte Region |
| `region.right` / `region.top` | Rechter / oberer Rand (berechnet) | | `inset(pt)` | Neue, allseitig verkleinerte Region |
| `region.area` | Fläche (Quadratpunkt) | | `export_coordinates()` | `[x0, y0, x1, y1]` |

```pdfl
check "Working with regions" {
  fuss = region(0, 0, 595, 60, "footer")

  require fuss.name == "footer"
  require fuss.top == 60.0
  require fuss.right == 595.0
  require fuss.area == 35700.0
  require fuss.contains_point(300, 30)
  require !fuss.contains_point(300, 500)

  // Überlappungserkennung: nützlich, um ein Element zu finden,
  // das in einen reservierten Bereich hineinragt
  kopf = region(0, 780, 595, 62)
  require !fuss.intersects(kopf)

  // expand/inset liefern eine NEUE Region (die ursprüngliche bleibt)
  require fuss.expand(5mm).area > fuss.area
  require fuss.inset(3mm).area < fuss.area
}

profile "medicine-label" {
  check "Prescription band" {
    // Das Band muss oben stehen und den Pflichttext tragen
    band = region(0, 700, 595, 142, "band")
    assert text::extract_from_region(1, band).contains("PRESCRIPTION ONLY"),
      "band is missing the mandatory text"
  }

  check "Ink in the fold area" {
    // Zu viel Farbe im Falz bricht in der Weiterverarbeitung
    falz = region(290, 0, 15, 842, "center fold")
    messung = prepress::calculate_tac_by_region(1, falz)
    assert messung.first() < 240,
      "too much ink on the fold: #{messung.first()}%"
  }

  check "Barcode in the right place" {
    codebereich = region(400, 20, 180, 80, "barcode area")
    assert codes::validate_barcode_position(codebereich),
      "barcode outside the reserved area"
  }
}
```

---

[← Die Sprache](01-language.md) · [Inhalt](README.md) · [Weiter: `text::` →](03-text.md)
