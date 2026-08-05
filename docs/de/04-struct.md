# 4. Namensraum `struct::` — Struktur und Metadaten

[← `text::`](03-text.md) · [Inhalt](README.md) · [Weiter: `visual::` →](05-visual.md)

23 Funktionen zur Datei selbst: Metadaten, interne Objekte, Sicherheit und
Nachvollziehbarkeit.

> Die Funktionen ab `list_objects` lesen die interne Struktur der Datei. Diese
> Analyse läuft **genau einmal**, beim ersten Gebrauch, und wird
> zwischengespeichert.

---

## 4.1 Metadaten

| Funktion | Gibt zurück |
|---|---|
| `struct::get_title()` | Den Titel |
| `struct::get_author()` | Den Autor |
| `struct::get_subject()` | Das Thema |
| `struct::get_keywords()` | Die Schlagwörter |
| `struct::get_creator()` | Das Programm, in dem das Dokument entstand |
| `struct::get_producer()` | Das Programm, das das PDF erzeugt hat |
| `struct::get_creation_date()` | Erstellungsdatum (`JJJJ-MM-TT HH:MM:SS`) |
| `struct::get_modification_date()` | Änderungsdatum (gleiches Format) |
| `struct::list_metadata_entries()` | Liste der nicht leeren Einträge (`"Schlüssel: Wert"`) |
| `struct::extract_xmp()` | Die XMP-Metadaten aus dem Katalog |

Alle geben eine leere Zeichenkette zurück, wenn das Feld fehlt.

```pdfl
check "Required metadata" {
  assert struct::get_title() != "", "PDF has no title"
  assert struct::get_author() != "", "PDF has no author"

  // Producer verrät das Ursprungswerkzeug — nützlich zur Fehlersuche
  print("produced by:", struct::get_producer())

  erstellt = struct::get_creation_date()
  assert erstellt != "", "PDF has no creation date"
  // Der Zeichenkettenvergleich funktioniert, weil das Format richtig sortiert
  assert erstellt > "2026-01-01", "file is too old for this campaign"

  xmp = struct::extract_xmp()
  assert xmp.contains("pdfaid"), "no PDF/A identification in the XMP"
}
```

---

## 4.2 Datei und Nachvollziehbarkeit

| Funktion | Zweck |
|---|---|
| `struct::file_size()` | Größe in Bytes |
| `struct::calculate_sha256()` | SHA-256-Prüfsumme der Datei |
| `struct::detect_file_bloat([kb_pro_seite])` | Wahr oberhalb der Grenze je Seite (Vorgabe 1024 KB) |

```pdfl
check "File size and traceability" {
  mb = struct::file_size() / 1024 / 1024
  assert mb < 10, "file is #{round(mb)} MB (10 MB e-mail limit)"

  // Die Prüfsumme belegt, welche Datei genau freigegeben wurde
  print("SHA-256:", struct::calculate_sha256())

  assert !struct::detect_file_bloat(1024),
    "heavy file: #{struct::file_size() / 1024} KB for #{doc.page_count} pages"
}
```

---

## 4.3 Interne Objekte

| Funktion | Zweck |
|---|---|
| `struct::count_objects()` | Anzahl der Inhaltsobjekte auf den Seiten |
| `struct::list_objects()` | Alle Objekte (`"Nummer: Typ"`) |
| `struct::detect_unreferenced_objects()` | Vom Trailer aus unerreichbare Objekte |
| `struct::detect_orphaned_resources()` | Unerreichbare Ressourcen (Schriften, Bilder) |
| `struct::measure_object_size(nummer)` | Ungefähre Größe eines Objekts in Bytes |

> Infrastrukturobjekte (`ObjStm`, `XRef`) sind ausgenommen: Sie werden per
> Definition nie vom Trailer referenziert, sie zu melden wäre ein Fehlalarm.

```pdfl
check "File hygiene" {
  require struct::count_objects() > 0

  lose = struct::detect_unreferenced_objects()
  assert lose.length == 0,
    "#{lose.length} unreferenced object(s): #{lose.join(", ")}"

  verwaist = struct::detect_orphaned_resources()
  assert verwaist.length == 0,
    "unused embedded resources: #{verwaist.join(", ")} — run 'pdfl fix' with remove_unused_resources()"
}
```

---

## 4.4 Sicherheit

| Funktion | Zweck |
|---|---|
| `struct::detect_javascript()` | Wahr, wenn eingebettetes JavaScript vorhanden ist |
| `struct::detect_suspicious_actions()` | Liste riskanter Aktionen |
| `struct::check_encryption()` | Wahr, wenn das Dokument verschlüsselt ist |
| `struct::validate_permissions()` | Wahr, wenn es keine Einschränkungen gibt |
| `struct::validate_signatures()` | Wahr, wenn Signaturfelder vorhanden sind |

`detect_suspicious_actions` findet `JavaScript`, `Launch` (startet ein
Programm), `URI`, `SubmitForm`, `ImportData` und `GoToR`.

> `validate_signatures` prüft das **Vorhandensein** dieser Felder. Die
> kryptografische Prüfung der Zertifikatskette leistet diese Version nicht.

```pdfl
check "Security" {
  // JavaScript in einem PDF ist ein verbreiteter Angriffsweg
  // und in einem Druckdokument überflüssig
  assert !struct::detect_javascript(), "PDF contains embedded JavaScript"

  aktionen = struct::detect_suspicious_actions()
  assert aktionen.length == 0,
    "suspicious actions in the PDF: #{aktionen.join("; ")}"

  // Ein verschlüsseltes PDF kann am RIP der Druckerei scheitern
  assert !struct::check_encryption(),
    "PDF is encrypted — remove protection before sending it to print"
  assert struct::validate_permissions(),
    "PDF has permission restrictions that may block processing"
}
```

---

## 4.5 Vollständiges Beispiel

```pdfl
// audit.pdfl — Konformitäts- und Sicherheitsprüfung
profile "file-audit" {

  check "Identification" tags: ["metadata"] {
    assert struct::get_title() != "", "no title"
    assert struct::get_author() != "", "no author"
    assert struct::get_creation_date() != "", "no creation date"
    print("produced by:", struct::get_producer())
  }

  check "Traceability" tags: ["audit"] {
    print("SHA-256:", struct::calculate_sha256())
    print("size:", struct::file_size() / 1024, "KB")
  }

  check "Security" tags: ["security"] {
    assert !struct::detect_javascript(), "embedded JavaScript"
    assert !struct::check_encryption(), "encrypted file"
    aktionen = struct::detect_suspicious_actions()
    assert aktionen.length == 0, "suspicious actions: #{aktionen.join("; ")}"
  }

  check "File hygiene" tags: ["optimization"] {
    verwaist = struct::detect_orphaned_resources()
    assert verwaist.length == 0, "unused resources: #{verwaist.join(", ")}"
    assert !struct::detect_file_bloat(1024), "bloated file"
  }
}
```

---

[← `text::`](03-text.md) · [Inhalt](README.md) · [Weiter: `visual::` →](05-visual.md)
