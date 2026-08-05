# 12. Rezepte

[← Kommandozeile](11-cli.md) · [Inhalt](README.md)

Vollständige Fälle, direkt übernehmbar. Jeder löst ein reales Problem aus der
Praxis.

---

## 12.1 Druckerei: Vorstufenprüfung eines Offset-Magazins

**Das Problem:** Der Kunde liefert die Datei; vor dem Plattenbelichten müssen
Farben, Schriften, Bilder und Anschnitt geprüft werden. Ein Fehler, der später
auffällt, kostet die ganze Auflage.

`profile/offset.pdfl`:

```pdfl
profile "offset-magazine" {

  const TAC_GRENZE = 300%      // Farbgrenze auf gestrichenem Papier
  const ANSCHNITT = 3mm        // Vorgabe des Ausschießens
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress"] {
    // Der exakte TAC liest die in der Datei deklarierten Farben; die
    // Schätzung über das Rendering unterschätzt Tiefschwarz und übersieht
    // Überschreitungen
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_GRENZE,
        "page #{page.number}: #{tac}% ink (limit #{TAC_GRENZE}%)"
    }
  }

  check "Colors" tags: ["prepress"] {
    assert prepress::detect_color_mode() != "RGB",
      "document is in RGB — convert to CMYK"

    sonder = prepress::detect_spot_colors()
    assert sonder.length == 0, "unquoted special ink: #{sonder.join(", ")}"

    assert !prepress::detect_rich_black(),
      "rich black detected — use 0/0/0/100 for text"
  }

  check "Fonts" tags: ["fonts"] {
    lose = prepress::detect_text_substitution()
    assert lose.length == 0,
      "fonts not embedded (text will change at the RIP): #{lose.join(", ")}"
    assert prepress::validate_font_size(6),
      "there is text below 6 pt — illegible once printed"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25),
      "strokes below 0.25 pt disappear in print"
    assert !prepress::detect_hairlines_exact(),
      "there is a stroke with 0 width — set a real thickness"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI (minimum #{MIN_DPI})"
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number}"
    }
  }

  check "Geometry" tags: ["prepress"] {
    assert prepress::validate_trim_box(),
      "no TrimBox — imposition cannot know where to trim"
    assert prepress::validate_bleed_box(), "no BleedBox — no bleed is defined"
    assert prepress::check_page_geometry(ANSCHNITT),
      "bleed smaller than 3 mm on some page"
  }
}
```

**Am Schalter:**

```bash
# HTML-Bericht, der an den Kunden zurückgeht
pdfl run profile/offset.pdfl kunde.pdf --output html --output-file bericht.html
```

**Als überwachter Ordner:** Die Bedienperson legt die Datei ab, der Bericht
erscheint daneben.

```bash
pdfl watch inbox/ --script profile/offset.pdfl \
  --output-dir berichte/ --report html
```

---

## 12.2 Rechtsverlag: Vertragsprüfung vor der Veröffentlichung

**Das Problem:** Verträge und Policen müssen die Pflichtklauseln tragen, keinen
Entwurfstext behalten, keine personenbezogenen Daten preisgeben und durchsuchbar
bleiben.

`profile/recht.pdfl`:

```pdfl
profile "standard-contract" {

  check "Mandatory clauses" tags: ["legal"] {
    // Vom Rechtsbereich gepflegtes Glossar
    fehlend = data::validate_against_reference("begriffe/klauseln.txt")
    assert fehlend.length == 0, "missing clauses: #{fehlend.join("; ")}"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "document marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("X{3,}"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    // Steuernummern zählen nur mit korrekter Prüfziffer,
    // Beispielnummern lösen also keinen Fehlalarm aus
    gefunden = text::detect_personal_data()
    assert gefunden.length == 0, "personal data in the document: #{gefunden.join("; ")}"
  }

  check "Numbering and initials" tags: ["legal"] {
    doc.pages.each { |page|
      fuss = region(0, 0, page.width, 60, "footer")
      inhalt = text::extract_from_region(page.number, fuss).trim()
      assert inhalt != "",
        "page #{page.number} has no numbering/initials in the footer"
    }
  }

  check "Searchable text" tags: ["accessibility"] {
    assert !text::detect_rasterized_text(),
      "there are scanned pages — text cannot be searched or read by screen readers"
  }
}
```

---

## 12.3 Pharmalabor: Beipackzettel mit Chargencode

**Das Problem:** Der Beipackzettel muss die vorgeschriebenen Texte tragen, und
der Strichcode muss auf das richtige Produkt zeigen. Codes zwischen Produkten zu
vertauschen, ist der teuerste Fehler dieser Branche.

`profile/beipackzettel.pdfl`:

```pdfl
profile "regulated-insert" {

  check "Mandatory texts" tags: ["regulatory"] {
    fehlend = data::validate_against_reference("datenbanken/pflichttexte.txt")
    assert fehlend.length == 0, "mandatory texts missing: #{fehlend.join("; ")}"
  }

  check "Legibility" tags: ["regulatory"] {
    assert prepress::validate_font_size(6), "there is text below 6 pt"
  }

  check "Barcode" tags: ["codes", "critical"] {
    assert codes::detect_barcodes(), "insert has no barcode"

    code = codes::decode_barcode(1)
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"

    // Diese Prüfung fängt den teuersten Fehler ab:
    // der Code des einen Produkts mit dem Text eines anderen
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Approved product" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    produkt = data::query_gtin(code)
    assert produkt, "GTIN #{code} is not in the product database"

    name = produkt.get(2)
    assert text::require_text(name),
      "the name '#{name}' does not appear on the insert"
    print("product verified:", name)
  }

  check "Code position" tags: ["layout"] {
    bereich = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(bereich),
      "code outside the reserved area — risk of being trimmed off"
  }
}
```

```bash
PDFL_DATA_DIR=./datenbanken pdfl run profile/beipackzettel.pdfl beipackzettel_v3.pdf
```

---

## 12.4 Freigabe: Vergleich mit der freigegebenen Fassung

**Das Problem:** Der Kunde hat v1 freigegeben. v2 kommt mit „wir haben nur ein
Wort geändert“. Das zu glauben, wird teuer.

```bash
# HTML, das zeigt, was sich tatsächlich geändert hat
pdfl compare freigegeben/katalog_v1.pdf erhalten/katalog_v2.pdf \
  --normalize --ignore-dates \
  --output html --output-file unterschiede.html

echo "exit: $?"   # 0 identisch · 1 nur Metadaten · 2 Inhalt geändert
```

Um auch das **Aussehen** zu prüfen, nicht nur den Text:

```pdfl
// profile/treue.pdfl
profile "visual-fidelity" {

  const FREIGEGEBEN = "freigegeben/katalog_v1.pdf"

  check "Pages visually identical" tags: ["approval"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, FREIGEGEBEN)
      assert ssim > 0.99,
        "page #{page.number} changed visually (SSIM #{ssim}, #{visual::pixel_diff(page.number, FREIGEGEBEN)}% of pixels)"
    }
  }

  check "No image replaced" tags: ["approval"] {
    doc.pages.each { |page|
      assert !visual::detect_image_replacement(page.number, FREIGEGEBEN),
        "page #{page.number}: image swapped compared to the approved version"
    }
  }
}
```

---

## 12.5 CI/CD: Prüfung im Stapel

**Das Problem:** Jede Datei, die ins Repository kommt, muss die Vorstufenprüfung
bestehen, ohne dass jemand sie von Hand startet.

`.github/workflows/preflight.yml`:

```yaml
name: PDF preflight

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install pdfl
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # automatisches Actions-Token, keine Einrichtung
        run: |
          gh release download --repo kotarokikuchi/pdflang \
            --pattern 'pdfl_*_amd64.deb'
          sudo dpkg -i pdfl_*_amd64.deb

      - name: Check the scripts themselves
        run: |
          for f in profile/*.pdfl; do
            pdfl lint "$f"
            pdfl fmt "$f" --check
          done

      - name: Preflight every PDF
        run: |
          pdfl watch dateien/ --script profile/offset.pdfl \
            --output-dir berichte/ --once

      - name: Publish the reports
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: berichte
          path: berichte/
```

---

## 12.6 Eine Verlagsdatei für die Druckerei aufbereiten

```pdfl
// profile/vorbereiten.pdfl
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask for the open version"
}

// Produktionsrahmen, die der Verlag nicht gesetzt hat
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Aufräumen
fix::remove_annotations()      // Korrekturkommentare
fix::remove_attachments()      // Anhänge, die nur beschweren
fix::flatten_layers()          // verhindert versehentlich zugeschaltete Ebenen
fix::remove_unused_resources()
```

```bash
pdfl fix verlag.pdf profile/vorbereiten.pdfl --output druck.pdf --dry-run  # prüfen
pdfl fix verlag.pdf profile/vorbereiten.pdfl --output druck.pdf            # anwenden
pdfl run profile/offset.pdfl druck.pdf                                     # validieren
```

---

## 12.7 Ein Profil im Team verteilen

**Das Problem:** Fünf Arbeitsplätze sollen genau dasselbe Profil und dieselben
Daten benutzen, ohne dass jemand daran dreht.

```bash
# Auf dem Rechner, der das Profil pflegt
pdfl pack profile/ --name druckprofil --version 1.2.0

# Auf den Produktionsrechnern
pdfl add druckprofil.pdflpkg
# installiert nach ./pdfl_profiles/druckprofil@1.2.0/, jede Prüfsumme geprüft

pdfl run pdfl_profiles/druckprofil@1.2.0/offset.pdfl datei.pdf
```

Wurde das Paket unterwegs verändert, **verweigert** `add` die Installation.

---

## 12.8 Einer problematischen Datei auf den Grund gehen

Vorgehen, wenn unklar ist, woher das Problem kommt:

```bash
# 1. Überblick in Sekunden
pdfl inspect verdaechtig.pdf

# 2. Untersuchungsskript, nur print()
cat > untersuchung.pdfl <<'EOF'
check "X-ray" {
  print("exact TAC:", prepress::calculate_exact_tac(), "%")
  print("estimated TAC:", prepress::calculate_tac(), "%")
  print("spots:", prepress::detect_spot_colors().join(", "))
  print("rich black?", prepress::detect_rich_black())
  print("overprint ok?", prepress::validate_overprint_settings())
  print("loose fonts:", prepress::detect_text_substitution().join(", "))

  doc.images.each { |img|
    print("image page", img.page_number, ":", img.width, "x", img.height,
          "@", round(img.dpi), "DPI", img.color_space)
  }
}
EOF

pdfl run untersuchung.pdfl verdaechtig.pdf > /dev/null
# print() schreibt auf die Fehlerausgabe: Man wirft den Bericht weg
# und behält nur die Ergebnisse der Untersuchung
```

---

[← Kommandozeile](11-cli.md) · [Inhalt](README.md)
