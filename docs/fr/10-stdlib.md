# 10. Bibliothèque standard

[← `data::`](09-data.md) · [Sommaire](README.md) · [Suivant : ligne de commande →](11-cli.md)

Les méthodes des listes et des chaînes, plus les fonctions globales disponibles
partout dans un script.

---

## 10.1 Méthodes des listes

| Méthode | Rôle |
|---|---|
| `liste.each { \|item\| ... }` | Exécute le bloc pour chaque élément |
| `liste.each_with_index { \|item, i\| ... }` | Donne aussi la position (à partir de **0**) |
| `liste.all { \|item\| ... }` | Vrai si tous satisfont la condition (vrai sur liste vide) |
| `liste.any { \|item\| ... }` | Vrai si au moins un la satisfait (faux sur liste vide) |
| `liste.filter { \|item\| ... }` | Ne garde que ceux qui la satisfont |
| `liste.map { \|item\| ... }` | Nouvelle liste transformée |
| `liste.length` | Nombre d'éléments (`length()` marche aussi) |
| `liste.contains(valeur)` | La valeur est-elle dans la liste ? |
| `liste.get(n)` | N-ième élément (à partir de **1**) |
| `liste.first()` / `liste.last()` | Premier / dernier (`null` si la liste est vide) |
| `liste.join([séparateur])` | Réunit en une chaîne (`", "` par défaut) |

```pdfl
check "List methods" {
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  doc.fonts.each_with_index { |font, i|
    print("font", i + 1, "of", doc.fonts.length, ":", font.name)
  }

  require doc.fonts.all { |f| f.is_embedded }
  assert doc.pages.any { |p| p.extract_text() != "" },
    "the entire document has no text"

  mauvaises = doc.images.filter { |img| img.dpi < 300 }
  assert mauvaises.length == 0, "#{mauvaises.length} image(s) with low resolution"

  print("fonts:", doc.fonts.map { |f| f.name }.join(", "))

  // get part de 1 : get(1) est le premier élément
  ligne = data::load_dataset("donnees/lots.csv").get(2)
  print("first column:", ligne.get(1))

  // Sûr même sur une liste vide : null est faux
  tons = prepress::detect_spot_colors()
  assert !tons.first() || tons.first() == "Varnish",
    "unexpected special ink: #{tons.first()}"
}
```

---

## 10.2 Méthodes des chaînes

| Méthode | Rôle |
|---|---|
| `texte.contains(sous)` | Contient-il ce fragment ? |
| `texte.starts_with(sous)` | Commence-t-il par ce fragment ? |
| `texte.ends_with(sous)` | Finit-il par ce fragment ? |
| `texte.trim()` | Retire les espaces de début et de fin |
| `texte.to_uppercase()` | Tout en majuscules |
| `texte.to_lowercase()` | Tout en minuscules |
| `texte.length` | Nombre de caractères |

```pdfl
check "String methods" {
  titre = doc.title
  require titre.length > 0
  require titre.trim() == titre          // aucun espace superflu
  assert !titre.to_lowercase().contains("draft"),
    "title still marked as draft"

  code = codes::decode_barcode(1)
  assert code.starts_with("789"), "GTIN is not Brazilian"
  assert doc.filename.ends_with(".pdf"), "unexpected extension"
}

check "contains on each type" {
  // Chaîne : cherche un FRAGMENT dans le texte
  require "final document".contains("final")

  // Liste : cherche un ÉLÉMENT complet
  require ["a", "b"].contains("a")
  require !["ab"].contains("a")      // "a" n'est pas un élément de cette liste
}
```

---

## 10.3 Fonctions globales

| Fonction | Rôle |
|---|---|
| `min(a, b)` / `max(a, b)` | Le plus petit / le plus grand |
| `abs(x)` | Valeur absolue |
| `round(x)` | Arrondit à l'entier le plus proche |
| `print(...)` | Affiche, séparé par des espaces, sur la **sortie d'erreur** |
| `region(x, y, l, h [, nom])` | Crée une région ([chapitre 2](02-types.md)) |

`print` écrit sur la sortie d'erreur : `> rapport.json` ne reçoit donc que le
rapport.

```pdfl
check "Global functions" {
  const LARGEUR_A4 = 595.0
  const TOLERANCE = 5.0

  // abs est la clé des comparaisons de dimensions avec tolérance
  doc.pages.each { |page|
    assert abs(page.width - LARGEUR_A4) < TOLERANCE,
      "page #{page.number} is outside A4: #{page.width}pt"
  }

  // round rend les messages lisibles
  // Sans : "217.4453125 DPI". Avec : "217 DPI".
  doc.images.each { |img|
    assert img.dpi >= 300,
      "image on page #{img.page_number}: #{round(img.dpi)} DPI"
  }

  print("document:", doc.filename)
  print("pages:", doc.page_count, "| fonts:", doc.fonts.length)
}
```

---

## 10.4 Tournures courantes

```pdfl
// Compter combien d'éléments ne passent pas
check "Problem count" {
  mauvaises = doc.images.filter { |i| i.dpi < 300 }
  assert mauvaises.length == 0,
    "#{mauvaises.length} of #{doc.images.length} images below 300 DPI"
}

// Lister les éléments fautifs dans le message
check "List in the message" {
  // Enchaînement sur la même ligne : pas de retour avant le point
  problemes = doc.pages.filter { |p| !p.has_trim_box }.map { |p| p.number }
  assert problemes.length == 0,
    "pages without a TrimBox: #{problemes.join(", ")}"
}

// Validation avec tolérance
function proche_de(valeur, cible, tolerance) {
  abs(valeur - cible) < tolerance
}

check "With tolerance" {
  doc.pages.each { |page|
    assert proche_de(page.width, 595.0, 2.0),
      "page #{page.number}: width #{page.width}pt (expected 595 ± 2)"
  }
}

// Ne pas planter sur un document vide
check "Defensive" {
  // Le court-circuit évite d'appeler first() sur une liste vide
  assert doc.pages.length == 0 || doc.pages.first().width > 0,
    "the first page has no width"
}
```

---

[← `data::`](09-data.md) · [Sommaire](README.md) · [Suivant : ligne de commande →](11-cli.md)
