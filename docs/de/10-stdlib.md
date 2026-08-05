# 10. Standardbibliothek

[← `data::`](09-data.md) · [Inhalt](README.md) · [Weiter: Kommandozeile →](11-cli.md)

Die Methoden von Listen und Zeichenketten sowie die globalen Funktionen, die
überall im Skript zur Verfügung stehen.

---

## 10.1 Listenmethoden

| Methode | Zweck |
|---|---|
| `liste.each { \|item\| ... }` | Führt den Block für jedes Element aus |
| `liste.each_with_index { \|item, i\| ... }` | Gibt zusätzlich die Position (ab **0**) |
| `liste.all { \|item\| ... }` | Wahr, wenn alle die Bedingung erfüllen (wahr bei leerer Liste) |
| `liste.any { \|item\| ... }` | Wahr, wenn mindestens eines sie erfüllt (falsch bei leerer Liste) |
| `liste.filter { \|item\| ... }` | Behält nur die, die sie erfüllen |
| `liste.map { \|item\| ... }` | Neue, umgewandelte Liste |
| `liste.length` | Anzahl der Elemente (`length()` geht auch) |
| `liste.contains(wert)` | Ist der Wert in der Liste? |
| `liste.get(n)` | N-tes Element (ab **1**) |
| `liste.first()` / `liste.last()` | Erstes / letztes (`null` bei leerer Liste) |
| `liste.join([trenner])` | Verbindet zu einer Zeichenkette (Vorgabe `", "`) |

```pdfl
check "List methods" {
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  doc.fonts.each_with_index { |font, i|
    print("font", i + 1, "of", doc.fonts.length, ":", font.name)
  }

  require doc.fonts.all { |f| f.is_embedded }
  assert doc.pages.any { |p| p.extract_text() != "" },
    "the entire document has no text"

  schlecht = doc.images.filter { |img| img.dpi < 300 }
  assert schlecht.length == 0, "#{schlecht.length} image(s) with low resolution"

  print("fonts:", doc.fonts.map { |f| f.name }.join(", "))

  // get beginnt bei 1: get(1) ist das erste Element
  zeile = data::load_dataset("daten/chargen.csv").get(2)
  print("first column:", zeile.get(1))

  // Auch bei leerer Liste sicher: null ist falsch
  sonder = prepress::detect_spot_colors()
  assert !sonder.first() || sonder.first() == "Varnish",
    "unexpected special ink: #{sonder.first()}"
}
```

---

## 10.2 Zeichenkettenmethoden

| Methode | Zweck |
|---|---|
| `text.contains(teil)` | Enthält er dieses Stück? |
| `text.starts_with(teil)` | Beginnt er damit? |
| `text.ends_with(teil)` | Endet er damit? |
| `text.trim()` | Entfernt Leerzeichen am Anfang und Ende |
| `text.to_uppercase()` | Alles groß |
| `text.to_lowercase()` | Alles klein |
| `text.length` | Anzahl der Zeichen |

```pdfl
check "String methods" {
  titel = doc.title
  require titel.length > 0
  require titel.trim() == titel          // keine überflüssigen Leerzeichen
  assert !titel.to_lowercase().contains("draft"),
    "title still marked as draft"

  code = codes::decode_barcode(1)
  assert code.starts_with("789"), "GTIN is not Brazilian"
  assert doc.filename.ends_with(".pdf"), "unexpected extension"
}

check "contains on each type" {
  // Zeichenkette: sucht ein STÜCK im Text
  require "final document".contains("final")

  // Liste: sucht ein vollständiges ELEMENT
  require ["a", "b"].contains("a")
  require !["ab"].contains("a")      // "a" ist kein Element dieser Liste
}
```

---

## 10.3 Globale Funktionen

| Funktion | Zweck |
|---|---|
| `min(a, b)` / `max(a, b)` | Der kleinere / größere Wert |
| `abs(x)` | Betrag |
| `round(x)` | Rundet auf die nächste ganze Zahl |
| `print(...)` | Gibt aus, durch Leerzeichen getrennt, auf der **Fehlerausgabe** |
| `region(x, y, b, h [, name])` | Erzeugt eine Region ([Kapitel 2](02-types.md)) |

`print` schreibt auf die Fehlerausgabe: `> bericht.json` enthält also nur den
Bericht.

```pdfl
check "Global functions" {
  const A4_BREITE = 595.0
  const TOLERANZ = 5.0

  // abs ist der Schlüssel für Maßvergleiche mit Toleranz
  doc.pages.each { |page|
    assert abs(page.width - A4_BREITE) < TOLERANZ,
      "page #{page.number} is outside A4: #{page.width}pt"
  }

  // round macht Meldungen lesbar
  // Ohne: "217.4453125 DPI". Mit: "217 DPI".
  doc.images.each { |img|
    assert img.dpi >= 300,
      "image on page #{img.page_number}: #{round(img.dpi)} DPI"
  }

  print("document:", doc.filename)
  print("pages:", doc.page_count, "| fonts:", doc.fonts.length)
}
```

---

## 10.4 Gängige Wendungen

```pdfl
// Zählen, wie viele Elemente durchfallen
check "Problem count" {
  schlecht = doc.images.filter { |i| i.dpi < 300 }
  assert schlecht.length == 0,
    "#{schlecht.length} of #{doc.images.length} images below 300 DPI"
}

// Die betroffenen Elemente in der Meldung auflisten
check "List in the message" {
  // Verkettung in derselben Zeile: kein Umbruch vor dem Punkt
  probleme = doc.pages.filter { |p| !p.has_trim_box }.map { |p| p.number }
  assert probleme.length == 0,
    "pages without a TrimBox: #{probleme.join(", ")}"
}

// Prüfung mit Toleranz
function nahe_bei(wert, ziel, toleranz) {
  abs(wert - ziel) < toleranz
}

check "With tolerance" {
  doc.pages.each { |page|
    assert nahe_bei(page.width, 595.0, 2.0),
      "page #{page.number}: width #{page.width}pt (expected 595 ± 2)"
  }
}

// Bei einem leeren Dokument nicht abstürzen
check "Defensive" {
  // Der Kurzschluss verhindert den first()-Aufruf auf einer leeren Liste
  assert doc.pages.length == 0 || doc.pages.first().width > 0,
    "the first page has no width"
}
```

---

[← `data::`](09-data.md) · [Inhalt](README.md) · [Weiter: Kommandozeile →](11-cli.md)
