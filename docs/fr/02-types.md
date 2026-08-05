# 2. Types du document

[← Le langage](01-language.md) · [Sommaire](README.md) · [Suivant : `text::` →](03-text.md)

Tout script reçoit automatiquement la variable `doc`, qui représente le PDF
analysé. À partir d'elle, on atteint les pages, les polices et les images.

---

## 2.1 `doc` — le document

| Propriété | Type | Contenu |
|---|---|---|
| `doc.page_count` | nombre | Nombre de pages |
| `doc.title` | texte | Titre des métadonnées (vide s'il manque) |
| `doc.author` | texte | Auteur des métadonnées (vide s'il manque) |
| `doc.filename` | texte | Nom du fichier analysé |
| `doc.pages` | liste | Toutes les pages |
| `doc.fonts` | liste | Toutes les polices employées |
| `doc.images` | liste | Toutes les images, toutes pages confondues |

Méthode : `doc.extract_text()` — le texte de tout le document, pages séparées
par des sauts de ligne.

```pdfl
check "Document properties" {
  print("file:", doc.filename)
  print("pages:", doc.page_count)

  // Ces collections sont des listes ordinaires — toutes les méthodes marchent
  require doc.pages.length == doc.page_count
  require doc.fonts.length > 0

  texte = doc.extract_text()
  assert texte.trim() != "", "PDF has no extractable text (images only?)"
  print("total characters:", texte.length)
}
```

---

## 2.2 `page` — la page

Les pages viennent de `doc.pages` (dans un bloc) ou de la variable `page` (dans
une `rule`).

| Propriété | Type | Contenu |
|---|---|---|
| `page.number` | nombre | Numéro de page, à partir de **1** |
| `page.index` | nombre | Indice, à partir de **0** |
| `page.width` / `page.height` | nombre | Largeur / hauteur en points |
| `page.images` | liste | Images de cette page |
| `page.tac` | nombre | Encrage total maximal estimé (%) |
| `page.ink_coverage` | nombre | Encrage moyen estimé (%) |
| `page.min_stroke_width` | nombre/null | Filet le plus fin (pt) ; `null` s'il n'y a aucun trait |
| `page.has_media_box` etc. | booléen | `has_crop_box`, `has_trim_box`, `has_bleed_box`, `has_art_box` |

Méthode : `page.extract_text()` — le texte de cette page seulement.

```pdfl
check "Page format" {
  doc.pages.each { |page|
    // number est le numéro que lisent les gens, index sert aux calculs internes
    assert page.width > 100mm,
      "page #{page.number} is too narrow: #{page.width}pt"

    // Les boîtes : indispensables pour l'impression
    assert page.has_trim_box, "page #{page.number} has no TrimBox (trim area)"
    assert page.has_bleed_box, "page #{page.number} has no BleedBox (bleed area)"

    assert page.tac <= 300,
      "page #{page.number}: #{page.tac}% ink (limit 300%)"

    // min_stroke_width peut être null (aucun trait sur la page).
    // null est faux, donc ceci est sûr :
    assert !page.min_stroke_width || page.min_stroke_width >= 0.25,
      "page #{page.number} has a hairline stroke"
  }
}

check "Blank pages" {
  vides = doc.pages.filter { |p| p.extract_text().trim() == "" }
  assert vides.length == 0,
    "#{vides.length} blank page(s): #{vides.map { |p| p.number }.join(", ")}"
}
```

---

## 2.3 `font` — la police

Vient de `doc.fonts`. Propriétés : `font.name` (nom) et `font.is_embedded`
(incorporée ou non).

```pdfl
check "Embedded fonts" {
  // Une police non incorporée est remplacée par le lecteur — le texte change
  doc.fonts.each { |font|
    assert font.is_embedded,
      "font '#{font.name}' is not embedded in the PDF"
  }
  print("fonts in use:", doc.fonts.map { |f| f.name }.join(", "))
}
```

---

## 2.4 `image` — l'image

Vient de `doc.images` (toutes) ou de `page.images` (celles d'une page).

| Propriété | Contenu |
|---|---|
| `image.width` / `image.height` | Largeur / hauteur en **pixels** |
| `image.dpi` | Résolution effective (la plus petite de dpi_x et dpi_y) |
| `image.dpi_x` / `image.dpi_y` | Résolution effective horizontale / verticale |
| `image.color_space` | `DeviceRGB`, `DeviceCMYK`, `Indexed`… |
| `image.page_number` | Page où elle se trouve (à partir de 1) |
| `image.bits_per_pixel` | Profondeur de bits |

> **Le DPI est effectif**, calculé comme « pixels ÷ taille imprimée sur la
> page », et non la valeur nominale des métadonnées. C'est ce chiffre-là qui
> décide de la qualité d'impression : une image de 1000 px étirée sur 20 cm a un
> DPI bas, quoi que disent ses métadonnées.

```pdfl
profile "images-for-offset" {
  const MIN_DPI = 300

  check "Resolution" {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image #{img.width}x#{img.height}px on page #{img.page_number}: #{img.dpi} DPI (minimum #{MIN_DPI})"
    }
  }

  check "Color space" {
    // L'offset travaille en CMJN ; le RVB doit être converti
    doc.images.each { |img|
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number} — convert to CMYK"
    }
  }

  check "Images per page" {
    doc.pages.each { |page|
      print("page", page.number, "has", page.images.length, "image(s)")
    }
  }
}
```

---

## 2.5 `region` — une zone de la page

Une région délimite une partie de la page par un rectangle. Elle sert à valider
un pied de page, un en-tête, l'emplacement d'un code-barres, un bandeau
réglementaire.

Création : `region(x, y, largeur, hauteur [, "nom"])`, l'origine (0,0) étant en
bas à gauche, comme dans le PDF.

| Propriété | Contenu | | Méthode | Rôle |
|---|---|---|---|---|
| `region.name` | Nom donné à la création | | `contains_point(x, y)` | Le point est-il dedans ? |
| `region.x` / `region.y` | Coin inférieur gauche | | `intersects(autre)` | Les deux régions se recouvrent-elles ? |
| `region.width` / `region.height` | Dimensions | | `expand(pt)` | Nouvelle région élargie de chaque côté |
| `region.right` / `region.top` | Bord droit / haut (calculés) | | `inset(pt)` | Nouvelle région rétrécie de chaque côté |
| `region.area` | Surface (points carrés) | | `export_coordinates()` | `[x0, y0, x1, y1]` |

```pdfl
check "Working with regions" {
  pied = region(0, 0, 595, 60, "footer")

  require pied.name == "footer"
  require pied.top == 60.0
  require pied.right == 595.0
  require pied.area == 35700.0
  require pied.contains_point(300, 30)
  require !pied.contains_point(300, 500)

  // Détection de recouvrement : utile pour repérer un élément
  // qui empiète sur une zone réservée
  entete = region(0, 780, 595, 62)
  require !pied.intersects(entete)

  // expand/inset renvoient une NOUVELLE région (l'originale ne change pas)
  require pied.expand(5mm).area > pied.area
  require pied.inset(3mm).area < pied.area
}

profile "medicine-label" {
  check "Prescription band" {
    // Le bandeau doit être en haut et porter la mention légale
    bandeau = region(0, 700, 595, 142, "band")
    assert text::extract_from_region(1, bandeau).contains("PRESCRIPTION ONLY"),
      "band is missing the mandatory text"
  }

  check "Ink in the fold area" {
    // Trop d'encre sur le pli craque au façonnage
    pli = region(290, 0, 15, 842, "center fold")
    mesure = prepress::calculate_tac_by_region(1, pli)
    assert mesure.first() < 240,
      "too much ink on the fold: #{mesure.first()}%"
  }

  check "Barcode in the right place" {
    zone_code = region(400, 20, 180, 80, "barcode area")
    assert codes::validate_barcode_position(zone_code),
      "barcode outside the reserved area"
  }
}
```

---

[← Le langage](01-language.md) · [Sommaire](README.md) · [Suivant : `text::` →](03-text.md)
