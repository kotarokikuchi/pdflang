# 3. Namensraum `text::` — der Text

[← Typen](02-types.md) · [Inhalt](README.md) · [Weiter: `struct::` →](04-struct.md)

25 Funktionen, um den Text eines Dokuments zu extrahieren, zu normalisieren, zu
durchsuchen und zu prüfen.

> Bei Funktionen, die mit `[text]` gekennzeichnet sind, ist das Argument
> **optional**: ohne es arbeitet die Funktion auf dem ganzen Dokument, mit ihm
> auf der übergebenen Zeichenkette.

---

## 3.1 Extraktion

| Funktion | Zweck |
|---|---|
| `text::extract_all()` | Der gesamte Text des Dokuments (Seiten durch Umbrüche verbunden) |
| `text::extract_from_page(page)` | Der Text einer Seite (ab 1) |
| `text::extract_from_region(page, region)` | Der Text eines Bereichs (leere Zeichenkette, wenn keiner da ist) |
| `text::extract_with_normalization()` | Der bereits normalisierte Text des Dokuments |

```pdfl
check "Extraction" {
  inhalt = text::extract_all()
  assert inhalt.trim() != "", "PDF has no extractable text"

  titelseite = text::extract_from_page(1)
  assert titelseite.contains("User Manual"), "cover lacks the expected title"

  // Produktionsfußzeilen (InDesign-Dateiname, Exportdatum) überleben
  // manchmal bis in die fertige Datei
  fuss = region(0, 0, 467, 40, "footer")
  doc.pages.each { |page|
    zeile = text::extract_from_region(page.number, fuss)
    assert !zeile.contains(".indd"),
      "page #{page.number} has a production mark in the footer: #{zeile.trim()}"
  }
}
```

---

## 3.2 Normalisierung und Zerlegung

| Funktion | Zweck |
|---|---|
| `text::normalize([text])` | Kleinschreibung + zusammengefasste Leerzeichen |
| `text::split_words([text])` | Zerlegt in Wörter (Satzzeichen an den Rändern entfernt) |
| `text::split_sentences([text])` | Zerlegt in Sätze |
| `text::split_paragraphs([text])` | Zerlegt in Absätze (Leerzeile) |
| `text::count_words([text])` | Anzahl der Wörter |
| `text::count_characters([text])` | Anzahl der Zeichen |
| `text::detect_language([text])` | `"pt"`, `"en"`, `"es"` oder `"unknown"` |

```pdfl
check "Normalization and splitting" {
  require text::normalize("  HELLO   World  ") == "hello world"

  woerter = text::split_words("Hello, world! (test)")
  require woerter.length == 3
  require woerter.first() == "Hello"

  // Beipackzettel und Verträge haben eine praktische Lesbarkeitsgrenze
  text::split_sentences().each { |satz|
    assert satz.length < 400,
      "sentence with #{satz.length} characters — hard to read"
  }

  require text::count_words() > 100
  assert text::detect_language() == "en",
    "document should be in English, detected: #{text::detect_language()}"
}
```

---

## 3.3 Suche und Pflichtinhalte

| Funktion | Zweck |
|---|---|
| `text::require_text(begriff)` | Wahr, wenn der Begriff vorkommt |
| `text::forbid_text(begriff)` | Wahr, wenn er nicht vorkommt |
| `text::require_match(regex)` | Wahr, wenn der reguläre Ausdruck etwas findet |
| `text::forbid_match(regex)` | Wahr, wenn er nichts findet |
| `text::fuzzy_match(a, b)` | Ähnlichkeit zweier Zeichenketten (0.0 bis 1.0) |

Der Vergleich ignoriert Groß-/Kleinschreibung und Leerzeichen.

```pdfl
profile "contract" {
  check "Mandatory clauses" {
    assert text::require_text("governing law"),
      "contract has no governing-law clause"
    assert text::require_match("\d{4}/\d{4}"), "contract number not found"
  }

  check "Forbidden terms" {
    assert text::forbid_text("DRAFT"), "document still marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text was not replaced"
    assert text::forbid_match("\d{2}-\d{2}-\d{4}"), "US-format date found"
  }

  check "Name with tolerance" {
    // Nützlich, wenn Tippfehler oder OCR-Rauschen zu erwarten sind
    gefunden = text::extract_from_region(1, region(50, 700, 300, 40))
    aehnlichkeit = text::fuzzy_match("Paracetamol 750mg", gefunden)
    assert aehnlichkeit > 0.9,
      "product name differs from expected (#{round(aehnlichkeit * 100)}% similar)"
  }
}
```

---

## 3.4 Personenbezogene Daten

`text::detect_personal_data()` und `text::detect_pii()` sind gleichbedeutend.
Sie geben die **Liste** der gefundenen personenbezogenen Daten zurück: CPF, CNPJ
(brasilianische Steuernummern), E-Mail-Adresse und Telefonnummer.

> CPF und CNPJ kommen nur in die Liste, wenn die **Prüfziffer stimmt**. Eine
> Nummer, die nur wie ein CPF aussieht (`111.111.111-12`), löst keinen Alarm aus.

```pdfl
check "Public document must carry no personal data" {
  gefunden = text::detect_personal_data()
  assert gefunden.length == 0, "personal data exposed: #{gefunden.join("; ")}"

  // Jeder Eintrag sieht aus wie "CPF: 529.982.247-25"
  text::detect_pii().each { |eintrag| print("found:", eintrag) }
}
```

---

## 3.5 Formatprüfungen

| Funktion | Zweck |
|---|---|
| `text::validate_cpf(text)` | Prüfziffer des CPF (mod 11) |
| `text::validate_cnpj(text)` | Prüfziffer des CNPJ |
| `text::validate_date_format(text [, format])` | Tatsächlich gültiges Kalenderdatum |
| `text::validate_phone_format(text)` | Brasilianisches Telefonformat |
| `text::validate_format(text, regex)` | Passt die **ganze** Zeichenkette? |

Akzeptierte Datumsformate: `"dd/mm/aaaa"` und `"aaaa-mm-dd"`; ohne zweites
Argument werden beide akzeptiert.

```pdfl
check "Format validation" {
  require text::validate_cpf("529.982.247-25")
  require !text::validate_cpf("111.111.111-11")    // lauter gleiche Ziffern
  require text::validate_cnpj("11.222.333/0001-81")

  require text::validate_date_format("29/02/2024")   // 2024 ist ein Schaltjahr
  require !text::validate_date_format("29/02/2023")  // 2023 nicht
  require !text::validate_date_format("31/04/2026")  // April hat 30 Tage

  require text::validate_phone_format("(11) 98765-4321")

  // Chargencode im Werksformat
  charge = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  assert text::validate_format(charge, "L\d{4}-\d{2}"),
    "batch code does not follow the L0000-00 pattern: #{charge}"
}
```

---

## 3.6 Vergleich und Diagnose

`text::diff(a, b)` listet die geänderten Zeilen (`-` entfernt, `+` ergänzt).
`text::detect_rasterized_text()` ist wahr, wenn Text in ein Bild verwandelt
wurde.

```pdfl
check "Comparison and diagnostics" {
  aenderungen = text::diff(text::extract_from_page(1), text::extract_from_page(2))
  print("changed lines:", aenderungen.length)

  // Eine gescannte oder in Pfade umgewandelte Seite ist weder durchsuchbar
  // noch für Screenreader lesbar
  assert !text::detect_rasterized_text(),
    "there are pages with rasterized text (scanned or outlined)"
}
```

> Um zwei **Dateien** zu vergleichen, nehmen Sie den Befehl `pdfl compare`: Er
> ordnet die Seiten selbstständig zu. Siehe [Kapitel 11](11-cli.md).

---

## 3.7 Vollständiges Beispiel

```pdfl
// rechtsdokument.pdfl — Prüfung eines Vertrags
profile "standard-contract" {

  check "Required content" tags: ["legal"] {
    assert text::require_text("governing law"), "no governing-law clause"
    assert text::require_text("term of agreement"), "no term clause"
    assert text::require_match("\d{4}/\d{4}"), "no contract number"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("XXX+"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    gefunden = text::detect_personal_data()
    assert gefunden.length == 0,
      "personal data in a public document: #{gefunden.join("; ")}"
  }

  check "Text quality" tags: ["text"] {
    assert text::detect_language() == "en", "document is not in English"
    assert !text::detect_rasterized_text(), "rasterized text blocks search"
    require text::count_words() > 200
  }
}
```

---

[← Typen](02-types.md) · [Inhalt](README.md) · [Weiter: `struct::` →](04-struct.md)
