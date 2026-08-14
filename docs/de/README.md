# PDFLang-Dokumentation — Deutsch

Vollständige Anleitung zur Sprache `.pdfl` und zum Kommandozeilenwerkzeug
`pdfl` — Version 0.18.0.

Jedes Beispiel in dieser Dokumentation ist lauffähiger, kommentierter Code. Wenn
Sie die Sprache zum ersten Mal verwenden, beginnen Sie mit dem Handbuch in
Kapitel 1; die übrigen Kapitel sind zum Nachschlagen gedacht.

> **Zur Sprache des Werkzeugs.** Die Meldungen von `pdfl` (Diagnosen, Fehler,
> Hilfe auf der Kommandozeile, Beschriftungen in Berichten) sind **englisch**,
> wie bei Kommandozeilenwerkzeugen üblich. Diese Dokumentation ist deutsch, aber
> eine fehlgeschlagene Prüfung zeigt etwas wie `page 7: 324% ink (limit 300%)`.
> Die Meldungen, die Sie **selbst** in Ihren Skripten schreiben, erscheinen
> unverändert in der Sprache, die Sie verwendet haben.

## Inhalt

| Kapitel | Inhalt |
|---|---|
| [1. Die Sprache](01-language.md) | Vollständiges Handbuch: checks, Zusicherungen, Typen, Einheiten, Blöcke, Funktionen, import, rule |
| [2. Typen des Dokuments](02-types.md) | `doc`, `page`, `font`, `image`, `region` — alle Eigenschaften und Methoden |
| [3. `text::`](03-text.md) | Text: Extraktion, Normalisierung, Suche, brasilianische Prüfungen, personenbezogene Daten |
| [4. `struct::`](04-struct.md) | Struktur und Metadaten: Objekte, XMP, Sicherheit, Prüfsummen |
| [5. `visual::`](05-visual.md) | Bilder: Auflösung, visueller Vergleich, pHash, SSIM, Qualität |
| [6. `prepress::`](06-prepress.md) | Druckvorstufe: Farbauftrag, Separationen, Sonderfarben, Schriften, Rahmen |
| [7. `codes::`](07-codes.md) | Strichcodes und QR-Codes: Erkennung, Decodierung, Prüfung |
| [8. `fix::`](08-fix.md) | Normalisierung: Rahmen, Seiten, Wasserzeichen, Zusammenführen/Teilen, Optimierung |
| [9. `data::`](09-data.md) | Externe Daten: Glossare, Datensätze, Nachschlagetabellen |
| [10. Standardbibliothek](10-stdlib.md) | Listen- und Zeichenkettenmethoden, globale Funktionen |
| [11. Kommandozeile](11-cli.md) | `run`, `compare`, `pixelcompare`, `watch`, `fix`, `inspect`, `lint`, `fmt`, `doc`, `pack`, `add`, `test`, `completions` |
| [12. Rezepte](12-recipes.md) | Vollständige Fälle: Druckerei, Rechtsverlag, Pharmalabor, CI/CD |
| [13. Änderungen](13-changelog.md) | Was sich in jeder Version geändert hat und was brechen kann |

## In 30 Sekunden loslegen

Legen Sie `mein_profil.pdfl` an:

```pdfl
// Jedes Skript ist eine Menge von checks. Ein check bündelt zusammengehörende
// Prüfungen und wird zu einem Abschnitt im Bericht.
check "Basic structure" {
  // require: Die Meldung entsteht automatisch aus dem Ausdruck
  require doc.page_count > 0

  // assert: mit der Meldung, die Sie selbst schreiben
  assert doc.title != "", "PDF has no title in its metadata"
}
```

Ausführen:

```bash
pdfl run mein_profil.pdfl dokument.pdf
```

Der Bericht erscheint als JSON auf der Standardausgabe. Der Exit-Code nennt das
Ergebnis: `0` alles bestanden, `1` nur Warnungen, `2` Prüffehler, `3`
Syntaxfehler.

## Konventionen dieser Dokumentation

- Zu jeder Funktion stehen **Signatur**, was sie **tut**, was sie **zurückgibt**
  und ein **kommentiertes Beispiel**.
- Argumente in eckigen Klammern sind optional: `calculate_tac([page])`.
- „ab 1“ heißt: die erste Seite ist `1`, nicht `0` — die Sprache zählt so, wie
  Menschen zählen, nicht so, wie Programmierer zählen.
- Maße sind immer in **Punkt** (1 pt = 1/72 Zoll). Einheiten-Literale (`3mm`,
  `1in`) rechnen für Sie um.

---

Andere Sprachen: [English](../en/) · [Português (Brasil)](../pt-br/) ·
[日本語](../ja/) · [中文](../zh/) · [Français](../fr/) · [العربية](../ar/)
