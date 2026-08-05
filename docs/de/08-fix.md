# 8. Namensraum `fix::` — Normalisierung

[← `codes::`](07-codes.md) · [Inhalt](README.md) · [Weiter: `data::` →](09-data.md)

19 Operationen, die das PDF **verändern** und unter einem neuen Namen
speichern. Die Ausgangsdatei wird nie angerührt.

---

## 8.1 Wie man es benutzt

`fix::` ist der einzige Namensraum, der schreibt, und hat deshalb einen eigenen
Befehl:

```bash
pdfl fix eingabe.pdf skript.pdfl --output korrigiert.pdf
```

| Option | Zweck |
|---|---|
| `--output <datei>` | Ausgabe-PDF (Pflicht) |
| `--dry-run` | Listet die Operationen, ohne zu speichern |
| `--report json\|csv\|html\|pdf` | Format des Berichts |
| `--report-file <datei>` | Schreibt den Bericht in eine Datei |

Ein `fix::`-Aufruf unter `pdfl run` erzeugt einen Fehler, der den richtigen
Befehl nennt — damit niemand eine Datei verändert, während er glaubt, sie nur zu
prüfen.

### Wie die Operationen ablaufen

```pdfl
// Dieses Skript braucht keine checks: Das sind Befehle,
// die der Reihe nach ausgeführt werden.
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::add_page_numbers()
fix::add_watermark("DRAFT")
```

Jeder Aufruf wird **an Ort und Stelle geprüft** (nicht vorhandene Seite,
ungültige Drehung, fehlende Datei), bevor er angewandt wird. Der Bericht hält im
Feld `fixes` fest, was geschehen ist:

```json
"fixes": [
  "TrimBox set to [8.5, 8.5, 586.5, 833.5]",
  "page numbering added",
  "watermark \"DRAFT\" added"
]
```

Prüfungen und Änderungen im selben Skript zu mischen, ist völlig in Ordnung:

```pdfl
// Erst prüfen, dann ändern — hält die Bedingung nicht, steht es im Bericht
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(), "file is encrypted, cannot fix it"
}

fix::add_page_numbers()
```

---

## 8.2 Seitenrahmen

| Operation | Zweck |
|---|---|
| `fix::set_page_size(breite, höhe)` | Setzt die MediaBox aller Seiten |
| `fix::set_crop_box(x0, y0, x1, y1)` | Setzt die CropBox aller Seiten |
| `fix::set_trim_box(x0, y0, x1, y1)` | Setzt die TrimBox aller Seiten |
| `fix::set_bleed_box(x0, y0, x1, y1)` | Setzt die BleedBox aller Seiten |

Koordinaten in Punkt, von unten links nach oben rechts.

```pdfl
// Mit Einheiten schreiben, die Umrechnung geschieht von selbst
fix::set_page_size(210mm, 297mm)

// Die Datei vom Verlag hat keine Produktionsrahmen:
// TrimBox = Endformat, BleedBox = mit 3 mm Anschnitt
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)
```

---

## 8.3 Seiten

| Operation | Zweck |
|---|---|
| `fix::rotate_page([seite,] grad)` | Dreht um 90/180/270° (ohne Nummer: alle Seiten) |
| `fix::delete_page(n)` | Löscht eine Seite |
| `fix::duplicate_page(n)` | Dupliziert eine Seite (die Kopie folgt direkt) |
| `fix::reorder_pages([neue, Reihenfolge])` | Ordnet neu (jede Seite genau einmal) |
| `fix::split_document(von, bis, "aus.pdf")` | Speichert einen Seitenbereich als Datei |
| `fix::merge_documents("andere.pdf")` | Hängt die Seiten eines anderen PDF an |

Die einzige Seite eines Dokuments zu löschen, wird ausdrücklich abgelehnt.

```pdfl
fix::rotate_page(90)        // alle Seiten
fix::rotate_page(3, 180)    // nur Seite 3
fix::delete_page(1)         // entfernt den Entwurfsumschlag
fix::reorder_pages([4, 1, 2, 3])

// Umschlag und Innenteil gehen an zwei verschiedene Lieferanten
fix::split_document(1, 2, "umschlag.pdf")
fix::split_document(3, 50, "innenteil.pdf")

fix::merge_documents("anhaenge/garantie.pdf")
```

---

## 8.4 Inhalt

| Operation | Zweck |
|---|---|
| `fix::add_watermark("text")` | Graues Wasserzeichen diagonal auf allen Seiten |
| `fix::add_stamps("text")` | Roter Stempel oben rechts auf jeder Seite |
| `fix::add_page_numbers()` | Setzt `n / gesamt` in die Fußzeile |
| `fix::remove_annotations()` | Entfernt alle Anmerkungen |
| `fix::remove_attachments()` | Entfernt alle Anhänge |
| `fix::flatten_layers()` | Löst die Struktur optionaler Inhalte (OCG) auf |

```pdfl
fix::add_watermark("DRAFT — DO NOT PRINT")
fix::add_stamps("APPROVED 2026-08-02")
fix::add_page_numbers()

// Vor dem Druck: Korrekturkommentare dürfen nicht mit,
// und Anhänge machen die Datei nur schwerer
fix::remove_annotations()
fix::remove_attachments()

// Verhindert, dass eine abgeschaltete Ebene „englische Fassung“
// in der Druckerei wieder eingeschaltet wird
fix::flatten_layers()
```

---

## 8.5 Optimierung

> Die Operationen dieses Abschnitts **schreiben nur, wenn die Datei kleiner
> wird**. Fällt das Ergebnis größer aus, bleibt das Original erhalten.

| Operation | Zweck |
|---|---|
| `fix::remove_unused_resources()` | Verwirft vom Trailer aus unerreichbare Objekte |
| `fix::downsample_images([dpi])` | Tastet Bilder über der Ziel-DPI neu ab (Vorgabe 300) |
| `fix::compress_images([qualität])` | Kodiert als JPEG neu (1 bis 100, Vorgabe 85) |

Die DPI-Angabe berechnet sich aus der **tatsächlich gedruckten Größe** auf der
Seite.

> **CMYK-Bilder bleiben unangetastet.** Sie neu abzutasten hieße, über RGB zu
> gehen, und das zerstörte die Separation für die Druckvorstufe. In einer
> Druckdatei kommt die Ersparnis von den RGB-Bildern.

```pdfl
// Eine Fassung zur Freigabe per E-Mail braucht keine 300 DPI
fix::downsample_images(96)
fix::compress_images(70)
fix::remove_unused_resources()
```

### Was es hier nicht gibt

`subset_fonts` und `linearize_document` sind **keine** `fix::`-Operationen; ein
Aufruf liefert den Fehler „unbekannte Funktion“.

- **subset_fonts**: Umgesetzt und dann gemessen. Professionelle Werkzeuge betten
  ohnehin nur die benutzten Glyphen ein; der gemessene Gewinn lag bestenfalls bei
  0,5 % und sonst bei null — das Risiko, eine Schrift zu beschädigen, ist das
  nicht wert. Um zu *prüfen*, ob Schriften Teilmengen sind, nehmen Sie
  [`prepress::subset_fonts()`](06-prepress.md).
- **linearize_document**: verlangt das Erzeugen der Hinweistabellen (§ 7.14 der
  PDF-Spezifikation). Keine Rust-Bibliothek leistet das, und eine
  Teilimplementierung erkennen Betrachter nicht als „Fast Web View“.

---

## 8.6 Vollständige Beispiele

```pdfl
// druck_vorbereiten.pdfl — bereitet eine Verlagsdatei für die Druckerei auf
// Aufruf: pdfl fix verlag.pdf druck_vorbereiten.pdfl --output druck.pdf

check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask the publisher for the open version"
}

// Produktionsrahmen, die der Verlag nicht gesetzt hat
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Aufräumen: weder Korrekturanmerkungen noch Anhänge gehen in den Druck
fix::remove_annotations()
fix::remove_attachments()
fix::flatten_layers()
fix::remove_unused_resources()
```

```pdfl
// email_fassung.pdfl — leichte Fassung zur Freigabe per E-Mail
// Aufruf: pdfl fix final.pdf email_fassung.pdfl --output freigabe.pdf

fix::downsample_images(96)
fix::compress_images(70)
fix::add_watermark("PROOF — NOT THE FINAL VERSION")
fix::add_page_numbers()
```

Prüfen Sie das Ergebnis mit `pdfl` selbst:

```bash
pdfl fix final.pdf email_fassung.pdfl --output freigabe.pdf
pdfl inspect freigabe.pdf          # Größe, DPI und Warnungen der neuen Datei
```

---

[← `codes::`](07-codes.md) · [Inhalt](README.md) · [Weiter: `data::` →](09-data.md)
