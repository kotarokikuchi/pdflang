# 9. Namensraum `data::` — externe Daten

[← `fix::`](08-fix.md) · [Inhalt](README.md) · [Weiter: Standardbibliothek →](10-stdlib.md)

8 Funktionen, um den Inhalt des PDF gegen eigene Listen und Tabellen zu prüfen.
Alles läuft lokal, es gehen keine Daten hinaus.

---

## 9.1 Wo die Dateien liegen

Glossare und Datensätze nehmen einen Pfad **relativ zum Arbeitsverzeichnis**:

```pdfl
data::load_glossary("begriffe/recht.txt")
data::load_dataset("daten/chargen.csv")
```

Die Nachschlagetabellen (`query_gtin`, `query_medicamento`,
`query_postal_code`) verwenden feste Dateinamen und suchen in dieser Reihenfolge:

1. `$PDFL_DATA_DIR` (Umgebungsvariable)
2. `./dados/`
3. `./`
4. Von `pdfl add` installierte Profile (`pdfl_profiles/*/dados/`)
5. Der Ordner des analysierten PDF

```bash
PDFL_DATA_DIR=/opt/datenbanken pdfl run profil.pdfl dokument.pdf
```

Wird nichts gefunden, sagt die Fehlermeldung, wohin die Datei gehört. Um die
Daten mit dem Profil zu verteilen, nehmen Sie `pdfl pack`
([Kapitel 11](11-cli.md)).

---

## 9.2 Glossare und Datensätze

| Funktion | Zweck |
|---|---|
| `data::load_glossary(datei)` | Liste von Begriffen (einer je Zeile, `#` = Kommentar) |
| `data::validate_against_reference(datei)` | Liste der im Dokument **fehlenden** Begriffe |
| `data::load_dataset(datei)` | Liest eine CSV als Liste von Zeilen |
| `data::lookup_value(datei, schlüssel)` | 2. Spalte der Zeile, deren 1. dem Schlüssel entspricht (sonst `null`) |

Der Vergleich ignoriert Groß-/Kleinschreibung und Leerzeichen.

`begriffe/pflicht.txt`:

```
# Begriffe, die jede Versicherungspolice enthalten muss
waiting period
covered benefits
general conditions
```

```pdfl
check "Glossary and dataset" {
  begriffe = data::load_glossary("begriffe/pflicht.txt")
  print("terms in the glossary:", begriffe.length)

  // Die unmittelbarste Verwendung
  fehlend = data::validate_against_reference("begriffe/pflicht.txt")
  assert fehlend.length == 0,
    "clauses missing from the policy: #{fehlend.join("; ")}"

  zeilen = data::load_dataset("daten/chargen.csv")
  print("columns:", zeilen.first().join(" | "))   // die 1. Zeile ist die Kopfzeile
  print("records:", zeilen.length - 1)

  // null ist falsch, deshalb geht die Prüfung direkt
  charge = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  beschreibung = data::lookup_value("daten/chargen.csv", charge)
  assert beschreibung, "batch #{charge} is not in the approved list"
}
```

### Datensätze als JSON

Eine Datei mit der Endung `.json` wird als JSON gelesen — von `load_dataset` wie
von `lookup_value`. Zwei Formen werden akzeptiert, denn in diesen beiden wird
ein Datensatz tatsächlich geschrieben.

Ein Array von Arrays sind die Zeilen, wie sie sind:

```json
[["charge", "beschreibung"],
 ["L2026-08", "Freigegebene Charge August/2026"]]
```

Ein Array von Objekten wird zu einer Kopfzeile plus einer Zeile je Objekt. Die
Spalten stehen in der Reihenfolge, die das **erste** Objekt schreibt, nicht
alphabetisch — der erste Schlüssel bleibt also der, den `lookup_value` sucht:

```json
[{"charge": "L2026-08", "beschreibung": "Freigegebene Charge August/2026"},
 {"charge": "L2026-09", "beschreibung": "Freigegebene Charge September/2026"}]
```

Ein Schlüssel, der in einem späteren Objekt fehlt, hinterlässt eine **leere
Zelle**, nie eine verschobene Zeile: ein Loch sieht man im Bericht, eine
Verschiebung nicht. Zahlen behalten die Ziffern, mit denen sie geschrieben
wurden, und `null` ist eine leere Zelle — dasselbe, was ein leeres CSV-Feld
bedeutet.

Beide Formen in einer Datei zu mischen ist ein Fehler, der die Zeile nennt.

---

## 9.3 Nachschlagetabellen

Dateien mit festen Namen, gesucht in der Reihenfolge aus Abschnitt 9.1. Sie
geben die **ganze Zeile** als Liste zurück, oder `null`, wenn nichts gefunden
wird.

| Funktion | Referenzdatei | Zweck |
|---|---|---|
| `data::query_gtin(code)` | `gtin.csv` | Suche nach GTIN (Satzzeichen egal) |
| `data::query_medicamento(zulassung_oder_name)` | `medicamentos.csv` | Nach Zulassungsnummer oder Namensteil |
| `data::query_postal_code(code)` | `ceps.csv` | Nach Postleitzahl (8 Ziffern) |
| `data::validate_address(code, "fragment")` | `ceps.csv` | Enthält die Adresse zu dieser PLZ das Fragment? |

`dados/gtin.csv`:

```csv
gtin,description,manufacturer
7891234567895,Dipyrone 500mg 20 tablets,Example Labs
```

```pdfl
check "Lookup tables" {
  // Abgleich mit dem auf der Verpackung gelesenen Strichcode
  code = codes::decode_barcode(1)
  produkt = data::query_gtin(code)
  assert produkt, "GTIN #{code} is not in the product database"
  print("product:", produkt.get(2), "| manufacturer:", produkt.get(3))

  // Arzneimittelangaben über die Zulassungsnummer
  zulassung = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
  medikament = data::query_medicamento(zulassung)
  assert medikament, "registration #{zulassung} not found"

  // Ein verschreibungspflichtiges Mittel verlangt den Pflichthinweis
  band = medikament.get(4)
  assert band != "prescription" || text::require_text("PRESCRIPTION ONLY"),
    "prescription medicine without the mandatory text"

  // Passt die gedruckte Adresse zur Postleitzahl?
  assert data::validate_address("01310100", "Avenida Paulista"),
    "printed address does not match the given postal code"
}
```

---

## 9.4 Vollständiges Beispiel

```pdfl
// beipackzettel_mit_daten.pdfl — Abgleich mit lokalen Daten
// Aufruf: PDFL_DATA_DIR=./datenbanken pdfl run beipackzettel_mit_daten.pdfl beipackzettel.pdf
profile "insert-with-references" {

  check "Mandatory regulatory terms" tags: ["glossary"] {
    fehlend = data::validate_against_reference("datenbanken/pflichttexte.txt")
    assert fehlend.length == 0, "mandatory texts missing: #{fehlend.join("; ")}"
  }

  check "Product in the database" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    produkt = data::query_gtin(code)
    assert produkt, "GTIN #{code} not approved"

    // Der eingetragene Name muss auf dem Druck erscheinen
    name = produkt.get(2)
    assert text::require_text(name),
      "the name '#{name}' from the database does not appear on the insert"
    print("product verified:", name)
  }

  check "Registration and band" tags: ["regulatory"] {
    zulassung = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
    med = data::query_medicamento(zulassung)
    assert med, "registration #{zulassung} not found"
    assert med.get(4) != "prescription" || text::require_text("PRESCRIPTION ONLY"),
      "prescription band requires the prescription notice"
  }

  check "Manufacturer address" tags: ["data"] {
    assert data::validate_address("01310100", "Avenida Paulista"),
      "manufacturer address does not match the postal code"
  }
}
```

---

[← `fix::`](08-fix.md) · [Inhalt](README.md) · [Weiter: Standardbibliothek →](10-stdlib.md)
