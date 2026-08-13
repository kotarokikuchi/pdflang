# 1. Le langage PDFLang

[← Sommaire](README.md) · [Suivant : types du document →](02-types.md)

PDFLang est conçu pour être lu par des gens qui n'écrivent pas de programmes.
Pas de classes, pas d'héritage, pas de déclarations de types, pas de
points-virgules. Un script est un ensemble de vérifications écrites presque en
langue naturelle.

---

## 1.1 Structure d'un script

```pdfl
// Un commentaire commence par deux barres obliques et va jusqu'au bout de la ligne.

profile "nom-du-profil" {         // profile est facultatif : il nomme et
                                  // regroupe l'ensemble, et son nom apparaît
                                  // dans le rapport.

  const LIMITE = 300%             // constantes : par convention en majuscules

  check "Nom du contrôle" {       // chaque check devient une section du rapport
    require doc.page_count > 0    // une validation
  }

  check "Autre contrôle" {        // autant de checks que nécessaire
    require doc.title != ""
  }
}
```

`profile` peut être omis — un script peut n'être qu'une suite de checks :

```pdfl
check "Simple" {
  require doc.page_count > 0
}
```

### Étiquettes sur les checks

Les étiquettes servent à classer et filtrer les checks dans le rapport :

```pdfl
check "Ink within limit" tags: ["prepress", "colors"] {
  require prepress::validate_tac_limits(300)
}
```

### Gravité d'un check

Par défaut un check en échec est une **erreur** et l'exécution sort avec 2. Un
check peut se déclarer consultatif :

```pdfl
check "Résolution des images" severity: warning {
  require !visual::detect_low_resolution(300)
}
```

`error` (par défaut), `warning` et `info`. Un avertissement ou une information
ne font pas échouer l'exécution — ils sortent avec 1 et 0 — sauf avec
`--fail-on warning`, par lequel la CI choisit sa sévérité sans toucher au script.

`tags:` et `severity:` peuvent venir dans n'importe quel ordre.

> Une erreur d'exécution dans le check — une variable mal orthographiée, un
> fichier absent — reste une erreur quoi qu'ait déclaré le check. Un script
> cassé n'est pas consultatif.

---

## 1.2 Deux façons de valider

Toute validation s'écrit avec `require` ou `assert`. La seule différence est le
message qui apparaît dans le rapport en cas d'échec.

```pdfl
check "Comparing both forms" {

  // require : le message est fabriqué à partir de l'expression elle-même.
  // En cas d'échec, le rapport affiche :
  //   "requirement not met: doc.page_count > 0"
  require doc.page_count > 0

  // assert : c'est vous qui écrivez le message que lira le destinataire.
  // En cas d'échec, il apparaît tel quel :
  //   "PDF has no title in its metadata"
  assert doc.title != "", "PDF has no title in its metadata"
}
```

**Règle pratique :** `require` quand l'expression se lit toute seule ; `assert`
quand la personne qui lira le rapport doit comprendre le problème sans connaître
le script.

### Un échec n'arrête pas les autres contrôles

```pdfl
check "Three independent validations" {
  assert doc.page_count > 100, "too few pages"    // échoue
  assert doc.title != "", "no title"              // s'exécute quand même
  assert doc.author != "", "no author"            // celle-ci aussi
}
```

Le rapport liste **tous** les problèmes d'un coup. C'est volontaire : la
personne qui reçoit le fichier veut la liste complète des corrections, pas une
correction à la fois.

Il en va de même entre les checks — si un check rencontre une erreur d'exécution
(une variable inconnue, par exemple), elle devient un diagnostic et les autres
checks continuent.

---

## 1.3 Valeurs et types

### Nombres et unités

```pdfl
check "Numbers" {
  x = 42          // entier
  y = 2.5         // décimal

  // Les unités de longueur sont converties en points (1 pt = 1/72 pouce) :
  a = 3mm         // 8,5039... pt
  b = 2.5cm       // 70,866... pt
  c = 1in         // 72 pt
  d = 10pt        // 10 pt

  // Le pourcentage garde la valeur telle quelle :
  limite = 300%   // 300

  require a < b            // tout est en points, la comparaison est directe
  require c == 72.0
  require limite == 300
}
```

Pouvoir écrire `3mm` au lieu de `8.504` est précisément l'intérêt : cela se lit
naturellement pour qui pense en millimètres, et la conversion ne se trompe pas.

### Texte

```pdfl
check "Strings" {
  simple = "texte simple"

  // Interpolation : #{...} insère la valeur de n'importe quelle expression
  nom = "document.pdf"
  message = "Analyzing #{nom} with #{doc.page_count} pages"

  // Échappements : \n (saut de ligne), \t (tabulation), \" (guillemet), \\ (barre)
  cite = "il a dit \"bonjour\""

  // Une barre oblique inverse inconnue est conservée telle quelle — c'est ce qui
  // permet d'écrire des expressions régulières sans double échappement :
  motif = "\d{3}\.\d{3}\.\d{3}-\d{2}"

  require message.contains("pages")
}
```

### Booléens et ce qui est « vrai »

```pdfl
check "True and false" {
  oui = true
  non = false

  // Seuls false et null sont faux. Tout le reste est vrai —
  // y compris 0, la chaîne vide et la liste vide.
  require 0        // passe (0 est vrai)
  require ""       // passe (la chaîne vide est vraie)

  // Donc pour vérifier un contenu, comparez explicitement :
  require doc.title != ""              // correct
  require doc.pages.length > 0         // correct
}
```

C'est utile avec les fonctions qui retournent `null` :

```pdfl
check "Taking advantage of null" {
  description = data::lookup_value("batches.csv", "L2026-08")
  // null est faux, on peut donc écrire directement :
  assert description, "batch not found in the table"
}
```

### Listes

```pdfl
check "Lists" {
  nombres = [1, 2, 3]
  mots = ["a", "b", "c"]
  melange = [1, "deux", true]

  require nombres.length == 3
  require nombres.contains(2)
  require mots.join(", ") == "a, b, c"

  // L'accès commence à 1 : le premier élément est le 1er
  require nombres.get(1) == 1
  require nombres.first() == 1
  require nombres.last() == 3
}
```

---

## 1.4 Opérateurs

```pdfl
check "Operators" {
  // Comparaison
  require 10 > 5
  require 10 >= 10
  require 3 < 4
  require 3 <= 3
  require "a" == "a"
  require "a" != "b"

  // Arithmétique
  require 2 + 3 == 5
  require 10 - 4 == 6
  require 3 * 4 == 12
  require 10 / 4 == 2.5        // division non entière : résultat décimal
  require 10 / 5 == 2          // division exacte : reste entier

  // Logique (évaluation court-circuit : la droite n'est évaluée qu'au besoin)
  require true && true
  require false || true
  require !false

  // Usage concret du court-circuit : sans pages, la droite n'est jamais
  // évaluée et un document vide ne provoque pas d'erreur.
  require doc.page_count == 0 || doc.pages.first().width > 0
}
```

---

## 1.5 Les blocs : répéter pour chaque élément

Un bloc est du code entre accolades, avec ses paramètres entre deux barres
verticales. Cela se lit « pour chaque page, faire… ».

```pdfl
check "Walking through pages" {

  // each : exécute le bloc pour chaque élément
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  // each_with_index : donne aussi la position (0, 1, 2…)
  doc.fonts.each_with_index { |font, i|
    print("font", i, ":", font.name)
  }

  // all : vrai si tous les éléments satisfont la condition
  require doc.fonts.all { |f| f.is_embedded }

  // any : vrai si au moins un élément la satisfait
  require doc.pages.any { |p| p.extract_text() != "" }

  // filter : ne garde que les éléments qui satisfont la condition
  vides = doc.pages.filter { |p| p.extract_text() == "" }
  assert vides.length == 0, "#{vides.length} blank page(s)"

  // map : transforme chaque élément en une nouvelle liste
  noms = doc.fonts.map { |f| f.name }
  print("fonts in use:", noms.join(", "))
}
```

Les blocs s'enchaînent — mais **sur la même ligne** : pas de retour à la ligne
avant le point.

```pdfl
check "Chaining" {
  // Polices non incorporées, seulement les noms, réunis par des virgules
  problemes = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  assert problemes.length == 0,
    "fonts not embedded: #{problemes.join(", ")}"
}
```

Si la ligne devient trop longue, coupez-la en étapes nommées plutôt que de
briser l'enchaînement — c'est de toute façon plus lisible :

```pdfl
check "Named steps" {
  libres = doc.fonts.filter { |f| !f.is_embedded }
  noms = libres.map { |f| f.name }
  assert noms.length == 0, "fonts not embedded: #{noms.join(", ")}"
}
```

---

## 1.6 Les fonctions : donner un nom à une règle

Quand la même validation revient plusieurs fois, donnez-lui un nom :

```pdfl
// La valeur d'une fonction est celle de sa dernière expression — pas de return.
function est_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}

function trop_encre(page, limite) {
  page.tac > limite
}

check "Format and ink" {
  // Le check se lit alors presque comme une phrase
  require doc.pages.all { |p| est_a4(p) }

  doc.pages.each { |page|
    assert !trop_encre(page, 300), "page #{page.number} has too much ink"
  }
}
```

Règles des fonctions :

- Les paramètres n'existent qu'à l'intérieur de la fonction.
- Une fonction peut en appeler d'autres.
- La récursion est permise, plafonnée à 200 appels (pour qu'un script emballé ne
  bloque pas le processus).

---

## 1.7 import : réutiliser entre profils

Mettez les règles communes dans un fichier et importez-le où vous en avez
besoin.

`bibliotheque.pdfl` :

```pdfl
// Constantes et fonctions partagées par l'équipe
const TAC_OFFSET = 300%
const FOND_PERDU = 3mm

function page_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}
```

`magazine.pdfl` :

```pdfl
// Le chemin est relatif à CE fichier
import "bibliotheque.pdfl"

check "Format" {
  // TAC_OFFSET et page_a4 viennent de l'import
  require doc.pages.all { |p| page_a4(p) }
  require prepress::validate_tac_limits(TAC_OFFSET)
}
```

Un même fichier n'est chargé **qu'une seule fois**, même si plusieurs scripts
l'importent — les imports circulaires ne bloquent donc rien.

---

## 1.8 rule : valider page par page

Une `rule` est un check exécuté une fois par page, la page étant déjà liée à la
variable `page` :

```pdfl
// Sans "on" : s'exécute sur toutes les pages
rule "Every page has text" {
  assert page.extract_text().trim() != "",
    "page #{page.number} is blank"
}
```

Avec `on`, vous choisissez les pages concernées :

```pdfl
rule "Body pages numbered" on doc.pages.filter { |p| p.number > 2 } {
  pied = region(0, 0, page.width, 60)
  assert text::extract_from_region(page.number, pied) != "",
    "page #{page.number} has no page number in the footer"
}
```

> **Point de syntaxe :** si l'expression après `on` se termine par une propriété
> (comme `on doc.pages`), mettez-la entre parenthèses ; sinon l'accolade du corps
> serait prise pour un bloc de cet appel :
>
> ```pdfl
> rule "Example" on (doc.pages) {     // parenthèses nécessaires
>   require page.width > 0
> }
> ```

---

## 1.9 Variables et portée

```pdfl
const GLOBAL = 100          // visible dans tout le fichier

check "Scope" {
  locale = 42               // visible seulement dans ce check

  doc.pages.each { |page|
    interne = page.width    // visible seulement dans ce bloc
    require interne > 0
  }

  require locale == 42      // toujours visible
  require GLOBAL == 100     // toujours visible
}
```

L'usage veut des majuscules pour les constantes et des minuscules pour les
variables. Le langage ne l'impose pas, mais les exemples et les profils fournis
suivent cette convention.

---

### Des valeurs venues de la ligne de commande

`--var nom=valeur` sur `pdfl run`, `pdfl test` et `pdfl watch` parvient au script
sous la forme `vars.nom`, toujours en texte. `test` et `watch` transmettent la
même valeur à chaque cas ou fichier : un nom de client pour toute l'exécution,
pas un par fichier. C'est ce qui évite qu'un profil devienne cinq copies presque
identiques :

```pdfl
check "Le travail correspond à la commande" {
  assert doc.title.contains(vars.commande),
    "le fichier dit \"#{doc.title}\", la commande est #{vars.commande}"
}
```

```bash
pdfl run reception.pdfl recu.pdf --var commande=SO-4471
```

Un nom non transmis est une **erreur qui nomme l'option censée le fournir**, et
non une chaîne vide : un check comparant à rien passerait, et annoncerait un
fichier que personne n'a validé.

---

## 1.10 Des messages utiles à qui reçoit le fichier

La qualité du rapport tient aux messages que vous écrivez. Comparez :

```pdfl
check "Poor messages" {
  require doc.pages.all { |p| p.tac <= 300 }
  // Rapport : "requirement not met: doc.pages.all() { ... }"
  // — le destinataire ne sait ni quelle page ni de combien.
}

check "Good messages" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "Page #{page.number}: ink coverage #{page.tac}% (max 300%)"
  }
  // Rapport : "Page 7: ink coverage 324% (max 300%)"
  // — l'opérateur sait immédiatement quoi corriger.
}
```

Pour les informations complémentaires qui ne sont pas des erreurs, utilisez
`print()`. Sa sortie va sur la sortie d'erreur et ne pollue pas le rapport :

```pdfl
check "Context" {
  print("Analyzing", doc.page_count, "pages")
  print("Fonts:", prepress::list_fonts().join(", "))
  require doc.page_count > 0
}
```

---

## 1.11 Erreurs courantes

| Message | Cause | Correction |
|---|---|---|
| `expected end of line after statement` | Deux instructions sur une ligne | Une instruction par ligne |
| `unknown variable: x` | Utilisée avant l'affectation, ou hors de portée | Déclarez-la au même niveau |
| `unknown function: text::xyz` | Nom erroné ou fonction inexistante | Voyez le chapitre de l'espace de noms |
| `fix:: is only available in the 'pdfl fix' command` | `fix::` employé avec `pdfl run` | `pdfl fix input.pdf script.pdfl --output out.pdf` |
| `unknown unit: 'kg'` | Unité invalide | Utilisez `pt`, `mm`, `cm`, `in` ou `%` |
| `expected '{' with the rule body` | L'expression après `on` finit par une propriété | Mettez-la entre parenthèses |
| `unexpected expression: Dot` | Enchaînement coupé par un retour à la ligne | Gardez `.methode` sur la même ligne, ou passez par une variable |

Avant d'exécuter, ces deux commandes valent toujours la peine :

```bash
pdfl lint mon_profil.pdfl    # variables inutilisées, checks en double…
pdfl fmt mon_profil.pdfl     # mise en forme uniforme
```

---

[← Sommaire](README.md) · [Suivant : types du document →](02-types.md)
