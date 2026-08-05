# 7. Namensraum `codes::` — Strichcodes und QR-Codes

[← `prepress::`](06-prepress.md) · [Inhalt](README.md) · [Weiter: `fix::` →](08-fix.md)

13 Funktionen, um Strichcodes und QR-Codes eines Dokuments zu erkennen, zu
decodieren und zu prüfen.

> Das Einlesen rendert die Seiten in hoher Auflösung und geschieht **nur
> einmal**, beim ersten Aufruf einer `codes::`-Funktion. Ein Skript, das diesen
> Namensraum nicht nutzt, zahlt diesen Preis nicht.

Unterstützte Formate: EAN-8/13, UPC-A/E, Code 128, Code 39, ITF, QR-Code, Data
Matrix, Aztec und PDF417.

---

## 7.1 Erkennung

| Funktion | Zweck |
|---|---|
| `codes::detect_barcodes()` | Wahr, wenn ein Strichcode vorhanden ist |
| `codes::detect_qrcodes()` | Wahr, wenn ein QR-Code vorhanden ist |
| `codes::count_barcodes()` | Gesamtzahl der gelesenen Codes |
| `codes::get_barcode_type(n)` | Format des n-ten Codes (`"EAN_13"`, `"QR_CODE"` …) |
| `codes::get_barcode_location(n)` | Position `[Seite, x, y]` in Punkt, Ursprung unten links |

```pdfl
check "Codes present" {
  assert codes::detect_barcodes(), "no barcode found in the artwork"
  assert codes::detect_qrcodes(), "the traceability QR code is missing"

  gesamt = codes::count_barcodes()
  assert gesamt == 2, "expected 2 codes (EAN + QR), found #{gesamt}"

  typ = codes::get_barcode_type(1)
  assert typ == "EAN_13", "the main code should be EAN-13, it is #{typ}"

  stelle = codes::get_barcode_location(1)
  assert stelle.first() == 1, "barcode is not on the cover"
}
```

---

## 7.2 Decodierung und Prüfung

| Funktion | Zweck |
|---|---|
| `codes::decode_barcode(n)` | Inhalt des n-ten Codes |
| `codes::validate_barcode_checksum(n)` | GTIN-Prüfziffer des n-ten Codes |
| `codes::validate_gtin(text)` / `codes::validate_ean(text)` | GTIN-Prüfziffer einer Zeichenkette |
| `codes::validate_code128()` | Wahr, wenn ein Code 128 erfolgreich decodiert wurde |

```pdfl
check "Code integrity" {
  code = codes::decode_barcode(1)
  print("code read:", code)

  // Ein GTIN mit falscher Prüfziffer wird an der Kasse abgewiesen
  assert codes::validate_barcode_checksum(1),
    "invalid check digit in code #{code}"
  assert code.starts_with("789"),
    "GTIN is not Brazilian (should start with 789): #{code}"

  // Abgleich mit der unter dem Code gedruckten Nummer
  gedruckt = text::extract_from_region(1, region(400, 20, 180, 15)).trim()
  assert codes::validate_ean(gedruckt),
    "the printed number is not a valid GTIN: #{gedruckt}"
}
```

---

## 7.3 Abgleiche

| Funktion | Zweck |
|---|---|
| `codes::compare_barcode_with_text()` | Wahr, wenn der Inhalt jedes Codes im Text vorkommt |
| `codes::validate_barcode_format(regex)` | Wahr, wenn alle Inhalte zum Ausdruck passen |
| `codes::validate_barcode_position(region)` oder `(x0, y0, x1, y1)` | Wahr, wenn alle Codes im Bereich liegen |

`compare_barcode_with_text` fängt den teuersten Fehler der Branche ab: Der Code
zeigt auf ein Produkt, der gedruckte Text nennt ein anderes.

```pdfl
check "Cross-checks" {
  assert codes::compare_barcode_with_text(),
    "the barcode number does not appear in the text — artwork with swapped data?"

  // Nur EAN-13 erlaubt: genau 13 Ziffern
  assert codes::validate_barcode_format("^\d{13}$"),
    "there is a code outside the EAN-13 pattern"

  // Eine benannte Region liest sich besser
  bereich = region(400, 20, 180, 80, "barcode area")
  assert codes::validate_barcode_position(bereich),
    "barcode outside the reserved area of the packaging"
}
```

---

## 7.4 Vollständiges Beispiel

```pdfl
// beipackzettel.pdfl — Codeprüfung eines Arzneimittel-Beipackzettels
// Aufruf: pdfl run beipackzettel.pdfl beipackzettel.pdf
profile "medicine-insert" {

  check "Codes present" tags: ["codes"] {
    assert codes::detect_barcodes(), "insert has no barcode"
    assert codes::count_barcodes() >= 1, "expected at least the product EAN"
  }

  check "Code integrity" tags: ["codes"] {
    code = codes::decode_barcode(1)
    typ = codes::get_barcode_type(1)
    print("code:", typ, "=", code)

    assert typ == "EAN_13", "main code is not EAN-13 (it is #{typ})"
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"
    assert code.starts_with("789"), "GTIN is not Brazilian: #{code}"
  }

  check "Cross-check with the text" tags: ["codes", "critical"] {
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Position in the artwork" tags: ["codes", "layout"] {
    reserviert = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(reserviert),
      "code outside the reserved area — risk of being trimmed off"
  }

  check "Cross-check with the product database" tags: ["data"] {
    // Zusammen mit data:: — siehe Kapitel 9
    code = codes::decode_barcode(1)
    produkt = data::query_gtin(code)
    assert produkt, "GTIN #{code} is not in the approved product database"
    print("product:", produkt.get(2))
  }
}
```

---

[← `prepress::`](06-prepress.md) · [Inhalt](README.md) · [Weiter: `fix::` →](08-fix.md)
