# 6. Namensraum `prepress::` — Druckvorstufe

[← `visual::`](05-visual.md) · [Inhalt](README.md) · [Weiter: `codes::` →](07-codes.md)

30 Funktionen für das, was eine Druckerei vor dem Plattenbelichten prüfen muss:
Gesamtfarbauftrag, Separationen, Schriften, Linienstärken, Seitenrahmen.

---

## 6.1 Gesamtfarbauftrag (TAC)

Der TAC (Total Area Coverage) ist die Summe der vier Farben an einem Punkt. Über
der Grenze der Maschine: Schmieren, schlechte Trocknung, Ablegen. Im Offsetdruck
auf gestrichenem Papier liegt die übliche Grenze bei 300 %.

Es gibt **zwei** Arten, ihn zu messen, und der Unterschied zählt.

| Funktion | Zweck |
|---|---|
| `prepress::calculate_exact_tac([seite])` | Rechnung aus den **deklarierten Farben** der Datei (exakt) |
| `prepress::calculate_tac([seite])` | Schätzung über RGB-Rendering (**untere Schranke**) |
| `prepress::validate_tac_limits([grenze])` | Wahr, wenn alle Seiten unter der Grenze bleiben (Vorgabe 300) |
| `prepress::calculate_ink_coverage([seite])` | Mittlerer Farbauftrag (%) |
| `prepress::calculate_tac_by_region(seite, region)` | `[max. TAC, Mittelwert]` des Bereichs |

Die Schätzung drückt satte Tiefschwarztöne Richtung 100 %.

```pdfl
check "Ink limit" {
  // Zum Prüfen einer Grenze immer den exakten TAC verwenden
  doc.pages.each { |page|
    tac = prepress::calculate_exact_tac(page.number)
    assert tac <= 300, "page #{page.number}: #{tac}% ink"
  }

  print("exact (declared in the file):", prepress::calculate_exact_tac(), "%")
  print("estimated (by rendering):", prepress::calculate_tac(), "%")
  // An einer echten Datei gemessen: exakt 324 %, geschätzt 299 %
  // — nur der exakte Wert zeigt die Überschreitung.

  // Zu viel Farbe im Falz bricht in der Weiterverarbeitung
  falz = region(290, 0, 15, 842, "center fold")
  messung = prepress::calculate_tac_by_region(1, falz)
  assert messung.first() < 240, "TAC of #{messung.first()}% on the fold (max 240%)"
}
```

---

## 6.2 Farben und Separationen

| Funktion | Zweck |
|---|---|
| `prepress::detect_spot_colors()` | Liste der Sonderfarben (Separation / DeviceN) |
| `prepress::detect_color_mode()` | `"CMYK"`, `"RGB"`, `"Mixed"`, `"None"` oder `"Other"` |
| `prepress::validate_color_space(raum)` | Wahr, wenn alle Bilder in diesem Farbraum liegen |
| `prepress::compare_colors_delta_e(a, b)` | Delta-E (CIE76) zwischen zwei Farben |
| `prepress::detect_rich_black()` | Wahr, wenn es ein aus mehreren Farben aufgebautes Schwarz gibt |
| `prepress::validate_overprint_settings()` | Wahr, wenn Überdrucken nicht aktiviert ist |
| `prepress::validate_output_intent([name])` | Gibt es ein Ausgabeprofil / passt der Name? |
| `prepress::check_rendering_intent([erwartet])` | Listet oder prüft das Rendering Intent |

Farben werden als Listen übergeben: 4 Werte = CMYK, 3 = RGB, 1 = Grau.
Anhaltspunkte für Delta-E: unter 1 nicht wahrnehmbar, bis 3 im Druck
hinnehmbar, über 5 deutlich verschieden.

> Die reservierten Separationen `All` und `None` erscheinen nicht: `All` dient
> den Passermarken, es ist keine Farbe.

```pdfl
check "Colors" {
  sonder = prepress::detect_spot_colors()
  assert sonder.length == 0, "file uses an unquoted special ink: #{sonder.join(", ")}"

  modus = prepress::detect_color_mode()
  assert modus == "CMYK" || modus == "None",
    "document is #{modus} — offset printing requires CMYK"

  // Toleranz für eine Hausfarbe
  abweichung = prepress::compare_colors_delta_e([1.0, 0.6, 0.0, 0.1], [1.0, 0.62, 0.0, 0.12])
  assert abweichung < 3.0, "brand color out of tolerance (ΔE #{abweichung})"

  // Tiefschwarz unter kleiner Schrift macht Passerfehler sichtbarer
  assert !prepress::detect_rich_black(),
    "rich black found — use flat black (0/0/0/100) for text"

  // Unbeabsichtigtes Überdrucken lässt Elemente verschwinden
  assert prepress::validate_overprint_settings(),
    "overprint is enabled — verify that it is intentional"

  assert prepress::validate_output_intent(),
    "PDF has no Output Intent — the shop cannot know the target color profile"
}
```

---

## 6.3 Linienstärken

| Funktion | Zweck |
|---|---|
| `prepress::detect_hairlines([grenze])` | Wahr bei Linien unter der Schwelle (Vorgabe 0,25 pt) |
| `prepress::detect_hairlines_exact()` | Wahr, wenn es eine Linie mit Stärke 0 gibt |
| `prepress::detect_fine_lines([grenze])` | Dasselbe (Vorgabe 1 pt) |
| `prepress::validate_minimum_stroke_width(min)` | Wahr, wenn alle Linien das Minimum erreichen |

Stärke 0 ist die klassische Haarlinie aus PostScript: Das Gerät gibt sie in
seiner kleinstmöglichen Breite aus — also unvorhersehbar.

```pdfl
check "Strokes" {
  assert !prepress::detect_hairlines(0.25),
    "there are strokes below 0.25 pt — they will disappear in print"
  assert !prepress::detect_hairlines_exact(),
    "there is a stroke with 0 width — set a real thickness"
  assert prepress::validate_minimum_stroke_width(0.5),
    "the shop contract requires strokes of at least 0.5 pt"
}
```

---

## 6.4 Schriften

| Funktion | Zweck |
|---|---|
| `prepress::list_fonts()` | Namen der verwendeten Schriften |
| `prepress::validate_font_embedding()` | Wahr, wenn alle eingebettet sind |
| `prepress::detect_text_substitution()` | Liste der nicht eingebetteten Schriften |
| `prepress::detect_missing_glyphs()` | Schriften ohne Breitentabelle |
| `prepress::subset_fonts()` | Wahr, wenn alle eingebetteten Schriften Teilmengen sind |
| `prepress::check_font_licensing()` | Schriften mit Lizenzrisiko (Type3 oder nicht eingebettet) |
| `prepress::validate_font_size([min])` | Wahr, wenn kein Text unter der Mindestgröße liegt (Vorgabe 6 pt) |

```pdfl
check "Fonts" {
  print("fonts:", prepress::list_fonts().join(", "))

  fehlend = prepress::detect_text_substitution()
  assert fehlend.length == 0,
    "fonts not embedded (text will change at the RIP): #{fehlend.join(", ")}"

  probleme = prepress::detect_missing_glyphs()
  assert probleme.length == 0,
    "fonts without a widths table: #{probleme.join(", ")}"

  assert prepress::subset_fonts(),
    "a full font is embedded — the file is larger than it needs to be"

  riskant = prepress::check_font_licensing()
  assert riskant.length == 0, "fonts with licensing risk: #{riskant.join(", ")}"

  // Beipackzettel und Verträge haben eine gesetzliche Mindestgröße
  assert prepress::validate_font_size(6),
    "there is text below 6 pt — illegible once printed"
}
```

---

## 6.5 Seiten und Rahmen

Die Rahmen des PDF legen die Arbeitsbereiche fest: **MediaBox** (das Papier),
**BleedBox** (der Anschnitt), **TrimBox** (das Endformat), **CropBox** (die
Anzeige), **ArtBox** (der Inhalt).

| Funktion | Zweck |
|---|---|
| `prepress::get_page_size([seite])` | `[Breite, Höhe]` in Punkt |
| `prepress::get_page_boxes([seite])` | Liste der definierten Rahmen |
| `prepress::validate_media_box()` | Wahr, wenn alle Seiten eine MediaBox haben |
| `prepress::validate_trim_box()` | Wahr, wenn alle eine TrimBox haben |
| `prepress::validate_bleed_box()` | Wahr, wenn alle eine BleedBox haben |
| `prepress::check_page_geometry([rand])` | Wahr, wenn der Anschnitt an allen vier Seiten reicht (Vorgabe 3 mm) |

```pdfl
check "Geometry" {
  groesse = prepress::get_page_size(1)
  assert abs(groesse.first() - 595.0) < 5, "width is outside A4"
  prepress::get_page_boxes(1).each { |rahmen| print(rahmen) }

  assert prepress::validate_trim_box(),
    "no TrimBox — the shop cannot know where to trim"
  assert prepress::validate_bleed_box(),
    "no BleedBox — no bleed area is defined"

  // Das Einheiten-Literal liest sich gut und rechnet selbst um
  assert prepress::check_page_geometry(3mm),
    "bleed smaller than 3 mm on some page"
}
```

---

## 6.6 Vollständiges Beispiel

```pdfl
// offset_magazin.pdfl — vollständige Druckvorstufenprüfung für Offset
// Aufruf: pdfl run offset_magazin.pdfl magazin.pdf --output html --output-file bericht.html
profile "offset-magazine" {

  const TAC_GRENZE = 300%
  const ANSCHNITT = 3mm
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress", "colors"] {
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_GRENZE,
        "page #{page.number}: #{tac}% ink (limit #{TAC_GRENZE}%)"
    }
    print("average coverage:", prepress::calculate_ink_coverage(), "%")
  }

  check "Colors" tags: ["prepress", "colors"] {
    assert prepress::detect_color_mode() != "RGB", "document is in RGB"
    sonder = prepress::detect_spot_colors()
    assert sonder.length == 0, "unquoted special ink: #{sonder.join(", ")}"
    assert !prepress::detect_rich_black(), "rich black in text"
    assert prepress::validate_output_intent(), "no Output Intent"
  }

  check "Fonts" tags: ["fonts"] {
    fehlend = prepress::detect_text_substitution()
    assert fehlend.length == 0, "fonts not embedded: #{fehlend.join(", ")}"
    assert prepress::validate_font_size(6), "text below 6 pt"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25), "strokes below 0.25 pt"
    assert !prepress::detect_hairlines_exact(), "stroke with 0 width"
  }

  check "Geometry" tags: ["prepress", "boxes"] {
    require prepress::validate_trim_box()
    require prepress::validate_bleed_box()
    assert prepress::check_page_geometry(ANSCHNITT), "bleed smaller than 3 mm"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI"
      assert img.color_space != "DeviceRGB", "RGB image on page #{img.page_number}"
    }
  }
}
```

---

[← `visual::`](05-visual.md) · [Inhalt](README.md) · [Weiter: `codes::` →](07-codes.md)
