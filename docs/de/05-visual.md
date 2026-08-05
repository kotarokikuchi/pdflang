# 5. Namensraum `visual::` — Bilder und visueller Vergleich

[← `struct::`](04-struct.md) · [Inhalt](README.md) · [Weiter: `prepress::` →](06-prepress.md)

16 Funktionen zu den Bildern des Dokuments und zum gerenderten Aussehen der
Seiten.

> Die Vergleichs- und Qualitätsfunktionen **rendern die Seite in Graustufen**.
> Jede Seite wird nur einmal gerendert und dann zwischengespeichert.

---

## 5.1 Bildbestand

| Funktion | Zweck |
|---|---|
| `visual::detect_images()` | Wahr, wenn es Bilder gibt |
| `visual::count_images()` | Gesamtzahl der Bilder |
| `visual::get_image_resolution(n)` | Effektive DPI des n-ten Bildes (ab 1) |
| `visual::get_image_size(n)` | Maße in Pixeln `[Breite, Höhe]` |
| `visual::detect_image_color_space([n])` | Liste der Farbräume oder der des n-ten Bildes |
| `visual::detect_low_resolution([min_dpi])` | Wahr, wenn ein Bild unter der Schwelle liegt (Vorgabe 300) |

```pdfl
check "Image inventory" {
  require visual::detect_images()
  print("total images:", visual::count_images())
  print("spaces present:", visual::detect_image_color_space().join(", "))

  // Offset verlangt durchgehend CMYK
  assert !visual::detect_image_color_space().contains("DeviceRGB"),
    "there are RGB images — convert to CMYK before printing"

  assert !visual::detect_low_resolution(300),
    "there are images below 300 DPI"
}
```

> Um zu erfahren, **welche** Bilder das Problem sind, gehen Sie `doc.images`
> durch — siehe [Kapitel 2](02-types.md):
>
> ```pdfl
> doc.images.each { |img|
>   assert img.dpi >= 300, "image on page #{img.page_number}: #{img.dpi} DPI"
> }
> ```

---

## 5.2 Visueller Vergleich zwischen Dateien

Diese Funktionen vergleichen eine Seite dieses Dokuments mit einer Seite einer
**anderen Datei**. Gemeinsame Signatur:

```
funktion(seite_hier, "andere.pdf" [, seite_dort])
```

Ohne die Nummer der anderen Seite wird dieselbe Seite verwendet. Unterschiedlich
große Seiten werden vor dem Vergleich neu abgetastet.

| Funktion | Zweck |
|---|---|
| `visual::measure_ssim(seite, "andere.pdf" [, seite_b])` | Strukturelle Ähnlichkeit (0.0 bis 1.0) |
| `visual::compare_images(...)` / `visual::diff_pages(...)` | Derselbe Vergleich, auf 0 bis 100 |
| `visual::pixel_diff(seite, "andere.pdf" [, seite_b, toleranz])` | Anteil abweichender Pixel |
| `visual::calculate_perceptual_hash([seite])` | 64-Bit-pHash (hexadezimal) |
| `visual::detect_image_replacement(seite, "andere.pdf" [, seite_b, abstand])` | Wahr, wenn die Änderung die Toleranz übersteigt |

```pdfl
check "Approved proof vs final file" {
  freigegeben = "approved/magazine_v1.pdf"

  doc.pages.each { |page|
    ssim = visual::measure_ssim(page.number, freigegeben)
    assert ssim > 0.99,
      "page #{page.number} changed visually (SSIM #{ssim})"

    // Eine höhere Toleranz übergeht Unterschiede der Kantenglättung
    weich = visual::pixel_diff(page.number, freigegeben, page.number, 30)
    assert weich < 1.0, "significant change on page #{page.number}"

    assert !visual::detect_image_replacement(page.number, freigegeben),
      "page #{page.number}: visual content was replaced"
  }
}
```

---

## 5.3 Bildqualität

| Funktion | Zweck |
|---|---|
| `visual::detect_image_artifacts([seite])` | Wahr bei sichtbaren JPEG-Blöcken |
| `visual::estimate_image_quality([seite])` | Note von 0 bis 100 aus der Blockbildung |
| `visual::detect_posterization([seite])` | Wahr, wenn zu wenige Tonwertstufen da sind |
| `visual::detect_banding([seite])` | Wahr, wenn ein Verlauf Stufen zeigt |

> Die Banding-Erkennung verlangt einen monotonen Verlauf mit breiten Plateaus:
> Eine kontrastreiche Textseite löst **keinen** Fehlalarm aus.

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

## 5.4 Vollständiges Beispiel

```pdfl
// visuelle_freigabe.pdfl — Vergleich mit der freigegebenen Fassung
// Aufruf: pdfl run visuelle_freigabe.pdfl neue_fassung.pdf
profile "visual-approval" {

  const FREIGEGEBEN = "approved/catalogue_v1.pdf"
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
      ssim = visual::measure_ssim(page.number, FREIGEGEBEN)
      assert ssim > 0.99,
        "page #{page.number} differs from the approved one (SSIM #{ssim}, #{visual::pixel_diff(page.number, FREIGEGEBEN)}% of pixels)"
    }
  }
}
```

---

[← `struct::`](04-struct.md) · [Inhalt](README.md) · [Weiter: `prepress::` →](06-prepress.md)
