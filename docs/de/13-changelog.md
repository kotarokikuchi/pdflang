# 13. Änderungen

[← Rezepte](12-recipes.md) · [Inhalt](README.md)

Was sich in jeder Version geändert hat und was davon bei Ihnen brechen kann.

Die Version ist noch `0.x`, ein Minor-Sprung darf also etwas brechen. Wenn er es
tut, sagt der Eintrag genau was und wie man sich anpasst. Hier ändert sich
nichts stillschweigend.

---

## Noch nicht veröffentlicht

### Behoben

- `pdfl pack` schrieb eine verschachtelte Datei mit dem Trennzeichen der
  bauenden Maschine auf — unter Windows `daten\chargen.csv`. Ein Paket wird auf
  der einen Maschine gebaut und auf einer anderen installiert: alles außer `/`
  ergibt ein Paket, das sich installiert und dann seine eigenen Dateien nicht
  findet, auch unsere Prüfung nicht. Unter Linux und macOS gebaute Pakete waren
  nie betroffen.
- `--output` und `--output-file` wurden ignoriert, wenn die PDF nicht lesbar
  war. `run`, `fix` und `compare` gaben JSON auf stdout aus, egal was verlangt
  wurde: Eine Pipeline, die JUnit in einer Datei anforderte, bekam gar keine
  Datei und einen Bericht auf einem Strom, den sie nicht las — eine defekte
  Eingabe sah aus wie ein Lauf, den es nie gab, während der Exit-Code korrekt
  `10` sagte. Der Fehlerbericht geht jetzt denselben Weg wie jeder andere.
- Ein Fehler von pdfium erreichte den Bericht als über drei Zeilen
  ausgedrucktes Rust-Enum. Eine Diagnose ist ein Feld einer CSV-Zeile und ein
  XML-Attribut, deshalb wird sie jetzt auf eine Zeile gefaltet.

### Gut zu wissen

- Die Kennung eines `loading`-Befunds ändert sich, denn sie leitet sich aus der
  Meldung ab, und die Meldung trägt diese Zeilenumbrüche nicht mehr.

---

## 0.17.0

### Neu

- `pdfl pixelcompare` zeigt je Stufe einen Fortschrittsbalken — Rastern beider
  Dateien, Vergleichen, Schreiben des Betrachters. Gezeichnet wird nur, wenn
  stderr ein Terminal ist, denn der Balken überschreibt seine eigene Zeile und
  eine Logdatei hat keinen Cursor; umgeleitet bleibt er still. `--quiet`
  schaltet ihn überall ab.
- `--jobs <n>` bei `pdfl pixelcompare` vergleicht so viele Seiten gleichzeitig
  und steht standardmäßig auf einer pro CPU: 41 Seiten mit 150 dpi gehen von
  3,6s auf 1,2s. Nur der Vergleich läuft parallel — pdfium serialisiert jeden
  Aufruf hinter einem einzigen globalen Lock, das Rastern kann es also nicht,
  und deshalb ist der Gewinn dreifach statt achtfach. Der Bericht hängt nicht
  vom Wert ab: die Seiten werden in Seitenreihenfolge zusammengeführt, also
  sind die Diagnosen, ihre Reihenfolge und ihre Fingerabdrücke identisch — und
  die Dateien des Betrachters ebenso.

  > Hier ist der Standard eine pro CPU, während `test` und `watch` auf
  > `--jobs 1` stehen. Dort ist ein Job ein Kindprozess mit eigenem Dokument;
  > hier liegen die Seiten schon im Speicher und die Threads teilen sie sich.

---

## 0.16.0

### Neu

- `pdfl pixelcompare` vergleicht zwei PDFs danach, wie sie aussehen, statt nach
  ihrem Text, Seite für Seite, und meldet den Anteil abweichender Pixel. Eine
  nur verschobene Seite wird vor dem Vergleich ausgerichtet, damit ein Pixel
  Versatz nicht die Änderung begräbt, auf die es ankommt.
- `--viewer <ordner>` bei `pixelcompare` schreibt eine eigenständige Anwendung
  — kein CDN, kein Bundler, kein Server — zum Wischen, Umschalten oder
  Überblenden zwischen beiden Dateien, mit den Unterschieden an Ort und Stelle
  eingefärbt: Rot für fehlende Farbe, Grün für neue, Blau für gleiches Gewicht
  in anderer Farbe. Die Seitenleiste filtert auf **Changed only**, und die
  Pfeiltasten folgen dem Filter — bei einem langen Dokument ist das Blättern an
  den unveränderten Seiten vorbei die langsame Arbeit.

---

## 0.15.0

### Neu

- `if` / `else if` / `else` als **Ausdruck**: der Wert ist der letzte Ausdruck
  des gelaufenen Zweigs — dieselbe Regel, der eine Funktion schon folgt. Damit
  taugt es als Wert (`const LIMIT = if gestrichen { 300 } else { 260 }`) wie als
  Wächter um Anweisungen, ohne zweite Syntax. Ein Zweig, der nicht läuft,
  liefert `null`, und jeder Zweig hat einen eigenen Gültigkeitsbereich — einer
  außerhalb bereits vorhandenen Variablen zuzuweisen ändert weiterhin jene.

---

## 0.14.0

### Behoben

- `--var` erreicht jetzt auch `pdfl test` und `pdfl watch`, nicht nur `pdfl
  run`. Keines der beiden reichte es an die gestarteten Kindprozesse weiter, ein
  Skript, das `vars.*` liest, konnte also weder getestet noch beobachtet
  werden: jeder Fall oder jede Datei scheiterte mit „was not provided",
  unabhängig vom Inhalt.

---

## 0.13.0

### Bricht

- **`pdfl pack` verpackt keine Tabellendateien mehr** (`.xlsx`, `.xls`, `.ods`)
  und nennt die Datei, die draußen blieb. Keine `data::`-Funktion kann eine
  öffnen; ein Paket, das sie mitführte, installierte sauber und scheiterte beim
  ersten Nachschlagen. Wer eine Tabelle paketierte, exportiert sie vorher nach
  `.csv` oder `.json`.

### Neu

- `--tags TAG` bei `run` filtert, welche Checks laufen. Wiederholbar; ein Check
  läuft, wenn er einen der angegebenen Tags trägt.
- `--json` bei `inspect` und `lint`, `--output json` bei `doc`. Jeder
  Unterbefehl ist nun maschinell lesbar.
- `--output sarif` und `--output junit`, überall wo ein Berichtsformat gewählt
  wird — `run`, `compare`, `watch` und `fix`. SARIF liest GitHub code scanning;
  JUnit liest das Test-Panel jeder CI.
- `pdfl completions <shell>` gibt ein Vervollständigungsskript für bash, zsh,
  fish, elvish oder powershell aus.
- `--quiet` bei jedem Befehl unterdrückt Fortschritt und Bestätigungen auf
  stderr. Fehler erscheinen weiterhin, und `print()` bleibt unangetastet — das
  ist die Ausgabe des Skripts selbst, und sie zu schlucken würde ändern, was das
  Skript tut.
- `data::load_dataset` und `data::lookup_value` lesen neben `.csv` auch `.json`:
  ein Array von Arrays, oder ein Array von Objekten, dessen erstes Objekt die
  Spalten in der Reihenfolge der Datei benennt.
- `pdfl test <skript>` führt ein Skript über einen Ordner PDFs aus und
  vergleicht jeden Bericht mit dem daneben aufgezeichneten. Ein Profil, das
  plötzlich etwas anderes findet, lässt so einen Test scheitern, statt jemanden
  weiter hinten zu überraschen. `--update` zeichnet die erwarteten Berichte auf.
- `--jobs <n>` bei `pdfl test` lässt so viele Fälle gleichzeitig laufen, jeden
  als eigenen Prozess. Acht Dateien à 41 Seiten: 8,9s bei `--jobs 1`, 1,1s bei
  `--jobs 8`. Die Vorgabe bleibt `1`, da jeder Job ein Dokument im Speicher
  hält; `--jobs 0` gibt einen je CPU.
- `--jobs <n>` auch bei `pdfl watch`: die Dateien werden von Kindprozessen
  geprüft, ein Stapeldurchlauf skaliert also genauso (9,5s auf 1,2s bei acht
  Dateien à 41 Seiten). Der geschriebene Bericht ist unabhängig von `--jobs`
  identisch.
- `--events` bei `pdfl watch` wartet auf Dateisystem-Benachrichtigungen statt
  auf einen Zeitgeber. Opt-in, nicht Vorgabe: inotify meldet auf einem NFS- oder
  SMB-Mount nur, was die lokale Maschine schreibt, ein Netz-Hot-Folder verstummte
  also. Lässt sich der Watcher nicht anlegen, sagt watch das und nimmt wieder den
  Zeitgeber.
- `--journal <datei>` bei `pdfl watch`: ein nur angehängtes Protokoll des
  Geprüften, ein JSON-Objekt je Datei. Ein erneuter Lauf mit demselben Journal
  überspringt die darin verzeichneten Dateien — ein bei viertausend von
  fünftausend unterbrochener Stapel erledigt die restlichen tausend — meldet
  ihre Urteile aber weiterhin, ein fortgesetzter Stapel behauptet also nie, ein
  Ordner sei sauber.
- `--timeout <s>` bei `pdfl watch` beendet die Prüfung einer Datei nach so
  vielen Sekunden und meldet sie als abgelehnt — ein Befund,
  `check_name: "timeout"` — statt den Stapel hängen zu lassen. Rekursion in
  einem `.pdfl`-Skript ist bereits tiefenbegrenzt, das Flag ist also für das,
  was ein Skript nicht verursachen kann: pdfium, das bei einem fehlerhaften
  oder feindseligen PDF in eine Schleife gerät oder blockiert.

### Wissenswert

- Ein Tag, den kein Check trägt, ist ein **Fehler**, kein leeres Bestehen. Sonst
  würde eine Pipeline mit vertipptem Tag nichts prüfen und eine saubere Datei
  melden.
- Eine `rule` trägt keine Tags, `--tags` überspringt sie also — dieselbe Antwort
  wie für einen Check ohne Tag.
- Der JSON-Bericht hat `checks_run` bekommen: die Checks und Rules, die gelaufen
  sind. Das hebt `schema_version` nicht an, denn ein Leser, der unbekannte
  Felder ignoriert, übersteht es. JUnit braucht es: die Diagnosen nennen nur die
  Checks, die etwas gefunden haben, und ein sauberer Lauf, der als null Tests
  gemeldet wird, ist für eine CI ein Lauf, der nie stattfand.

### Behoben

- `pdfl watch` wacht jetzt auf, wenn die frischeste Datei fertig geschrieben
  ist, statt bis zu ein ganzes Intervall später. Mit `--debounce 3000` wird eine
  ankommende Datei nach etwa 3s gemeldet statt nach bis zu 6s.

---

## 0.12.0

### Neu

- Skripte nehmen Werte von der Kommandozeile: `--var name=wert`, gelesen als
  `vars.name`. Ein fehlender Wert nennt das Flag, das ihn liefern würde, statt
  sich zu nichts aufzulösen.
- Vier ausgearbeitete Beispiele für den Vergleich zweier Dokumente mit
  `visual::`: `proof.pdfl`, `reprint.pdfl`, `scope.pdfl` und `intake.pdfl`.

### Bricht

Nichts. Ein Skript, das `vars` nie erwähnt, verhält sich genau wie vorher.

---

## 0.11.0

### Bricht

**Die Diagnose-Kennungen haben eine andere Form.** Sie waren `PDFL-001`, ein
Zähler innerhalb des Laufs; jetzt leiten sie sich aus dem Befund selbst ab, etwa
`PDFL-093751a2`.

> Alles, was auf `PDFL-\d+` passt, passt nicht mehr. Dafür überlebt eine Kennung
> jetzt einen Check, der darüber eingefügt wird — und genau das macht eine
> freigegebene Baseline haltbar.

**Eine unlesbare Eingabe endet mit `10` statt `2`.** Eine kaputte und eine
abgelehnte Datei waren für eine Pipeline nicht unterscheidbar.

> Behandelt Ihre CI `2` als „diese Datei wurde abgelehnt", sieht sie jetzt `10`,
> wenn die Datei nie beurteilt wurde. Befunde nutzen weiterhin `0`, `1` und `2`;
> ein Syntaxfehler im Skript weiterhin `3`.

### Neu

- Ein Check kann den Schweregrad seiner Befunde deklarieren:
  `check "..." severity: warning { ... }` — `error` (Vorgabe), `warning` oder
  `info`. Erst dadurch hat `--fail-on warning` etwas, worauf es wirken kann.
- Der JSON-Bericht beginnt mit `schema_version`, damit ein Konsument weiß,
  welche Form er liest. Sie steigt nur, wenn ein Leser der vorherigen Ausgabe
  brechen würde; ein zusätzliches Feld lässt sie unverändert.

---

## 0.10.1

### Behoben

- Der PDF-Bericht war teilweise portugiesisch: die Abschnittsüberschrift lautete
  `Diagnósticos`, und jede Diagnose trug `(linha N)`. Beides ist jetzt englisch,
  wie es die Dokumentation immer versprochen hat.

---

## 0.10.0

### Bricht

**Die Release-Ziele heißen `amd64` statt `x64`**, damit hat sich jeder
Asset-Name geändert.

**Portable Archive werden nicht mehr veröffentlicht**, mit Ausnahme eines für
Linux amd64.

> Alles, was `pdfl-<version>-linux-x64.tar.gz` lädt, oder irgendein portables
> Archiv außer Linux amd64, muss angepasst werden. Installieren Sie in der CI
> aus dem `.deb` — die Rezepte dieser Dokumentation wurden darauf umgestellt —
> oder nehmen Sie das Linux-amd64-Tarball, wo Installieren nicht in Frage kommt.

### Behoben

- Zwei Lücken, gefunden beim Abgleich der Dokumentation mit dem Quelltext:
  `text::detect_personal_data` und `text::detect_pii` nehmen eine optionale
  Zeichenkette, die nirgends dokumentiert war, und `fix::reorder_pages` stand in
  zwei verschiedenen Schreibweisen zwischen den Sprachen.

---

## 0.9.0

### Neu

- Installer für jede Plattform: `.deb` für Linux, `.dmg` für macOS,
  `-setup.exe` und `.msi` für Windows.
- macOS-Intel-Builds, quer übersetzt vom Apple-Silicon-Runner.

### Behoben

- Der Windows-Installer wurde mit Pfaden gebaut, die gegen das falsche
  Verzeichnis auflösten, und erzeugte deshalb nie eine Datei.
- Die Release-Pakete trugen die C-Header und Build-Dateien von pdfium mit, die
  nur jemanden betreffen, der gegen pdfium compiliert. Rund 550 KB je Paket.

---

## 0.8.0

### Neu

- Windows x64 kommt zu den veröffentlichten Plattformen.

> Die mitgelieferte `pdfium.dll` liegt in `pdfium\bin`, nicht in `pdfium\lib`.
> Wenn Sie `pdfl` selbst paketieren, behalten Sie das ausgelieferte Layout: die
> Binärdatei sucht die Bibliothek neben sich.

---

## 0.7.0

### Bricht

**Release-Assets tragen die Version im Namen**, als
`pdfl-<version>-<ziel>.tar.gz`, und das Verzeichnis darin ebenso.

> `.../releases/latest/download/<name>` löst nicht mehr auf, denn dieser
> Endpunkt braucht den exakten Dateinamen. Laden Sie stattdessen per Muster:
> `gh release download --pattern 'pdfl-*-linux-amd64.*'`.

### Neu

- Quelltext, README und Beispiele sind auf Englisch. Die Dokumentation bleibt in
  sieben Sprachen.

---

## v0.6.1

Erste öffentliche Version. Die Sprache, der Interpreter und zehn CLI-Befehle,
mit Dokumentation in sieben Sprachen.

---

[← Rezepte](12-recipes.md) · [Inhalt](README.md)
