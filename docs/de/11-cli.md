# 11. Kommandozeile

[← Standardbibliothek](10-stdlib.md) · [Inhalt](README.md) · [Weiter: Rezepte →](12-recipes.md)

Zehn Befehle: vier für PDFs, vier für Skripte und zwei für die Verteilung.

| Befehl | Zweck |
|---|---|
| [`run`](#pdfl-run) | Prüft ein PDF mit einem Skript |
| [`compare`](#pdfl-compare) | Vergleicht zwei Fassungen |
| [`watch`](#pdfl-watch) | Überwacht einen Ordner und prüft, was eintrifft |
| [`fix`](#pdfl-fix) | Wendet Änderungen an und speichert ein neues PDF |
| [`inspect`](#pdfl-inspect) | Schneller Überblick über ein PDF |
| [`lint`](#pdfl-lint) | Analysiert ein Skript, ohne es auszuführen |
| [`fmt`](#pdfl-fmt) | Formatiert ein Skript |
| [`doc`](#pdfl-doc) | Erzeugt die Dokumentation eines Skripts |
| [`pack`](#pdfl-pack) | Packt Profile und Daten |
| [`add`](#pdfl-add) | Installiert ein Paket |

---

## Exit-Codes

Gelten für alle Befehle, die prüfen.

| Code | Bedeutung |
|---|---|
| `0` | Alles bestanden |
| `1` | Nur Warnungen |
| `2` | Prüffehler oder PDF nicht lesbar |
| `3` | Syntaxfehler im Skript |

```bash
pdfl run profil.pdfl datei.pdf > bericht.json
case $? in
  0) echo "approved" ;;
  1) echo "approved with warnings" ;;
  2) echo "rejected — see bericht.json" ;;
  3) echo "error in the validation script" ;;
esac
```

---

## `pdfl run`

Prüft ein PDF mit einem Skript.

```bash
pdfl run <skript.pdfl> <eingabe.pdf> [optionen]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | Format des Berichts |
| `--output-file <datei>` | — | Schreibt in eine Datei statt auf die Standardausgabe |
| `--fail-on error\|warning` | `error` | Mit `warning` führt auch eine Warnung zu Code 2 |
| `--verbose` | — | Zusatzinformationen auf der Fehlerausgabe |

```bash
pdfl run vorstufe.pdfl magazin.pdf                                     # JSON im Terminal
pdfl run vorstufe.pdfl magazin.pdf --output html --output-file bericht.html
pdfl run vorstufe.pdfl magazin.pdf --output pdf --output-file bericht.pdf
pdfl run vorstufe.pdfl magazin.pdf --output csv --output-file befunde.csv
pdfl run vorstufe.pdfl magazin.pdf --fail-on warning                   # strenger Modus
```

### Der JSON-Bericht

```json
{
  "script_name": "prepress.pdfl",
  "input_file": "magazine.pdf",
  "profile": "offset-magazine",
  "status": "FAIL",
  "total_pages_analyzed": 120,
  "error_count": 2,
  "warning_count": 0,
  "info_count": 0,
  "diagnostics": [
    {
      "id": "PDFL-093751a2",
      "severity": "error",
      "check_name": "Ink coverage",
      "message": "page 7: 324% ink (limit 300%)",
      "line": 12
    }
  ]
}
```

Dasselbe PDF mit demselben Skript ergibt stets einen **Byte für Byte
identischen Bericht**: Man kann ihn versionieren und Unterschiede in der CI
vergleichen.

---

## `pdfl compare`

Vergleicht zwei Fassungen: Text, Struktur und Metadaten.

```bash
pdfl compare <v1.pdf> <v2.pdf> [optionen]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | Format |
| `--output-file <datei>` | — | Schreibt in eine Datei |
| `--normalize` | — | Ignoriert Groß-/Kleinschreibung und Leerzeichen |
| `--ignore-dates` | — | Maskiert Datumsangaben vor dem Vergleich |
| `--similarity-threshold <0-100>` | `100` | Kleinste hinnehmbare Ähnlichkeit |

```bash
pdfl compare freigegeben_v1.pdf erhalten_v2.pdf --normalize --ignore-dates

# Bis zu 1 % Abweichung ist erlaubt; darunter ist es ein Fehler
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file unterschiede.html
```

### Wie es arbeitet

- Seiten werden **nach Inhalt** einander zugeordnet, nicht nach Nummer: Eine in
  der Mitte eingefügte Seite lässt nicht alles Folgende als Unterschied
  erscheinen. Funktioniert auch bei mehr als tausend Seiten.
- Jedes Paar bekommt einen Ähnlichkeitswert und eine Auswahl der geänderten
  Zeilen (`-` entfernt, `+` ergänzt).
- Eine Änderung der Metadaten ist eine **Warnung**; eine Textänderung unter der
  Schwelle ist ein **Fehler**, darüber eine **Warnung**.
- Der Gesamtwert steht im Feld `similarity` des Berichts.

```
page 4 → 4: similarity 97.8% | -original title | +revised title
```

---

## `pdfl watch`

Überwacht einen Ordner und prüft jedes PDF, das eintrifft oder sich ändert.

```bash
pdfl watch <ordner> --script <skript.pdfl> [optionen]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | Welche Dateien verarbeitet werden |
| `--exclude <glob>` | — | Welche übergangen werden |
| `--output-dir <ordner>` | neben dem PDF | Wohin die Berichte gehen |
| `--depth <n>` | `1` | Tiefe der Unterordner |
| `--debounce <ms>` | `1000` | Wartezeit, bis die Datei stabil ist |
| `--report json\|csv\|html\|pdf` | `json` | Format der Berichte |
| `--fail-fast` | — | Hält beim ersten Fehler an |
| `--once` | — | Verarbeitet den Bestand und beendet sich |

```bash
# Eingangsordner einer Druckerei, im Dauerbetrieb
pdfl watch inbox/ --script preflight.pdfl --output-dir berichte/ --report html

# Stapellauf für die CI: beendet sich mit dem schlechtesten Code
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

Das **debounce** gibt es, weil große Dateien in Stücken ankommen: Verarbeitet
wird nur eine Datei, die sich nicht mehr ändert — also nie ein halb
geschriebenes PDF.

Die Berichte entstehen als `<name>.report.json` (oder `.csv`, `.html`, `.pdf`).

---

## `pdfl fix`

Wendet die `fix::`-Operationen an und speichert ein neues PDF. Einzelheiten in
[Kapitel 8](08-fix.md).

```bash
pdfl fix original.pdf normalisieren.pdfl --output out.pdf --dry-run  # nur ansehen
pdfl fix original.pdf normalisieren.pdfl --output korrigiert.pdf     # anwenden
```

---

## `pdfl inspect`

Überblick über ein PDF, ohne Skript.

```bash
pdfl inspect <datei.pdf>
```

```
File:     magazine.pdf
Size:     26 KB (27284713 bytes)
SHA-256:  af1029842e5bfeae338ead82fb449ef851be742b1d63117c12596e3ea123a616

Pages:    120
Page size: 496 x 709 pt
Boxes:    MediaBox, TrimBox, BleedBox

Metadata:
  Title: Example Magazine
  Creator: Adobe InDesign 19.3

Fonts:    26
  ABCDEF+Helvetica — embedded
  Arial — NOT embedded
Images:   81 (minimum DPI 136, spaces: DeviceCMYK, Indexed)
Max. estimated TAC: 300% (RGB render approximation)

Warnings:
  ! there are non-embedded fonts
  ! 3 image(s) below 300 DPI
```

Der erste Befehl, wenn eine Datei eintrifft: In Sekunden weiß man, ob sie das
Öffnen lohnt.

---

## `pdfl lint`

Analysiert ein Skript, ohne es auszuführen, und meldet Qualitätsprobleme.

```bash
pdfl lint <skript.pdfl>
```

Was es findet:

- Variablen, Blockparameter und Funktionen, die deklariert und **nie benutzt**
  werden (mit `_` davor lässt sich die Warnung unterdrücken: `_page`)
- **Doppelte** oder **leere** checks
- Unbekannte Namensräume (`text::`, `struct::`, `visual::`, `prepress::`,
  `codes::`, `fix::`, `data::`)
- `assert` / `require` außerhalb eines checks
- Gebrauch von `fix::` (läuft nur unter `pdfl fix`)

```bash
$ pdfl lint profil.pdfl
profil.pdfl: warning: variable 'LIMIT' declared and never used
profil.pdfl: warning: check "Fonts" declared 2 times
```

Bei Warnungen ist der Exit-Code `1` — in der CI verwendbar.

---

## `pdfl fmt`

Formatiert ein Skript: zwei Leerzeichen Einrückung, einheitliche Abstände,
zusammengefasste Leerzeilen. Kommentare und Einheiten (`3mm` bleibt `3mm`)
bleiben erhalten.

```bash
pdfl fmt <skript.pdfl>            # formatiert an Ort und Stelle
pdfl fmt <skript.pdfl> --check    # ändert nichts; Code 1, wenn unformatiert
```

```bash
# Teamstandard in der CI durchsetzen
for f in profile/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

Erzeugt die Dokumentation aus dem Skript selbst.

```bash
pdfl doc <skript.pdfl> [--output markdown|html]
```

Ausgegeben werden: das Profil, eine Tabelle der Konstanten, die Funktionen, die
Importe und zu jedem check seine Etiketten und das, was er prüft (die Meldungen
der `assert` werden zu den Beschreibungen).

```bash
pdfl doc vorstufe.pdfl > docs/vorstufen-profil.md
pdfl doc vorstufe.pdfl --output html > profil.html
```

Das ist das Ergebnis, das einer Produktionsleitung, die keinen Code liest,
erklärt, was ein Profil prüft.

---

## `pdfl pack`

Packt Skripte und Daten in ein verteilbares `.pdflpkg`.

```bash
pdfl pack <ordner> [--name <name>] [--version <version>] [--output <datei>]
```

Es sammelt rekursiv die `.pdfl`-, `.csv`-, `.txt`-, `.json`- und
`.xlsx`-Dateien des Ordners und legt ein `manifest.json` bei, das den SHA-256
jeder Datei notiert. Das Packen ist deterministisch: Derselbe Ordner ergibt
dieselben Bytes.

```bash
pdfl pack profile/druckerei --name druckprofil --version 1.0.0
```

---

## `pdfl add`

Installiert ein lokales Paket und prüft dabei die Prüfsummen des Manifests.

```bash
pdfl add druckprofil.pdflpkg
# installiert nach ./pdfl_profiles/druckprofil@1.0.0/

pdfl run pdfl_profiles/druckprofil@1.0.0/vorstufe.pdfl datei.pdf
```

Stimmt die Prüfsumme einer Datei nicht, wird die Installation **verweigert** —
ein beschädigtes oder verändertes Paket kommt nicht hinein.

> Ferne Verzeichnisse und digitale Signaturen gehören nicht zu dieser Version:
> `add` installiert aus einer lokalen Datei.

---

[← Standardbibliothek](10-stdlib.md) · [Inhalt](README.md) · [Weiter: Rezepte →](12-recipes.md)
