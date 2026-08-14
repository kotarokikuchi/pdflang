# 1. Die Sprache PDFLang

[← Inhalt](README.md) · [Weiter: Typen des Dokuments →](02-types.md)

PDFLang ist so entworfen, dass Menschen es lesen können, die keine Programme
schreiben. Keine Klassen, keine Vererbung, keine Typdeklarationen, keine
Semikolons. Ein Skript ist eine Sammlung von Prüfungen, fast in natürlicher
Sprache geschrieben.

---

## 1.1 Aufbau eines Skripts

```pdfl
// Ein Kommentar beginnt mit zwei Schrägstrichen und reicht bis zum Zeilenende.

profile "profil-name" {           // profile ist optional: es benennt und
                                  // gruppiert das Ganze, und der Name
                                  // erscheint im Bericht.

  const GRENZE = 300%             // Konstanten: üblicherweise in Großbuchstaben

  check "Name der Prüfung" {      // jeder check wird ein Abschnitt im Bericht
    require doc.page_count > 0    // eine Prüfung
  }

  check "Weitere Prüfung" {       // beliebig viele checks
    require doc.title != ""
  }
}
```

`profile` darf entfallen — ein Skript kann auch nur eine Folge von checks sein:

```pdfl
check "Simple" {
  require doc.page_count > 0
}
```

### Etiketten an checks

Etiketten dienen dazu, checks im Bericht zu ordnen und zu filtern:

```pdfl
check "Ink within limit" tags: ["prepress", "colors"] {
  require prepress::validate_tac_limits(300)
}
```

### Schweregrad eines Checks

Standardmäßig ist ein fehlschlagender Check ein **Fehler** und der Lauf endet
mit 2. Ein Check kann sich stattdessen als beratend deklarieren:

```pdfl
check "Bildauflösung" severity: warning {
  require !visual::detect_low_resolution(300)
}
```

`error` (Vorgabe), `warning` und `info`. Warnung und Hinweis lassen den Lauf
nicht scheitern — sie enden mit 1 und 0 — außer mit `--fail-on warning`, womit
die CI die Strenge bestimmt, ohne das Skript zu ändern.

`tags:` und `severity:` dürfen in beliebiger Reihenfolge stehen.

> Ein Laufzeitfehler im Check — ein Tippfehler in einer Variablen, eine fehlende
> Datei — bleibt ein Fehler, ganz gleich was der Check deklariert hat. Ein
> kaputtes Skript ist nicht beratend.

---

## 1.2 Zwei Arten zu prüfen

Jede Prüfung wird mit `require` oder `assert` geschrieben. Der einzige
Unterschied ist die Meldung, die bei einem Fehlschlag im Bericht steht.

```pdfl
check "Comparing both forms" {

  // require: Die Meldung entsteht aus dem Ausdruck selbst.
  // Bei einem Fehlschlag zeigt der Bericht:
  //   "requirement not met: doc.page_count > 0"
  require doc.page_count > 0

  // assert: Sie schreiben die Meldung, die die empfangende Person liest.
  // Bei einem Fehlschlag erscheint sie unverändert:
  //   "PDF has no title in its metadata"
  assert doc.title != "", "PDF has no title in its metadata"
}
```

**Faustregel:** `require`, wenn der Ausdruck für sich spricht; `assert`, wenn
die Person, die den Bericht liest, das Problem verstehen soll, ohne das Skript
zu kennen.

### Ein Fehlschlag hält die übrigen Prüfungen nicht auf

```pdfl
check "Three independent validations" {
  assert doc.page_count > 100, "too few pages"    // schlägt fehl
  assert doc.title != "", "no title"              // läuft trotzdem
  assert doc.author != "", "no author"            // diese auch
}
```

Der Bericht listet **alle** Probleme auf einmal. Das ist Absicht: Wer die Datei
bekommt, will die vollständige Korrekturliste, nicht eine Korrektur nach der
anderen.

Zwischen den checks gilt dasselbe — trifft ein check auf einen Laufzeitfehler
(etwa eine unbekannte Variable), wird daraus eine Diagnose, und die übrigen
checks laufen weiter.

---

## 1.3 Werte und Typen

### Zahlen und Einheiten

```pdfl
check "Numbers" {
  x = 42          // ganze Zahl
  y = 2.5         // Dezimalzahl

  // Längeneinheiten werden in Punkt umgerechnet (1 pt = 1/72 Zoll):
  a = 3mm         // 8,5039... pt
  b = 2.5cm       // 70,866... pt
  c = 1in         // 72 pt
  d = 10pt        // 10 pt

  // Prozent behält den Zahlenwert:
  grenze = 300%   // 300

  require a < b            // alles in Punkt, direkt vergleichbar
  require c == 72.0
  require grenze == 300
}
```

`3mm` statt `8.504` schreiben zu können, ist genau der Punkt: Es liest sich
natürlich für jemanden, der in Millimetern denkt, und die Umrechnung geht nicht
daneben.

### Text

```pdfl
check "Strings" {
  einfach = "einfacher Text"

  // Interpolation: #{...} setzt den Wert eines beliebigen Ausdrucks ein
  name = "dokument.pdf"
  meldung = "Analyzing #{name} with #{doc.page_count} pages"

  // Escapes: \n (Zeilenumbruch), \t (Tabulator), \" (Anführungszeichen), \\ (Backslash)
  zitat = "er sagte \"hallo\""

  // Ein unbekannter Backslash bleibt erhalten — dadurch lassen sich reguläre
  // Ausdrücke ohne doppeltes Escapen schreiben:
  muster = "\d{3}\.\d{3}\.\d{3}-\d{2}"

  require meldung.contains("pages")
}
```

### Wahrheitswerte und was „wahr“ ist

```pdfl
check "True and false" {
  ja = true
  nein = false

  // Nur false und null sind falsch. Alles andere ist wahr —
  // auch 0, die leere Zeichenkette und die leere Liste.
  require 0        // besteht (0 ist wahr)
  require ""       // besteht (die leere Zeichenkette ist wahr)

  // Um Inhalt zu prüfen, vergleichen Sie also ausdrücklich:
  require doc.title != ""              // richtig
  require doc.pages.length > 0         // richtig
}
```

Nützlich bei Funktionen, die `null` zurückgeben:

```pdfl
check "Taking advantage of null" {
  beschreibung = data::lookup_value("batches.csv", "L2026-08")
  // null ist falsch, deshalb geht das direkt:
  assert beschreibung, "batch not found in the table"
}
```

### Listen

```pdfl
check "Lists" {
  zahlen = [1, 2, 3]
  woerter = ["a", "b", "c"]
  gemischt = [1, "zwei", true]

  require zahlen.length == 3
  require zahlen.contains(2)
  require woerter.join(", ") == "a, b, c"

  // Der Zugriff beginnt bei 1: das erste Element ist das 1.
  require zahlen.get(1) == 1
  require zahlen.first() == 1
  require zahlen.last() == 3
}
```

---

## 1.4 Operatoren

```pdfl
check "Operators" {
  // Vergleich
  require 10 > 5
  require 10 >= 10
  require 3 < 4
  require 3 <= 3
  require "a" == "a"
  require "a" != "b"

  // Arithmetik
  require 2 + 3 == 5
  require 10 - 4 == 6
  require 3 * 4 == 12
  require 10 / 4 == 2.5        // geht nicht auf: Ergebnis ist dezimal
  require 10 / 5 == 2          // geht auf: bleibt ganzzahlig

  // Logik (Kurzschlussauswertung: rechts wird nur bei Bedarf ausgewertet)
  require true && true
  require false || true
  require !false

  // Praktischer Nutzen des Kurzschlusses: ohne Seiten wird rechts nie
  // ausgewertet, ein leeres Dokument erzeugt also keinen Fehler.
  require doc.page_count == 0 || doc.pages.first().width > 0
}
```

---

## 1.5 `if`: zwischen zwei Dingen wählen

`if` ist ein **Ausdruck**, wie alles hier, was einen Wert liefert: es gibt den
letzten Ausdruck des gelaufenen Zweigs zurück. Also taugt es als Wert:

```pdfl
check "Das Farblimit hängt vom Papier ab" {
  // vars.papier kommt von --var papier=gestrichen
  const LIMIT = if vars.papier == "gestrichen" { 300 } else { 260 }

  doc.pages.each { |page|
    assert page.tac <= LIMIT,
      "Seite #{page.number}: #{page.tac}% Farbe, Grenze #{LIMIT}%"
  }
}
```

und als Wächter um Anweisungen, ohne `else`:

```pdfl
check "Den Umschlag nur prüfen, wenn es einen gibt" {
  if doc.page_count > 1 {
    require doc.pages.first().width > 0
  }
}
```

`else if` verkettet ohne zusätzliche Klammern:

```pdfl
groesse = if doc.page_count > 500 { "groß" }
          else if doc.page_count > 50 { "mittel" }
          else { "klein" }
```

Drei Dinge, die man wissen sollte:

- **Ein Zweig, der nicht läuft, liefert `null`.** Ein `if` ohne `else` mit
  falscher Bedingung ist `null` — und das ist falsch, also direkt prüfbar.
- **Jeder Zweig hat einen eigenen Gültigkeitsbereich**, wie ein Block oder eine
  Funktion. Eine im Zweig angelegte Variable existiert danach nicht mehr. Einer
  Variablen zuzuweisen, die außerhalb schon existiert, ändert weiterhin jene.
- **Die `{` nach der Bedingung öffnet den Rumpf.** Endet die Bedingung selbst in
  einem Block, setzen Sie sie in Klammern, sonst würde die Klammer des Rumpfes
  als die jenes Blocks gelesen:

```pdfl
// falsch: die { wird als Rumpf des if genommen
// if doc.pages.all { |p| p.width > 0 } { ... }

// richtig
if (doc.pages.all { |p| p.width > 0 }) {
  require doc.page_count > 0
}
```

> `if` dient dazu, einen Wert zu wählen oder eine Anweisung zu schützen — nicht
> dazu, eine gescheiterte Prüfung durch eine stille zu ersetzen. Eine Validierung,
> die scheitern soll, scheitert weiterhin; siehe [1.2](#12-zwei-arten-zu-validieren).

---

## 1.6 Blöcke: für jedes Element wiederholen

Ein Block ist Code in geschweiften Klammern, mit den Parametern zwischen zwei
senkrechten Strichen. Es liest sich als „für jede Seite tue …“.

```pdfl
check "Walking through pages" {

  // each: führt den Block für jedes Element aus
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  // each_with_index: gibt zusätzlich die Position (0, 1, 2 …)
  doc.fonts.each_with_index { |font, i|
    print("font", i, ":", font.name)
  }

  // all: wahr, wenn alle Elemente die Bedingung erfüllen
  require doc.fonts.all { |f| f.is_embedded }

  // any: wahr, wenn mindestens eines sie erfüllt
  require doc.pages.any { |p| p.extract_text() != "" }

  // filter: behält nur die Elemente, die sie erfüllen
  leere = doc.pages.filter { |p| p.extract_text() == "" }
  assert leere.length == 0, "#{leere.length} blank page(s)"

  // map: verwandelt jedes Element in eine neue Liste
  namen = doc.fonts.map { |f| f.name }
  print("fonts in use:", namen.join(", "))
}
```

Blöcke lassen sich verketten — aber **in derselben Zeile**: kein Zeilenumbruch
vor dem Punkt.

```pdfl
check "Chaining" {
  // Nicht eingebettete Schriften, nur die Namen, mit Komma verbunden
  probleme = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  assert probleme.length == 0,
    "fonts not embedded: #{probleme.join(", ")}"
}
```

Wird die Zeile zu lang, teilen Sie sie in benannte Schritte, statt die Kette zu
brechen — das liest sich ohnehin besser:

```pdfl
check "Named steps" {
  lose = doc.fonts.filter { |f| !f.is_embedded }
  namen = lose.map { |f| f.name }
  assert namen.length == 0, "fonts not embedded: #{namen.join(", ")}"
}
```

---

## 1.7 Funktionen: einer Regel einen Namen geben

Wenn dieselbe Prüfung mehrfach auftaucht, geben Sie ihr einen Namen:

```pdfl
// Der Wert einer Funktion ist der ihres letzten Ausdrucks — kein return.
function ist_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}

function zu_viel_farbe(page, grenze) {
  page.tac > grenze
}

check "Format and ink" {
  // So liest sich der check fast wie ein Satz
  require doc.pages.all { |p| ist_a4(p) }

  doc.pages.each { |page|
    assert !zu_viel_farbe(page, 300), "page #{page.number} has too much ink"
  }
}
```

Regeln für Funktionen:

- Parameter gelten nur innerhalb der Funktion.
- Eine Funktion darf andere aufrufen.
- Rekursion ist erlaubt, begrenzt auf 200 Aufrufe (damit ein außer Kontrolle
  geratenes Skript den Prozess nicht blockiert).

---

## 1.8 import: zwischen Profilen wiederverwenden

Legen Sie gemeinsame Regeln in eine Datei und importieren Sie sie dort, wo Sie
sie brauchen.

`bibliothek.pdfl`:

```pdfl
// Vom Team gemeinsam genutzte Konstanten und Funktionen
const OFFSET_TAC = 300%
const STANDARD_ANSCHNITT = 3mm

function a4_seite(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}
```

`magazin.pdfl`:

```pdfl
// Der Pfad ist relativ zu DIESER Datei
import "bibliothek.pdfl"

check "Format" {
  // OFFSET_TAC und a4_seite stammen aus dem Import
  require doc.pages.all { |p| a4_seite(p) }
  require prepress::validate_tac_limits(OFFSET_TAC)
}
```

Dieselbe Datei wird **nur einmal** geladen, auch wenn mehrere Skripte sie
importieren — zyklische Importe blockieren also nichts.

---

## 1.9 rule: Seite für Seite prüfen

Eine `rule` ist ein check, der einmal pro Seite läuft, wobei die Seite bereits an
die Variable `page` gebunden ist:

```pdfl
// Ohne "on": läuft auf allen Seiten
rule "Every page has text" {
  assert page.extract_text().trim() != "",
    "page #{page.number} is blank"
}
```

Mit `on` wählen Sie die betroffenen Seiten:

```pdfl
rule "Body pages numbered" on doc.pages.filter { |p| p.number > 2 } {
  fuss = region(0, 0, page.width, 60)
  assert text::extract_from_region(page.number, fuss) != "",
    "page #{page.number} has no page number in the footer"
}
```

> **Zur Syntax:** Endet der Ausdruck nach `on` mit einer Eigenschaft (etwa
> `on doc.pages`), setzen Sie ihn in Klammern; sonst würde die geschweifte
> Klammer des Rumpfes als Block dieses Aufrufs gelesen:
>
> ```pdfl
> rule "Example" on (doc.pages) {     // Klammern nötig
>   require page.width > 0
> }
> ```

---

## 1.10 Variablen und Gültigkeitsbereich

```pdfl
const GLOBAL = 100          // in der ganzen Datei sichtbar

check "Scope" {
  lokal = 42                // nur in diesem check sichtbar

  doc.pages.each { |page|
    innen = page.width      // nur in diesem Block sichtbar
    require innen > 0
  }

  require lokal == 42       // weiterhin sichtbar
  require GLOBAL == 100     // weiterhin sichtbar
}
```

Üblich sind Großbuchstaben für Konstanten und Kleinbuchstaben für Variablen. Die
Sprache erzwingt das nicht, aber die Beispiele und die mitgelieferten Profile
halten sich daran.

---

### Werte von der Kommandozeile

`--var name=wert` bei `pdfl run`, `pdfl test` und `pdfl watch` erreicht das Skript
als `vars.name`, immer als Text. `test` und `watch` reichen denselben Wert an
jeden Fall oder jede Datei weiter — ein Kundenname für den ganzen Lauf, nicht
einer je Datei. Das verhindert, dass aus einem Profil fünf fast gleiche Kopien werden:

```pdfl
check "Auftrag passt zur Bestellung" {
  assert doc.title.contains(vars.auftrag),
    "die Datei sagt \"#{doc.title}\", der Auftrag ist #{vars.auftrag}"
}
```

```bash
pdfl run eingang.pdfl erhalten.pdf --var auftrag=SO-4471
```

Ein nicht übergebener Name ist ein **Fehler, der das Flag nennt, das ihn liefern
würde** — keine leere Zeichenkette: ein Check, der gegen nichts vergleicht,
bestünde sonst und meldete eine Datei, die niemand geprüft hat.

---

## 1.11 Meldungen, die dem Empfänger helfen

Die Qualität des Berichts hängt an den Meldungen, die Sie schreiben. Vergleichen
Sie:

```pdfl
check "Poor messages" {
  require doc.pages.all { |p| p.tac <= 300 }
  // Bericht: "requirement not met: doc.pages.all() { ... }"
  // — die empfangende Person weiß weder welche Seite noch um wie viel.
}

check "Good messages" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "Page #{page.number}: ink coverage #{page.tac}% (max 300%)"
  }
  // Bericht: "Page 7: ink coverage 324% (max 300%)"
  // — die Bedienperson weiß sofort, was zu ändern ist.
}
```

Für Zusatzinformationen, die keine Fehler sind, nehmen Sie `print()`. Die
Ausgabe geht auf die Fehlerausgabe und verschmutzt den Bericht nicht:

```pdfl
check "Context" {
  print("Analyzing", doc.page_count, "pages")
  print("Fonts:", prepress::list_fonts().join(", "))
  require doc.page_count > 0
}
```

---

## 1.12 Häufige Fehler

| Meldung | Ursache | Abhilfe |
|---|---|---|
| `expected end of line after statement` | Zwei Anweisungen in einer Zeile | Eine Anweisung pro Zeile |
| `unknown variable: x` | Vor der Zuweisung benutzt oder außerhalb des Bereichs | Auf derselben Ebene deklarieren |
| `unknown function: text::xyz` | Falscher Name oder Funktion existiert nicht | Kapitel des Namensraums nachschlagen |
| `fix:: is only available in the 'pdfl fix' command` | `fix::` unter `pdfl run` benutzt | `pdfl fix input.pdf script.pdfl --output out.pdf` |
| `unknown unit: 'kg'` | Ungültige Einheit | `pt`, `mm`, `cm`, `in` oder `%` verwenden |
| `expected '{' with the rule body` | Ausdruck nach `on` endet mit einer Eigenschaft | In Klammern setzen |
| `the '{' here opens the body of the if` | die `if`-Bedingung endet in einem Block | In Klammern setzen |
| `unexpected expression: Dot` | Kette durch Zeilenumbruch getrennt | `.methode` in derselben Zeile lassen oder Zwischenvariable nutzen |

Vor dem Ausführen lohnen sich diese beiden Befehle immer:

```bash
pdfl lint mein_profil.pdfl    # ungenutzte Variablen, doppelte checks …
pdfl fmt mein_profil.pdfl     # einheitliche Formatierung
```

---

[← Inhalt](README.md) · [Weiter: Typen des Dokuments →](02-types.md)
