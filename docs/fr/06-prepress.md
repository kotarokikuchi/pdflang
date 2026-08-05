# 6. Espace de noms `prepress::` — prépresse

[← `visual::`](05-visual.md) · [Sommaire](README.md) · [Suivant : `codes::` →](07-codes.md)

30 fonctions qui couvrent ce qu'une imprimerie doit vérifier avant de graver les
plaques : encrage total, séparations, polices, filets, boîtes de page.

---

## 6.1 Encrage total (TAC)

Le TAC (Total Area Coverage) est la somme des quatre encres en un point donné.
Au-delà de la limite de la presse : maculage, séchage difficile, décalque. En
offset sur papier couché, la limite usuelle est 300 %.

Il existe **deux** façons de le mesurer, et la différence compte.

| Fonction | Rôle |
|---|---|
| `prepress::calculate_exact_tac([page])` | Calcul d'après les **couleurs déclarées** dans le fichier (exact) |
| `prepress::calculate_tac([page])` | Estimation par rendu RVB (**borne basse**) |
| `prepress::validate_tac_limits([limite])` | Vrai si toutes les pages sont sous la limite (300 par défaut) |
| `prepress::calculate_ink_coverage([page])` | Encrage moyen (%) |
| `prepress::calculate_tac_by_region(page, region)` | `[TAC max, moyenne]` de la zone |

L'estimation écrase les noirs riches vers 100 %.

```pdfl
check "Ink limit" {
  // Pour valider une limite, utilisez toujours le TAC exact
  doc.pages.each { |page|
    tac = prepress::calculate_exact_tac(page.number)
    assert tac <= 300, "page #{page.number}: #{tac}% ink"
  }

  print("exact (declared in the file):", prepress::calculate_exact_tac(), "%")
  print("estimated (by rendering):", prepress::calculate_tac(), "%")
  // Mesuré sur un fichier réel : exact 324 %, estimé 299 %
  // — seul l'exact révèle le dépassement.

  // Trop d'encre sur le pli craque au façonnage
  pli = region(290, 0, 15, 842, "center fold")
  mesure = prepress::calculate_tac_by_region(1, pli)
  assert mesure.first() < 240, "TAC of #{mesure.first()}% on the fold (max 240%)"
}
```

---

## 6.2 Couleurs et séparations

| Fonction | Rôle |
|---|---|
| `prepress::detect_spot_colors()` | Liste des tons directs (Separation / DeviceN) |
| `prepress::detect_color_mode()` | `"CMYK"`, `"RGB"`, `"Mixed"`, `"None"` ou `"Other"` |
| `prepress::validate_color_space(espace)` | Vrai si toutes les images sont dans cet espace |
| `prepress::compare_colors_delta_e(a, b)` | Delta-E (CIE76) entre deux couleurs |
| `prepress::detect_rich_black()` | Vrai s'il existe un noir composé de plusieurs encres |
| `prepress::validate_overprint_settings()` | Vrai si la surimpression n'est pas activée |
| `prepress::validate_output_intent([nom])` | Y a-t-il une intention de sortie / correspond-elle ? |
| `prepress::check_rendering_intent([attendu])` | Liste ou valide l'intention de rendu |

Les couleurs se passent en listes : 4 valeurs = CMJN, 3 = RVB, 1 = gris. Repères
de Delta-E : moins de 1 imperceptible, jusqu'à 3 acceptable en impression,
au-delà de 5 nettement différent.

> Les séparations réservées `All` et `None` ne sont pas listées : `All` sert aux
> repères, ce n'est pas une encre.

```pdfl
check "Colors" {
  tons = prepress::detect_spot_colors()
  assert tons.length == 0, "file uses an unquoted special ink: #{tons.join(", ")}"

  mode = prepress::detect_color_mode()
  assert mode == "CMYK" || mode == "None",
    "document is #{mode} — offset printing requires CMYK"

  // Tolérance sur une couleur de marque
  ecart = prepress::compare_colors_delta_e([1.0, 0.6, 0.0, 0.1], [1.0, 0.62, 0.0, 0.12])
  assert ecart < 3.0, "brand color out of tolerance (ΔE #{ecart})"

  // Le noir riche sur du petit texte rend le repérage plus visible
  assert !prepress::detect_rich_black(),
    "rich black found — use flat black (0/0/0/100) for text"

  // Une surimpression involontaire fait disparaître des éléments
  assert prepress::validate_overprint_settings(),
    "overprint is enabled — verify that it is intentional"

  assert prepress::validate_output_intent(),
    "PDF has no Output Intent — the shop cannot know the target color profile"
}
```

---

## 6.3 Épaisseur des filets

| Fonction | Rôle |
|---|---|
| `prepress::detect_hairlines([limite])` | Vrai s'il existe un filet sous le seuil (0,25 pt par défaut) |
| `prepress::detect_hairlines_exact()` | Vrai s'il existe un filet d'épaisseur 0 |
| `prepress::detect_fine_lines([limite])` | Idem (1 pt par défaut) |
| `prepress::validate_minimum_stroke_width(min)` | Vrai si tous les filets atteignent le minimum |

L'épaisseur 0 est le filet capillaire classique de PostScript : le périphérique
le rend à sa plus petite largeur possible, donc imprévisible.

```pdfl
check "Strokes" {
  assert !prepress::detect_hairlines(0.25),
    "there are strokes below 0.25 pt — they will disappear in print"
  assert !prepress::detect_hairlines_exact(),
    "there is a stroke with 0 width — set a real thickness"
  assert prepress::validate_minimum_stroke_width(0.5),
    "the shop contract requires strokes of at least 0.5 pt"
}
```

---

## 6.4 Polices

| Fonction | Rôle |
|---|---|
| `prepress::list_fonts()` | Noms des polices employées |
| `prepress::validate_font_embedding()` | Vrai si toutes sont incorporées |
| `prepress::detect_text_substitution()` | Liste des polices non incorporées |
| `prepress::detect_missing_glyphs()` | Polices sans table de chasses |
| `prepress::subset_fonts()` | Vrai si toutes les polices incorporées sont des sous-ensembles |
| `prepress::check_font_licensing()` | Polices à risque de licence (Type3 ou non incorporées) |
| `prepress::validate_font_size([min])` | Vrai s'il n'y a pas de texte sous la taille minimale (6 pt par défaut) |

```pdfl
check "Fonts" {
  print("fonts:", prepress::list_fonts().join(", "))

  manquantes = prepress::detect_text_substitution()
  assert manquantes.length == 0,
    "fonts not embedded (text will change at the RIP): #{manquantes.join(", ")}"

  problemes = prepress::detect_missing_glyphs()
  assert problemes.length == 0,
    "fonts without a widths table: #{problemes.join(", ")}"

  assert prepress::subset_fonts(),
    "a full font is embedded — the file is larger than it needs to be"

  risque = prepress::check_font_licensing()
  assert risque.length == 0, "fonts with licensing risk: #{risque.join(", ")}"

  // Notices et contrats ont une taille minimale réglementaire
  assert prepress::validate_font_size(6),
    "there is text below 6 pt — illegible once printed"
}
```

---

## 6.5 Pages et boîtes

Les boîtes du PDF définissent les zones de travail : **MediaBox** (le papier),
**BleedBox** (le fond perdu), **TrimBox** (le format fini), **CropBox**
(l'affichage), **ArtBox** (le contenu).

| Fonction | Rôle |
|---|---|
| `prepress::get_page_size([page])` | `[largeur, hauteur]` en points |
| `prepress::get_page_boxes([page])` | Liste des boîtes définies |
| `prepress::validate_media_box()` | Vrai si toutes les pages ont une MediaBox |
| `prepress::validate_trim_box()` | Vrai si toutes ont une TrimBox |
| `prepress::validate_bleed_box()` | Vrai si toutes ont une BleedBox |
| `prepress::check_page_geometry([marge])` | Vrai si le fond perdu atteint la valeur sur les quatre côtés (3 mm par défaut) |

```pdfl
check "Geometry" {
  taille = prepress::get_page_size(1)
  assert abs(taille.first() - 595.0) < 5, "width is outside A4"
  prepress::get_page_boxes(1).each { |boite| print(boite) }

  assert prepress::validate_trim_box(),
    "no TrimBox — the shop cannot know where to trim"
  assert prepress::validate_bleed_box(),
    "no BleedBox — no bleed area is defined"

  // Le littéral d'unité se lit bien et convertit tout seul
  assert prepress::check_page_geometry(3mm),
    "bleed smaller than 3 mm on some page"
}
```

---

## 6.6 Exemple complet

```pdfl
// magazine_offset.pdfl — contrôle prépresse complet pour l'offset
// Usage : pdfl run magazine_offset.pdfl magazine.pdf --output html --output-file rapport.html
profile "offset-magazine" {

  const LIMITE_TAC = 300%
  const FOND_PERDU = 3mm
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress", "colors"] {
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= LIMITE_TAC,
        "page #{page.number}: #{tac}% ink (limit #{LIMITE_TAC}%)"
    }
    print("average coverage:", prepress::calculate_ink_coverage(), "%")
  }

  check "Colors" tags: ["prepress", "colors"] {
    assert prepress::detect_color_mode() != "RGB", "document is in RGB"
    tons = prepress::detect_spot_colors()
    assert tons.length == 0, "unquoted special ink: #{tons.join(", ")}"
    assert !prepress::detect_rich_black(), "rich black in text"
    assert prepress::validate_output_intent(), "no Output Intent"
  }

  check "Fonts" tags: ["fonts"] {
    manquantes = prepress::detect_text_substitution()
    assert manquantes.length == 0, "fonts not embedded: #{manquantes.join(", ")}"
    assert prepress::validate_font_size(6), "text below 6 pt"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25), "strokes below 0.25 pt"
    assert !prepress::detect_hairlines_exact(), "stroke with 0 width"
  }

  check "Geometry" tags: ["prepress", "boxes"] {
    require prepress::validate_trim_box()
    require prepress::validate_bleed_box()
    assert prepress::check_page_geometry(FOND_PERDU), "bleed smaller than 3 mm"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI"
      assert img.color_space != "DeviceRGB", "RGB image on page #{img.page_number}"
    }
  }
}
```

---

[← `visual::`](05-visual.md) · [Sommaire](README.md) · [Suivant : `codes::` →](07-codes.md)
