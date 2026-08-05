# 6. فضاء الأسماء `prepress::` — ما قبل الطبع

[← `visual::`](05-visual.md) · [الفهرس](README.md) · [التالي: `codes::` →](07-codes.md)

30 دالة تغطي ما يجب على المطبعة التحقق منه قبل حرق الألواح: تغطية الحبر،
والفصل اللوني، والخطوط، وسماكات الخطوط، وأُطر الصفحة.

---

## 6.1 تغطية الحبر الكلية (TAC)

الـ TAC (Total Area Coverage) مجموع الأحبار الأربعة عند نقطة واحدة. وتجاوز حدّ
الآلة يعني التلطيخ وسوء الجفاف وانتقال الحبر. وفي الأوفست على الورق المطلي
الحدّ المعتاد 300 %.

وثمة **طريقتان** لقياسه، والفارق بينهما مهم.

| الدالة | الغرض |
|---|---|
| `prepress::calculate_exact_tac([page])` | حساب من **الألوان المعلَنة** في الملف (دقيق) |
| `prepress::calculate_tac([page])` | تقدير عبر تصيير RGB (**حدّ أدنى**) |
| `prepress::validate_tac_limits([limit])` | صحيحة إن بقيت كل الصفحات دون الحدّ (300 افتراضًا) |
| `prepress::calculate_ink_coverage([page])` | متوسط تغطية الحبر (%) |
| `prepress::calculate_tac_by_region(page, region)` | `[أقصى TAC, المتوسط]` للمنطقة |

التقدير يضغط الأسود الغني نحو 100 %.

```pdfl
check "Ink limit" {
  // لتدقيق حدّ ما استعمل دائمًا الـ TAC الدقيق
  doc.pages.each { |page|
    tac = prepress::calculate_exact_tac(page.number)
    assert tac <= 300, "page #{page.number}: #{tac}% ink"
  }

  print("exact (declared in the file):", prepress::calculate_exact_tac(), "%")
  print("estimated (by rendering):", prepress::calculate_tac(), "%")
  // قياس على ملف حقيقي: الدقيق 324 % والمقدَّر 299 %
  // — الدقيق وحده يكشف التجاوز.

  // كثرة الحبر على الطية تتشقق في التشطيب
  fold = region(290, 0, 15, 842, "center fold")
  measured = prepress::calculate_tac_by_region(1, fold)
  assert measured.first() < 240, "TAC of #{measured.first()}% on the fold (max 240%)"
}
```

---

## 6.2 الألوان والفصل اللوني

| الدالة | الغرض |
|---|---|
| `prepress::detect_spot_colors()` | قائمة الألوان الخاصة (Separation / DeviceN) |
| `prepress::detect_color_mode()` | `"CMYK"` أو `"RGB"` أو `"Mixed"` أو `"None"` أو `"Other"` |
| `prepress::validate_color_space(space)` | صحيحة إن كانت كل الصور في هذا الفضاء |
| `prepress::compare_colors_delta_e(a, b)` | دلتا-E (CIE76) بين لونين |
| `prepress::detect_rich_black()` | صحيحة إن وُجد أسود مركّب من عدة أحبار |
| `prepress::validate_overprint_settings()` | صحيحة إن لم تكن الطباعة الفوقية مفعَّلة |
| `prepress::validate_output_intent([name])` | هل ثمة نيّة إخراج / هل يطابق الاسم؟ |
| `prepress::check_rendering_intent([expected])` | تسرد نيّة التصيير أو تدقّقها |

تُمرَّر الألوان قوائمَ: 4 قيم = CMYK، و3 = RGB، وواحدة = رمادي. ومؤشرات
دلتا-E: أقل من 1 غير محسوس، وحتى 3 مقبول في الطباعة، وفوق 5 مختلف بوضوح.

> الفصلان المحجوزان `All` و`None` لا يُسردان: فـ `All` لعلامات التسجيل وليس
> حبرًا.

```pdfl
check "Colors" {
  spots = prepress::detect_spot_colors()
  assert spots.length == 0, "file uses an unquoted special ink: #{spots.join(", ")}"

  mode = prepress::detect_color_mode()
  assert mode == "CMYK" || mode == "None",
    "document is #{mode} — offset printing requires CMYK"

  // تسامح لون العلامة التجارية
  difference = prepress::compare_colors_delta_e([1.0, 0.6, 0.0, 0.1], [1.0, 0.62, 0.0, 0.12])
  assert difference < 3.0, "brand color out of tolerance (ΔE #{difference})"

  // الأسود الغني تحت النص الصغير يُظهر خطأ التسجيل أكثر
  assert !prepress::detect_rich_black(),
    "rich black found — use flat black (0/0/0/100) for text"

  // الطباعة الفوقية غير المقصودة تُخفي عناصر
  assert prepress::validate_overprint_settings(),
    "overprint is enabled — verify that it is intentional"

  assert prepress::validate_output_intent(),
    "PDF has no Output Intent — the shop cannot know the target color profile"
}
```

---

## 6.3 سماكة الخطوط

| الدالة | الغرض |
|---|---|
| `prepress::detect_hairlines([limit])` | صحيحة إن وُجد خط دون الحدّ (0.25 pt افتراضًا) |
| `prepress::detect_hairlines_exact()` | صحيحة إن وُجد خط سماكته صفر |
| `prepress::detect_fine_lines([limit])` | مثلها (1 pt افتراضًا) |
| `prepress::validate_minimum_stroke_width(min)` | صحيحة إن بلغت كل الخطوط الحدّ الأدنى |

السماكة صفر هي الخط الشعري الكلاسيكي في PostScript: يصيّره الجهاز بأصغر عرض
ممكن، أي على نحو غير متوقَّع.

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

## 6.4 الخطوط

| الدالة | الغرض |
|---|---|
| `prepress::list_fonts()` | أسماء الخطوط المستعملة |
| `prepress::validate_font_embedding()` | صحيحة إن كانت كلها مضمَّنة |
| `prepress::detect_text_substitution()` | قائمة الخطوط غير المضمَّنة |
| `prepress::detect_missing_glyphs()` | خطوط بلا جدول عروض |
| `prepress::subset_fonts()` | صحيحة إن كانت كل الخطوط المضمَّنة مجموعات جزئية |
| `prepress::check_font_licensing()` | خطوط فيها خطر ترخيص (Type3 أو غير مضمَّنة) |
| `prepress::validate_font_size([min])` | صحيحة إن لم يكن ثمة نص دون الحجم الأدنى (6 pt افتراضًا) |

```pdfl
check "Fonts" {
  print("fonts:", prepress::list_fonts().join(", "))

  missing = prepress::detect_text_substitution()
  assert missing.length == 0,
    "fonts not embedded (text will change at the RIP): #{missing.join(", ")}"

  problems = prepress::detect_missing_glyphs()
  assert problems.length == 0,
    "fonts without a widths table: #{problems.join(", ")}"

  assert prepress::subset_fonts(),
    "a full font is embedded — the file is larger than it needs to be"

  risky = prepress::check_font_licensing()
  assert risky.length == 0, "fonts with licensing risk: #{risky.join(", ")}"

  // للنشرات الدوائية والعقود حجم أدنى تنظيمي
  assert prepress::validate_font_size(6),
    "there is text below 6 pt — illegible once printed"
}
```

---

## 6.5 الصفحات والأُطر

تحدّد أُطر PDF مناطق العمل: **MediaBox** (الورق) و**BleedBox** (الفيض)
و**TrimBox** (المقاس النهائي) و**CropBox** (العرض) و**ArtBox** (المحتوى).

| الدالة | الغرض |
|---|---|
| `prepress::get_page_size([page])` | `[العرض, الارتفاع]` بالنقاط |
| `prepress::get_page_boxes([page])` | قائمة الأُطر المعرَّفة |
| `prepress::validate_media_box()` | صحيحة إن كان لكل الصفحات MediaBox |
| `prepress::validate_trim_box()` | صحيحة إن كان لكلها TrimBox |
| `prepress::validate_bleed_box()` | صحيحة إن كان لكلها BleedBox |
| `prepress::check_page_geometry([margin])` | صحيحة إن بلغ الفيض القيمة من الجهات الأربع (3mm افتراضًا) |

```pdfl
check "Geometry" {
  size = prepress::get_page_size(1)
  assert abs(size.first() - 595.0) < 5, "width is outside A4"
  prepress::get_page_boxes(1).each { |box| print(box) }

  assert prepress::validate_trim_box(),
    "no TrimBox — the shop cannot know where to trim"
  assert prepress::validate_bleed_box(),
    "no BleedBox — no bleed area is defined"

  // حرف الوحدة يُقرأ جيدًا ويحوّل من تلقائه
  assert prepress::check_page_geometry(3mm),
    "bleed smaller than 3 mm on some page"
}
```

---

## 6.6 مثال كامل

```pdfl
// offset_magazine.pdfl — تدقيق كامل لما قبل الطبع في الأوفست
// الاستعمال: pdfl run offset_magazine.pdfl magazine.pdf --output html --output-file report.html
profile "offset-magazine" {

  const TAC_LIMIT = 300%
  const BLEED = 3mm
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress", "colors"] {
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMIT,
        "page #{page.number}: #{tac}% ink (limit #{TAC_LIMIT}%)"
    }
    print("average coverage:", prepress::calculate_ink_coverage(), "%")
  }

  check "Colors" tags: ["prepress", "colors"] {
    assert prepress::detect_color_mode() != "RGB", "document is in RGB"
    spots = prepress::detect_spot_colors()
    assert spots.length == 0, "unquoted special ink: #{spots.join(", ")}"
    assert !prepress::detect_rich_black(), "rich black in text"
    assert prepress::validate_output_intent(), "no Output Intent"
  }

  check "Fonts" tags: ["fonts"] {
    missing = prepress::detect_text_substitution()
    assert missing.length == 0, "fonts not embedded: #{missing.join(", ")}"
    assert prepress::validate_font_size(6), "text below 6 pt"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25), "strokes below 0.25 pt"
    assert !prepress::detect_hairlines_exact(), "stroke with 0 width"
  }

  check "Geometry" tags: ["prepress", "boxes"] {
    require prepress::validate_trim_box()
    require prepress::validate_bleed_box()
    assert prepress::check_page_geometry(BLEED), "bleed smaller than 3 mm"
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

[← `visual::`](05-visual.md) · [الفهرس](README.md) · [التالي: `codes::` →](07-codes.md)
