# 11. Kommandozeile

[← Standardbibliothek](10-stdlib.md) · [Inhalt](README.md) · [Weiter: Rezepte →](12-recipes.md)

Dreizehn Befehle: sechs für PDFs, vier für Skripte, zwei für die Verteilung und
einer für die Shell.

| Befehl | Zweck |
|---|---|
| [`run`](#pdfl-run) | Prüft ein PDF mit einem Skript |
| [`compare`](#pdfl-compare) | Vergleicht zwei Fassungen |
| [`pixelcompare`](#pdfl-pixelcompare) | Vergleicht zwei PDFs Pixel für Pixel, mit einem Betrachter für die Änderung |
| [`watch`](#pdfl-watch) | Überwacht einen Ordner und prüft, was eintrifft |
| [`fix`](#pdfl-fix) | Wendet Änderungen an und speichert ein neues PDF |
| [`inspect`](#pdfl-inspect) | Schneller Überblick über ein PDF |
| [`lint`](#pdfl-lint) | Analysiert ein Skript, ohne es auszuführen |
| [`fmt`](#pdfl-fmt) | Formatiert ein Skript |
| [`test`](#pdfl-test) | Führt ein Skript über einen Ordner PDFs aus und vergleicht jeden Bericht |
| [`doc`](#pdfl-doc) | Erzeugt die Dokumentation eines Skripts |
| [`pack`](#pdfl-pack) | Packt Profile und Daten |
| [`add`](#pdfl-add) | Installiert ein Paket |
| [`completions`](#pdfl-completions) | Gibt ein Vervollständigungsskript für Ihre Shell aus |

---

## Exit-Codes

Gelten für alle Befehle, die prüfen.

| Code | Bedeutung |
|---|---|
| `0` | Alles bestanden |
| `1` | Nur Warnungen |
| `2` | Prüffehler |
| `3` | Syntaxfehler im Skript |
| `10` | Das Dokument war nicht lesbar oder eine Datei nicht schreibbar — es gab kein Urteil |

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

## Globale Optionen

| Option | Zweck |
|---|---|
| `--quiet` | Unterdrückt Fortschritt und Bestätigungen auf stderr |

`--quiet` wirkt vor wie nach dem Unterbefehl, und bei jedem von ihnen. Es nimmt
die Zeilen weg, die ein Mensch will und eine Pipeline nicht — `report saved to
…`, `watching …`, die Zeile je Datei von `watch`. Fehler nimmt es **nicht** weg:
ein stiller Lauf, der scheitert, sagt weiterhin warum.

Auch `print()` bleibt. Das ist die Ausgabe des Skripts selbst, und sie zu
schlucken würde ändern, was das Skript tut. Leiten Sie stderr um, wenn Sie sie
loswerden wollen.

`--quiet` gewinnt gegen `--verbose`.

---

## `pdfl run`

Prüft ein PDF mit einem Skript.

```bash
pdfl run <skript.pdfl> <eingabe.pdf> [optionen]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format des Berichts |
| `--output-file <datei>` | — | Schreibt in eine Datei statt auf die Standardausgabe |
| `--fail-on error\|warning` | `error` | Mit `warning` führt auch eine Warnung zu Code 2 |
| `--verbose` | — | Zusatzinformationen auf der Fehlerausgabe |
| `--var NAME=WERT` | — | Wert, den das Skript als `vars.NAME` liest; wiederholbar |
| `--tags TAG` | — | Führt nur Checks mit diesem Tag aus; wiederholbar. Ein Tag, den kein Check trägt, ist ein Fehler, kein leeres Bestehen |

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
  "schema_version": 1,
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
  ],
  "checks_run": ["Ink coverage", "Fonts", "Bleed"]
}
```

Dasselbe PDF mit demselben Skript ergibt stets einen **Byte für Byte
identischen Bericht**: Man kann ihn versionieren und Unterschiede in der CI
vergleichen.

`schema_version` steht als erster Schlüssel, damit ein Konsument sich
entscheiden kann, bevor er den Rest parst. Sie steigt nur, wenn ein Leser der
vorherigen Ausgabe brechen würde; ein zusätzliches Feld lässt sie unverändert.

### SARIF und JUnit

Zwei weitere Formate, damit das Ergebnis dort auftaucht, wo das Team ohnehin
hinsieht, statt in einem Log, das niemand öffnet.

```bash
# GitHub code scanning: die Befunde werden zu Anmerkungen am Pull Request
pdfl run vorstufe.pdfl magazin.pdf --output sarif --output-file pdfl.sarif

# Test-Panel jeder CI: ein Test je Check, die bestandenen eingeschlossen
pdfl run vorstufe.pdfl magazin.pdf --output junit --output-file pdfl.xml
```

In SARIF hängt ein Befund am **Skript**, nicht am PDF: die Zeile, die wir
kennen, ist die des Checks, und das PDF ist meist ein Artefakt auf dem Weg durch
die CI und keine Datei im Repository — dorthin zu zeigen würde einen Pfad
annotieren, den es nicht gibt. Die geprüfte Datei reist in
`properties.inputFile`, die Diagnose-Kennung in `partialFingerprints` — und
genau daran erkennt GitHub einen bereits gesehenen Befund, statt ihn bei jedem
Lauf neu zu öffnen.

In JUnit ist jeder gelaufene Check ein Testfall, auch die, die nichts gefunden
haben. Ein Format, das nur die Fehlschläge auflistet, meldete einen sauberen
Lauf als null Tests, und eine CI liest das als einen Lauf, der nie stattfand.
Ein `info`-Befund lässt seinen Fall nicht durchfallen; er landet in
`<system-out>`.

```yaml
- name: Vorstufe
  run: pdfl run vorstufe.pdfl magazin.pdf --output sarif --output-file pdfl.sarif
  # Exit 2 heißt abgelehnte Datei, und der Upload muss trotzdem laufen
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: pdfl.sarif
```

---

## `pdfl compare`

Vergleicht zwei Fassungen: Text, Struktur und Metadaten.

```bash
pdfl compare <v1.pdf> <v2.pdf> [optionen]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format |
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

## `pdfl pixelcompare`

Vergleicht zwei PDFs danach, wie sie *aussehen*, Seite für Seite.

```bash
pdfl pixelcompare <original.pdf> <neu.pdf> [optionen]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format des Berichts |
| `--output-file <datei>` | — | Schreibt den Bericht in eine Datei |
| `--viewer <ordner>` | — | Schreibt einen eigenständigen Betrachter: die Seiten, die Unterschiede und eine `index.html` dazu |
| `--dpi <n>` | `150` | Auflösung beim Rendern. Höher sieht mehr und kostet mehr |
| `--threshold <0.0-1.0>` | `0.05` | Farbabstand, ab dem zwei Pixel als verschieden gelten |
| `--max-diff <prozent>` | `0.0` | Wie viel einer Seite sich ändern darf, bevor es gemeldet wird |
| `--pages <bereich>` | alle | `1-10` oder `1,3,7-12` |
| `--no-align` | — | Gleicht eine globale Verschiebung nicht aus |
| `--blur <radius>` | `0` | Weichzeichnen vor dem Vergleich, gegen Kantenglättung |
| `--jobs <n>` | eine pro CPU | Gleichzeitig verglichene Seiten |

`pdfl compare` beantwortet „hat sich Text oder Struktur geändert". Dies
beantwortet eine andere Frage — „sieht es noch genauso aus" — und die beiden
widersprechen sich öfter als man denkt. Ein um 2mm verschobenes Logo, eine
verschwundene Haarlinie, eine Sonderfarbe durch ihren CMYK-Aufbau ersetzt: in
allen drei Fällen ist der Text identisch.

```bash
# Das ganze Dokument, als JSON
pdfl pixelcompare freigegeben.pdf nachdruck.pdf

# Mit einem Ort, an dem man die Unterschiede wirklich ansehen kann
pdfl pixelcompare freigegeben.pdf nachdruck.pdf --viewer diff/

# Etwas Rauschen dulden und den Rest genauer ansehen
pdfl pixelcompare freigegeben.pdf nachdruck.pdf --max-diff 0.1 --dpi 300
```

Ein Befund je geänderter Seite, mit dem Anteil der Pixel und der Zahl der
getrennten Bereiche:

```
page 7: 0.51% of the pixels differ, in 29 area(s)
```

Eine Seite, die es in einer Datei gibt und in der anderen nicht, ist ein
eigener Befund — es gibt nichts, womit man sie vergleichen könnte. Das
`similarity` des Berichts ist der Mittelwert über die verglichenen Seiten, eine
neu gebaute Seite unter zweihundert macht also kein anderes Dokument daraus;
die Zahlen je Seite stehen in den Diagnosen.

### Ausrichtung, und warum sie an ist

Eine Datei, die aus derselben Quelle neu exportiert wurde, liegt oft ein bis
zwei Pixel daneben. Ohne Ausgleich ist jede Glyphenkante der Seite
„unterschiedlich" und die eine Änderung, auf die es ankommt, geht darin unter.
`pixelcompare` sucht eine einzige globale Verschiebung — erst grob auf einer
verkleinerten Kopie, dann verfeinert — und meldet sie, wenn es eine findet:

```
page 3: 2.10% of the pixels differ, in 44 area(s) (aligned by 2, -1 px)
```

Mit `--no-align` abschalten, wenn gerade die Position das Geprüfte *ist*.

### Der Betrachter

`--viewer diff/` schreibt einen Ordner mit drei PNGs je Seite und einer
`index.html`. Ohne jede Abhängigkeit — kein CDN, kein Bundler, kein Server.
Datei öffnen, oder den Ordner packen und an die Person schicken, die den
Nachdruck freigibt.

Drei Bereiche nebeneinander, immer auf derselben Seite:

| Bereich | Was er zeigt |
|---|---|
| **Original** | die Seite der ersten Datei, unangetastet |
| **New** | die Seite der zweiten Datei, unangetastet |
| **Difference** | beide, mit dem Geänderten darübergelegt — zum Wischen ziehen |

Alle drei Bereiche tragen dasselbe Leistenpaar — eine stehende, eine liegende —
an derselben Stelle, und beide bewegen sich in allen dreien zugleich. Die
stehende Leiste wird gezogen, die liegende folgt dem Zeiger, gedrückt oder
nicht. Ihr Kreuzungspunkt ist die Ecke des Freigelegten, und der Punkt sitzt auf
der stehenden Leiste in dieser Höhe — er markiert also die Stelle, die der
Zeiger hält.

Im Bereich **Difference** schneiden die Leisten: die neue Datei erscheint rechts
der stehenden und unterhalb der liegenden, überall sonst das Original.
Unberührt liegt die flache Leiste oben, womit die stehende ein gewöhnlicher
Wischer über die volle Höhe ist — die flache zieht man herunter, wenn die
gesuchte Änderung in einem Band statt in einer Spalte liegt. In den beiden
anderen Bereichen sind die Leisten Lineale auf derselben Spalte und derselben
Zeile der Seite.

Beide Positionen sind Prozentsätze der Seite, nicht eines Bereichs, und
überstehen daher einen Seitenwechsel und das Ändern der Fenstergröße.

Das Mausrad zoomt, bis 8×, und alle drei Bereiche zoomen gemeinsam um den Punkt
unter dem Zeiger — was man betrachtet hat, bleibt also, wo es war.
Herauszoomen endet bei der eingepassten Seite: darunter gibt es nichts
Nützliches, der Bereich fasst die ganze Seite bereits. Die Leisten behalten bei
jedem Zoom ihre Stärke. **Reset view** setzt den Zoom auf die ganze Seite und
die Leisten auf ihren Ausgangsplatz zurück; solange es nichts zurückzunehmen
gibt, ist die Schaltfläche deaktiviert.

Die Unterschiede werden an Ort und Stelle eingefärbt, und die Farbe sagt, welcher
Art sie sind:

| Farbe | Bedeutung |
|---|---|
| Rot | Farbe, die in der neuen Datei fehlt |
| Grün | Farbe, die dort neu ist |
| Blau | Gleiche Stärke, andere Farbe |

Die drei Bereiche werden am Fenster bemessen, der ganze Vergleich passt also
ohne Scrollen auf den Schirm, und sie behalten bei jedem Fensterformat die
Proportionen der Seite. Wo die beiden Dateien sich über die Größe einer Seite
uneinig sind — eine wurde quer gestellt —, wird jede ganz im gemeinsamen Rahmen
gezeigt statt zum Füllen verzerrt.

**Er öffnet auf den Seiten, die sich unterscheiden.** Bei zweihundert Seiten,
von denen drei sich geändert haben, sind genau diese drei der Grund, warum man
ihn geöffnet hat; **All** holt den Rest zurück. Die Pfeile und `←` `→` folgen
dem Filter und überspringen, was die Leiste ausblendet. Unterscheidet sich
nichts, sagt der Filterknopf das und bleibt deaktiviert, statt die Leiste auf
nichts zusammenzustreichen.

### Fortschritt

Ein langes Dokument bei 300 dpi zu rastern dauert Minuten, deshalb zeichnet jede
Stufe einen Balken auf stderr: einen je gerasterter Datei, einen für den
Vergleich und einen für das Schreiben des Betrachters.

```
rasterising freigegeben.pdf  [############------------]  98/207
```

Gezeichnet wird nur, wenn stderr ein Terminal ist. Der Balken arbeitet, indem er
an den Zeilenanfang zurückgeht und überschreibt; eine Logdatei hat keinen
Cursor, ein umgeleiteter Lauf sammelte also Tausende Fragmente. Umgeleitet
bleibt er still, die gewöhnlichen Meldungen kommen weiterhin durch. `--quiet`
schaltet ihn überall ab.

### Geschwindigkeit

Der Vergleich läuft standardmäßig auf allen CPUs. Bei 41 Seiten mit 150 dpi:

| `--jobs` | Zeit |
|---|---|
| `1` | 3,6s |
| `4` | 1,7s |
| `8` | 1,2s |
| `20` | 1,3s |

Ab etwa acht bringt es nichts mehr, denn diese Stufe wird von der
Speicherbandbreite begrenzt, nicht von der Rechenarbeit — sie schiebt ganze
Seiten durch die CPU. Danach warten die Threads nur noch auf denselben
Speicher. Mehr zu verlangen schadet nicht, nützt aber nichts.

Beachten Sie, was **nicht** parallel läuft: das Rastern. pdfium serialisiert
jeden Aufruf hinter einem einzigen globalen Lock, ein zweiter Thread davor
wartet also nur. Das setzt dem Lauf einen Boden — etwa 0,8s der obigen Zahlen —
und deshalb ist `--jobs 8` dreimal schneller und nicht achtmal.

Hier ist der Standard eine pro CPU, während `pdfl test` und `pdfl watch`
`--jobs 1` verwenden. Der Unterschied ist echt: dort ist ein Job ein
Kindprozess mit einem eigenen Dokument, also jedes Mal ein weiteres Dokument im
Speicher. Hier liegen die Seiten bereits im Speicher und die Threads teilen sie
sich, ein Job kostet also den Arbeitsbereich einer Seite. Verringern Sie den
Wert, wenn Sie sich die Maschine teilen.

Exit-Codes: `0` keine Seite änderte sich um mehr als `--max-diff`, `2`
mindestens eine, `10` eine Datei war nicht lesbar oder der Betrachter nicht
schreibbar.

Der Bericht hängt nicht von `--jobs` ab. Die Seiten werden in Seitenreihenfolge
zusammengeführt, deshalb kommen die Diagnosen, ihre Reihenfolge und ihre
Fingerabdrücke bei jedem Wert identisch heraus — ein Test sichert das ab, und
die Dateien des Betrachters entstehen Byte für Byte gleich.

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
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format der Berichte |
| `--fail-fast` | — | Hält beim ersten Fehler an |
| `--events` | — | Wacht auf Systembenachrichtigungen statt auf einen Zeitgeber — nicht auf Netzfreigaben |
| `--journal <datei>` | — | Nur angehängtes Protokoll des Geprüften; ein erneuter Lauf überspringt, was darin steht |
| `--timeout <s>` | — | Bricht die Prüfung einer Datei nach so vielen Sekunden ab und meldet sie als abgelehnt |
| `--var NAME=WERT` | — | Wert, den jede Datei als `vars.NAME` liest; wiederholbar |
| `--jobs <n>` | `1` | Gleichzeitig geprüfte Dateien; `0` heißt eine je CPU |
| `--once` | — | Verarbeitet den Bestand und beendet sich |

```bash
# Eingangsordner einer Druckerei, im Dauerbetrieb
pdfl watch inbox/ --script preflight.pdfl --output-dir berichte/ --report html

# Stapellauf für die CI: beendet sich mit dem schlechtesten Code
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

`--jobs` gilt für alles, was ein Durchlauf zu bewältigen hat — im Stapel wie bei
einem Schwung Neuzugänge. Jede Datei wird von ihrem eigenen `pdfl`-Prozess
geprüft (aus demselben Grund wie bei `pdfl test`), und dieser Prozess rendert die
Berichte, die geschriebene Datei ist also unabhängig von `--jobs` identisch. Bei
acht Dateien à 41 Seiten: 9,5s mit `--jobs 1`, 1,2s mit `--jobs 0`.

Mit `--fail-fast` wird keine neue Datei mehr begonnen, sobald eine gescheitert
ist; die laufenden werden fertig, denn sie abzubrechen hinterließe halb
geschriebene Berichte. Die Berichte entstehen in der Fundreihenfolge, ein Stapel
druckt also dieselben Zeilen, wie viele auch gleichzeitig liefen.

Die Wartezeit endet genau dann, wenn die frischeste Datei fertig ist, eine
während des Wartens ankommende Datei wird also kein ganzes Intervall länger
gehalten.

Der Ordner wird standardmäßig auf einem Zeitgeber gelistet; mit `--events`
wartet watch stattdessen auf die Benachrichtigungen des Betriebssystems. Die
Vorgabe ist der Zeitgeber, und das wurde gemessen: 10.000 Dateien alle 200ms zu
listen kostet keine messbare CPU, und die Setzzeit dominiert die Latenz ohnehin
— auf einem lokalen Ordner liegen beide Modi Hundertstelsekunden auseinander.

Verwenden Sie `--events` nicht auf einer Netzfreigabe. inotify meldet auf einem
NFS- oder SMB-Mount nur, was die lokale Maschine schreibt; von anderswo
eintreffende Dateien würden nie bemerkt — und watch sagte nichts dazu. Es lohnt
sich auf einer Maschine, die viele Ordner beobachtet, oder wo ein
Verzeichnislisting teuer ist. Lässt sich der Watcher nicht anlegen, sagt watch
das und fällt auf den Zeitgeber zurück, statt zu verstummen.

Das **debounce** gibt es, weil große Dateien in Stücken ankommen: Verarbeitet
wird nur eine Datei, die sich nicht mehr ändert — also nie ein halb
geschriebenes PDF.

### Das Journal: einen unterbrochenen Stapel zu Ende bringen

Fünftausend Dateien, und bei viertausend startet die Maschine neu. Ohne
Aufzeichnung beginnt der nächste Lauf bei der ersten.

```bash
pdfl watch eingang/ --script offset.pdfl --once --journal stapel.jsonl
```

Ein JSON-Objekt je Datei, angehängt sobald sie geprüft ist:

```json
{"input":"eingang/umschlag.pdf","sha256":"9f2b…","status":"FAIL","errors":2,"warnings":0,"exit":2}
```

Beim nächsten Lauf mit demselben Journal werden die darin verzeichneten Dateien
übersprungen. Ihre Urteile nicht: ein fortgesetzter Stapel, der eine abgelehnte
Datei überspringt, endet weiterhin mit `2` — das Journal ist die Aufzeichnung des
Stapels, der Exit-Code sein Urteil. Ein Stapel, der sauber meldet, weil er den
Fehlschlag schon gesehen hat, wäre der schlimmste Fehler, den dieses Werkzeug
haben könnte.

Erkannt wird eine Datei **an ihren Bytes**, nicht am Namen und nicht am
Zeitstempel. Ersetzen Sie `umschlag.pdf` durch ein anderes `umschlag.pdf`, wird
sie erneut geprüft — ihr Hash ist nicht der aufgezeichnete.

Ohne `--journal` wird nichts geschrieben. Das Werkzeug hält keinen eigenen
Zustand; dies ist eine Datei, die Sie beim Namen verlangt haben, genau wie ein
Bericht. Und in einer Zeile steht keine Zeit: das Journal sagt, *ob* eine Datei
geprüft wurde und was dabei herauskam, der Bericht daneben sagt *was*, und das
Dateisystem sagt *wann* — so bleibt ein erneuter Lauf Byte für Byte wie der
erste, wie alles hier.

Die Zeilen werden einzeln geschrieben, was ein Absturz hinterlässt, stimmt also
so weit es reicht. Ein Journal, das sich nicht lesen lässt, hält den Lauf an und
nennt die Zeile; Dateien aufgrund eines falsch gelesenen Eintrags zu überspringen
wäre schlimmer, als von vorn zu beginnen.

### `--timeout`: eine schlechte Datei darf den Stapel nicht aufhalten

```bash
pdfl watch eingang/ --script offset.pdfl --once --timeout 60
```

Eine Datei, deren Prüfung länger als `60` Sekunden dauert, wird beendet und
genauso gemeldet wie ein unlesbares PDF — ein Bericht mit einem Befund,
`check_name: "timeout"` — er wird also gedruckt, auf die Platte geschrieben und
geht ins Journal ein wie jedes andere Urteil. Nichts wird still übersprungen,
und der Stapel geht zur nächsten Datei über, statt an dieser hängenzubleiben.

```json
{"input":"eingang/adversarial.pdf","sha256":"7a1c…","status":"FAIL","errors":1,"warnings":0,"exit":2}
```

In der Sprache `.pdfl` gibt es nichts, womit ein Skript den Interpreter absichtlich
aufhängen könnte — Rekursion ist tiefenbegrenzt — `--timeout` existiert also für
das, was ein Skript nicht verursachen kann: pdfium, das bei einem fehlerhaften
oder feindseligen PDF in eine Schleife gerät oder blockiert. Ohne das Flag wartet
die Prüfung einer Datei so lange wie nötig, das einzige Verhalten, bevor es dieses
Flag gab.

`--var` erreicht jede Datei unverändert — ein Wert für den ganzen Lauf, sinnvoll
für etwas in einem Ordner Konstantes (ein Kundenname), nicht für etwas je Datei
Unterschiedliches (eine Auftragsnummer). Ohne das Flag könnte ein Skript, das
`vars.*` liest, nie beobachtet werden: jede Datei scheiterte mit „was not
provided".

Die Berichte entstehen als `<name>.report.json` (oder `.csv`, `.html`, `.pdf`).

---

## `pdfl fix`

Wendet die `fix::`-Operationen an und speichert ein neues PDF. Einzelheiten in
[Kapitel 8](08-fix.md).

```bash
pdfl fix <eingabe.pdf> <skript.pdfl> --output <ausgabe.pdf> [optionen]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--output <Datei>` | — | Ausgabe-PDF (erforderlich) |
| `--dry-run` | — | Listet die Operationen, ohne zu speichern |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format des Berichts |
| `--report-file <Datei>` | — | Schreibt den Bericht in eine Datei |

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

`--json` liefert dieselbe Zusammenfassung als Daten.

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

`--json` liefert dieselben Warnungen als Daten.

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
pdfl doc <skript.pdfl> [--output markdown|html|json]
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

Es sammelt rekursiv die `.pdfl`-, `.csv`-, `.txt`- und `.json`-Dateien des
Ordners und legt ein `manifest.json` bei, das den SHA-256 jeder Datei notiert.
Das Packen ist deterministisch: Derselbe Ordner ergibt dieselben Bytes.

Eine Tabellendatei (`.xlsx`, `.xls`, `.ods`) wird **nicht** verpackt, und `pack`
sagt, welche Datei draußen blieb. Keine `data::`-Funktion kann eine öffnen; sie
mitzugeben hieße, ein Paket auszuliefern, das sauber installiert und beim ersten
Nachschlagen scheitert.

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

## `pdfl test`

Führt ein Skript über jedes PDF eines Ordners aus und vergleicht jeden Bericht
mit dem, der daneben aufgezeichnet ist. Ein Profil, das plötzlich etwas anderes
findet, lässt einen Test scheitern, statt jemanden weiter hinten zu überraschen.

```bash
pdfl test <skript.pdfl> [--dir <ordner>] [--update]
```

| Option | Vorgabe | Zweck |
|---|---|---|
| `--dir <ordner>` | `tests/` neben dem Skript | Wo die Fall-PDFs liegen |
| `--update` | — | Zeichnet die erwarteten Berichte auf, statt zu vergleichen |
| `--jobs <n>` | `1` | Gleichzeitig laufende Fälle; `0` heißt einer je CPU |
| `--var NAME=WERT` | — | Wert, den jeder Fall als `vars.NAME` liest; wiederholbar |

Ein Fall ist ein PDF und der von ihm erwartete Bericht, nebeneinander:

```
profile/druckerei/
  vorstufe.pdfl
  tests/
    freigegeben.pdf
    freigegeben.expected.json
    viel_farbe.pdf
    viel_farbe.expected.json
```

```bash
# Beim ersten Mal: aufzeichnen, was das Skript heute findet
pdfl test vorstufe.pdfl --update

# Von da an
pdfl test vorstufe.pdfl
```

```
ok   freigegeben.pdf
FAIL viel_farbe.pdf
     error_count: expected 1, got 0
     missing:    PDFL-093751a2 [error] Farbauftrag (line 12): Seite 7: 324% Farbe (Grenze 300%)
1 passed, 1 failed
```

Der Fehlschlag benennt, was sich geändert hat — die Zähler, das Urteil und
welche Befunde auftauchten oder verschwanden —, statt zwei JSON-Dateien
nebeneinander zu drucken.

Aufzeichnen ist immer eine bewusste Handlung: ein Lauf, der seine eigene
Baseline auffrischt, könnte nie scheitern. Lesen Sie erst den Unterschied und
zeichnen Sie mit `--update` neu auf, wenn die Änderung die gemeinte ist.

Der erwartete Bericht ist der von `pdfl run`, mit `input_file` auf den Dateinamen
verkürzt — eine Baseline, die sich je nach Aufrufverzeichnis ändert, ist keine.
Ein PDF, das sich nicht öffnen lässt, lässt seinen eigenen Fall scheitern und die
übrigen laufen.

Exit-Codes: `0` alle bestanden, `2` mindestens einer scheiterte, `10` der Ordner
war nicht lesbar oder enthält kein PDF.

### Fälle gleichzeitig laufen lassen

Jeder Fall läuft als eigener `pdfl`-Prozess, `--jobs` macht aus einer Suite also
echte Parallelarbeit: bei acht Dateien à 41 Seiten brauchte `--jobs 1` 8,9s und
`--jobs 8` 1,1s. Threads innerhalb eines Prozesses hätten es nicht geschafft —
pdfium serialisiert jeden Aufruf hinter einem einzigen Mutex, und die Variante
mit Threads maß sich *langsamer* als die sequentielle.

Die Vorgabe ist `1`, denn jeder Job ist ein Prozess, der ein Dokument im
Speicher hält, und dieses Werkzeug existiert für Dateien, die sehr groß sein
können. Erhöhen Sie den Wert bei gewöhnlichen Fällen: `--jobs 0` gibt einen je
CPU.

Die Reihenfolge der Ausgabe ändert sich mit `--jobs` nie: die Fälle werden in
der Fundreihenfolge beurteilt, gleich welches Kind zuerst fertig war.

Ein Fall, dessen PDF sich nicht lesen lässt, wird wie jeder andere beurteilt —
sein Bericht führt den Grund als Befund, „diese Datei muss als unlesbar
abgelehnt werden" kann also selbst ein Test sein. Dieser Bericht nennt die Datei
so, wie sie übergeben wurde: zeichnen Sie Baselines mit einem **relativen**
`--dir` auf, wenn sie eingecheckt werden sollen.

`--var` erreicht jeden Fall unverändert — ein Wert für den ganzen Lauf, nicht
einer je Datei. Ohne das Flag könnte ein Skript, das `vars.*` liest, nie
getestet werden: jeder Fall scheiterte mit „was not provided", gleich welches
PDF.

---

## `pdfl completions`

Gibt ein Vervollständigungsskript für Ihre Shell auf stdout aus.

```bash
pdfl completions <bash|zsh|fish|elvish|powershell>
```

```bash
# bash, für den aktuellen Benutzer
pdfl completions bash > ~/.local/share/bash-completion/completions/pdfl

# zsh — irgendwo in Ihrem $fpath
pdfl completions zsh > ~/.zfunc/_pdfl

# fish
pdfl completions fish > ~/.config/fish/completions/pdfl.fish
```

Sonst geht nichts nach stdout, die Ausgabe lässt sich also direkt in das
Vervollständigungsverzeichnis umleiten. Nach einem Upgrade neu erzeugen: das
Skript entsteht aus den Befehlen und Flags der Binärdatei, die es gedruckt hat.

---

[← Standardbibliothek](10-stdlib.md) · [Inhalt](README.md) · [Weiter: Rezepte →](12-recipes.md)
