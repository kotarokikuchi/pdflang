# 5. فضاء الأسماء `visual::` — الصور والمقارنة البصرية

[← `struct::`](04-struct.md) · [الفهرس](README.md) · [التالي: `prepress::` →](06-prepress.md)

16 دالة تخصّ صور المستند وشكل الصفحات بعد التصيير.

> دوال المقارنة والجودة **تصيّر الصفحة بتدرّج الرمادي**. وكل صفحة تُصيَّر مرة
> واحدة ثم تُخزَّن مؤقتًا.

---

## 5.1 جرد الصور

| الدالة | الغرض |
|---|---|
| `visual::detect_images()` | صحيحة إن وُجدت صور |
| `visual::count_images()` | العدد الكلي للصور |
| `visual::get_image_resolution(n)` | الدقة الفعلية للصورة رقم n (ابتداءً من 1) |
| `visual::get_image_size(n)` | الأبعاد بالبكسل `[العرض, الارتفاع]` |
| `visual::detect_image_color_space([n])` | قائمة الفضاءات اللونية أو فضاء الصورة رقم n |
| `visual::detect_low_resolution([min_dpi])` | صحيحة إن وُجدت صورة دون الحدّ (300 افتراضًا) |

```pdfl
check "Image inventory" {
  require visual::detect_images()
  print("total images:", visual::count_images())
  print("spaces present:", visual::detect_image_color_space().join(", "))

  // الأوفست يقتضي CMYK في كل شيء
  assert !visual::detect_image_color_space().contains("DeviceRGB"),
    "there are RGB images — convert to CMYK before printing"

  assert !visual::detect_low_resolution(300),
    "there are images below 300 DPI"
}
```

> لمعرفة **أي** الصور فيها المشكلة، امشِ على `doc.images` — انظر
> [الفصل 2](02-types.md):
>
> ```pdfl
> doc.images.each { |img|
>   assert img.dpi >= 300, "image on page #{img.page_number}: #{img.dpi} DPI"
> }
> ```

---

## 5.2 المقارنة البصرية بين الملفات

تقارن هذه الدوال صفحةً من هذا المستند بصفحةٍ من **ملف آخر**. والتوقيع المشترك:

```
function(page_here, "other.pdf" [, page_there])
```

وإن أُهمل رقم الصفحة الأخرى استُعملت الصفحة نفسها. والصفحات المختلفة الأحجام
تُعاد معاينتها قبل المقارنة.

| الدالة | الغرض |
|---|---|
| `visual::measure_ssim(page, "other.pdf" [, page_b])` | التشابه البنيوي (0.0 إلى 1.0) |
| `visual::compare_images(...)` / `visual::diff_pages(...)` | المقارنة نفسها على مقياس 0 إلى 100 |
| `visual::pixel_diff(page, "other.pdf" [, page_b, tolerance])` | نسبة البكسلات المختلفة |
| `visual::calculate_perceptual_hash([page])` | pHash بطول 64 بت (بالنظام الست عشري) |
| `visual::detect_image_replacement(page, "other.pdf" [, page_b, distance])` | صحيحة إن تجاوز التغيّر التسامح |

```pdfl
check "Approved proof vs final file" {
  approved = "approved/magazine_v1.pdf"

  doc.pages.each { |page|
    ssim = visual::measure_ssim(page.number, approved)
    assert ssim > 0.99,
      "page #{page.number} changed visually (SSIM #{ssim})"

    // رفع التسامح يتجاوز فروق تنعيم الحواف
    smooth = visual::pixel_diff(page.number, approved, page.number, 30)
    assert smooth < 1.0, "significant change on page #{page.number}"

    assert !visual::detect_image_replacement(page.number, approved),
      "page #{page.number}: visual content was replaced"
  }
}
```

---

## 5.3 جودة الصور

| الدالة | الغرض |
|---|---|
| `visual::detect_image_artifacts([page])` | صحيحة عند ظهور مربعات JPEG |
| `visual::estimate_image_quality([page])` | درجة من 0 إلى 100 مستنتجة من التربيع |
| `visual::detect_posterization([page])` | صحيحة إذا قلّت درجات التدرّج |
| `visual::detect_banding([page])` | صحيحة إذا ظهرت درجات في تدرّج لوني |

> كشف التخطيط يشترط تدرّجًا رتيبًا بمسطّحات عريضة، لذا لا تُطلق صفحة نصية
> عالية التباين إنذارًا كاذبًا.

```pdfl
check "Image quality" {
  doc.pages.each { |page|
    assert !visual::detect_image_artifacts(page.number),
      "page #{page.number} shows visible compression blockiness"

    score = visual::estimate_image_quality(page.number)
    assert score >= 70,
      "page #{page.number} scores #{score}/100 — recompressed too hard?"

    assert !visual::detect_posterization(page.number),
      "page #{page.number}: possible posterization (too few tones)"
    assert !visual::detect_banding(page.number),
      "page #{page.number} shows banding in a gradient"
  }
}
```

---

## 5.4 مثال كامل

```pdfl
// visual_approval.pdfl — المقارنة بالنسخة المعتمدة
// الاستعمال: pdfl run visual_approval.pdfl new_version.pdf
profile "visual-approval" {

  const APPROVED = "approved/catalogue_v1.pdf"
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
      ssim = visual::measure_ssim(page.number, APPROVED)
      assert ssim > 0.99,
        "page #{page.number} differs from the approved one (SSIM #{ssim}, #{visual::pixel_diff(page.number, APPROVED)}% of pixels)"
    }
  }
}
```

---

[← `struct::`](04-struct.md) · [الفهرس](README.md) · [التالي: `prepress::` →](06-prepress.md)
