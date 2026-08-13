# 13. Änderungen

[← Rezepte](12-recipes.md) · [Inhalt](README.md)

Was sich in jeder Version geändert hat und was davon bei Ihnen brechen kann.

Die Version ist noch `0.x`, ein Minor-Sprung darf also etwas brechen. Wenn er es
tut, sagt der Eintrag genau was und wie man sich anpasst. Hier ändert sich
nichts stillschweigend.

---

## Noch nicht veröffentlicht

### Neu

- `--tags TAG` bei `run` filtert, welche Checks laufen. Wiederholbar; ein Check
  läuft, wenn er einen der angegebenen Tags trägt.
- `--json` bei `inspect` und `lint`, `--output json` bei `doc`. Jeder
  Unterbefehl ist nun maschinell lesbar.

### Wissenswert

- Ein Tag, den kein Check trägt, ist ein **Fehler**, kein leeres Bestehen. Sonst
  würde eine Pipeline mit vertipptem Tag nichts prüfen und eine saubere Datei
  melden.
- Eine `rule` trägt keine Tags, `--tags` überspringt sie also — dieselbe Antwort
  wie für einen Check ohne Tag.

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
