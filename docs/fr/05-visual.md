# 5. Espace de noms `visual::` — images et comparaison visuelle

[← `struct::`](04-struct.md) · [Sommaire](README.md) · [Suivant : `prepress::` →](06-prepress.md)

16 fonctions sur les images du document et sur l'aspect rendu des pages.

> Les fonctions de comparaison et de qualité **rendent la page en niveaux de
> gris**. Chaque page n'est rendue qu'une fois, puis mise en cache.

---

## 5.1 Inventaire des images

| Fonction | Rôle |
|---|---|
| `visual::detect_images()` | Vrai s'il y a des images |
| `visual::count_images()` | Nombre total d'images |
| `visual::get_image_resolution(n)` | DPI effectif de la n-ième image (à partir de 1) |
| `visual::get_image_size(n)` | Dimensions en pixels `[largeur, hauteur]` |
| `visual::detect_image_color_space([n])` | Liste des espaces, ou celui de la n-ième image |
| `visual::detect_low_resolution([dpi_min])` | Vrai s'il existe une image sous le seuil (300 par défaut) |

```pdfl
check "Image inventory" {
  require visual::detect_images()
  print("total images:", visual::count_images())
  print("spaces present:", visual::detect_image_color_space().join(", "))

  // L'offset exige du CMJN partout
  assert !visual::detect_image_color_space().contains("DeviceRGB"),
    "there are RGB images — convert to CMYK before printing"

  assert !visual::detect_low_resolution(300),
    "there are images below 300 DPI"
}
```

> Pour savoir **lesquelles** posent problème, parcourez `doc.images` — voir le
> [chapitre 2](02-types.md) :
>
> ```pdfl
> doc.images.each { |img|
>   assert img.dpi >= 300, "image on page #{img.page_number}: #{img.dpi} DPI"
> }
> ```

---

## 5.2 Comparaison visuelle entre fichiers

Ces fonctions comparent une page de ce document à une page d'un **autre
fichier**. Signature commune :

```
fonction(page_ici, "autre.pdf" [, page_là])
```

Sans le numéro de l'autre page, la même page est utilisée. Des pages de tailles
différentes sont rééchantillonnées avant comparaison.

| Fonction | Rôle |
|---|---|
| `visual::measure_ssim(page, "autre.pdf" [, page_b])` | Similarité structurelle (0.0 à 1.0) |
| `visual::compare_images(...)` / `visual::diff_pages(...)` | Même comparaison, sur 0 à 100 |
| `visual::pixel_diff(page, "autre.pdf" [, page_b, tolérance])` | Pourcentage de pixels différents |
| `visual::calculate_perceptual_hash([page])` | pHash 64 bits (hexadécimal) |
| `visual::detect_image_replacement(page, "autre.pdf" [, page_b, distance])` | Vrai si le changement dépasse la tolérance |

```pdfl
check "Approved proof vs final file" {
  approuve = "approved/magazine_v1.pdf"

  doc.pages.each { |page|
    ssim = visual::measure_ssim(page.number, approuve)
    assert ssim > 0.99,
      "page #{page.number} changed visually (SSIM #{ssim})"

    // Une tolérance élevée ignore les écarts d'anticrénelage
    doux = visual::pixel_diff(page.number, approuve, page.number, 30)
    assert doux < 1.0, "significant change on page #{page.number}"

    assert !visual::detect_image_replacement(page.number, approuve),
      "page #{page.number}: visual content was replaced"
  }
}
```

---

## 5.3 Qualité des images

| Fonction | Rôle |
|---|---|
| `visual::detect_image_artifacts([page])` | Vrai s'il y a des blocs JPEG visibles |
| `visual::estimate_image_quality([page])` | Note de 0 à 100 déduite de l'effet de bloc |
| `visual::detect_posterization([page])` | Vrai si les niveaux de tons sont trop peu nombreux |
| `visual::detect_banding([page])` | Vrai si un dégradé montre des marches |

> La détection de banding exige une progression monotone avec de larges
> plateaux : une page de texte, très contrastée, **ne déclenche pas** de fausse
> alerte.

```pdfl
check "Image quality" {
  doc.pages.each { |page|
    assert !visual::detect_image_artifacts(page.number),
      "page #{page.number} shows visible compression blockiness"

    note = visual::estimate_image_quality(page.number)
    assert note >= 70,
      "page #{page.number} scores #{note}/100 — recompressed too hard?"

    assert !visual::detect_posterization(page.number),
      "page #{page.number}: possible posterization (too few tones)"
    assert !visual::detect_banding(page.number),
      "page #{page.number} shows banding in a gradient"
  }
}
```

---

## 5.4 Exemple complet

```pdfl
// approbation_visuelle.pdfl — comparaison avec la version approuvée
// Usage : pdfl run approbation_visuelle.pdfl nouvelle_version.pdf
profile "visual-approval" {

  const APPROUVE = "approved/catalogue_v1.pdf"
  const MIN_DPI = 300

  check "Inventory" tags: ["images"] {
    require visual::detect_images()
    print("images:", visual::count_images())
    print("color spaces:", visual::detect_image_color_space().join(", "))
  }

  check "Resolution" tags: ["images", "prepress"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI (minimum #{MIN_DPI})"
    }
  }

  check "Quality" tags: ["images"] {
    doc.pages.each { |page|
      assert !visual::detect_image_artifacts(page.number),
        "page #{page.number} has compression artifacts"
      assert !visual::detect_banding(page.number), "page #{page.number} shows banding"
    }
  }

  check "Fidelity to the approved version" tags: ["approval"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APPROUVE)
      assert ssim > 0.99,
        "page #{page.number} differs from the approved one (SSIM #{ssim}, #{visual::pixel_diff(page.number, APPROUVE)}% of pixels)"
    }
  }
}
```

---

[← `struct::`](04-struct.md) · [Sommaire](README.md) · [Suivant : `prepress::` →](06-prepress.md)
