# 2. أنواع المستند

[← اللغة](01-language.md) · [الفهرس](README.md) · [التالي: `text::` →](03-text.md)

يحصل كل نص برمجي تلقائيًا على المتغير `doc` الذي يمثّل ملف PDF قيد التحليل.
ومنه تُبلغ الصفحات والخطوط والصور.

---

## 2.1 `doc` — المستند

| الخاصية | النوع | المحتوى |
|---|---|---|
| `doc.page_count` | عدد | عدد الصفحات |
| `doc.title` | نص | العنوان من البيانات الوصفية (فارغ إن غاب) |
| `doc.author` | نص | المؤلف من البيانات الوصفية (فارغ إن غاب) |
| `doc.filename` | نص | اسم الملف المُحلَّل |
| `doc.pages` | قائمة | كل الصفحات |
| `doc.fonts` | قائمة | كل الخطوط المستعملة |
| `doc.images` | قائمة | كل الصور في جميع الصفحات |

التابع: `doc.extract_text()` — نص المستند كله، والصفحات مفصولة بأسطر جديدة.

```pdfl
check "Document properties" {
  print("file:", doc.filename)
  print("pages:", doc.page_count)

  // هذه المجموعات قوائم عادية — كل توابع القوائم تعمل عليها
  require doc.pages.length == doc.page_count
  require doc.fonts.length > 0

  text = doc.extract_text()
  assert text.trim() != "", "PDF has no extractable text (images only?)"
  print("total characters:", text.length)
}
```

---

## 2.2 `page` — الصفحة

تأتي الصفحات من `doc.pages` (داخل كتلة) أو من المتغير `page` (داخل `rule`).

| الخاصية | النوع | المحتوى |
|---|---|---|
| `page.number` | عدد | رقم الصفحة، ابتداءً من **1** |
| `page.index` | عدد | الفهرس، ابتداءً من **0** |
| `page.width` / `page.height` | عدد | العرض / الارتفاع بالنقاط |
| `page.images` | قائمة | صور هذه الصفحة |
| `page.tac` | عدد | أقصى تغطية حبر مقدَّرة (%) |
| `page.ink_coverage` | عدد | متوسط تغطية الحبر المقدَّر (%) |
| `page.min_stroke_width` | عدد/null | أدقّ خط (pt)؛ `null` إن لم يكن ثمة خطوط |
| `page.has_media_box` وغيرها | منطقي | `has_crop_box` و`has_trim_box` و`has_bleed_box` و`has_art_box` |

التابع: `page.extract_text()` — نص هذه الصفحة وحدها.

```pdfl
check "Page format" {
  doc.pages.each { |page|
    // number هو الرقم الذي يقرؤه الناس، و index للحسابات الداخلية
    assert page.width > 100mm,
      "page #{page.number} is too narrow: #{page.width}pt"

    // الأُطر: لا غنى عنها للطباعة
    assert page.has_trim_box, "page #{page.number} has no TrimBox (trim area)"
    assert page.has_bleed_box, "page #{page.number} has no BleedBox (bleed area)"

    assert page.tac <= 300,
      "page #{page.number}: #{page.tac}% ink (limit 300%)"

    // قد تكون min_stroke_width تساوي null (لا خطوط في الصفحة).
    // و null خاطئ، فهذا آمن:
    assert !page.min_stroke_width || page.min_stroke_width >= 0.25,
      "page #{page.number} has a hairline stroke"
  }
}

check "Blank pages" {
  blank = doc.pages.filter { |p| p.extract_text().trim() == "" }
  assert blank.length == 0,
    "#{blank.length} blank page(s): #{blank.map { |p| p.number }.join(", ")}"
}
```

---

## 2.3 `font` — الخط

يأتي من `doc.fonts`. الخصائص: `font.name` (الاسم) و`font.is_embedded` (مضمَّن
أو لا).

```pdfl
check "Embedded fonts" {
  // الخط غير المضمَّن يستبدله البرنامج القارئ — فيتغير شكل النص
  doc.fonts.each { |font|
    assert font.is_embedded,
      "font '#{font.name}' is not embedded in the PDF"
  }
  print("fonts in use:", doc.fonts.map { |f| f.name }.join(", "))
}
```

---

## 2.4 `image` — الصورة

تأتي من `doc.images` (كلها) أو `page.images` (صور صفحة واحدة).

| الخاصية | المحتوى |
|---|---|
| `image.width` / `image.height` | العرض / الارتفاع **بالبكسل** |
| `image.dpi` | الدقة الفعلية (الأصغر بين dpi_x و dpi_y) |
| `image.dpi_x` / `image.dpi_y` | الدقة الفعلية أفقيًا / رأسيًا |
| `image.color_space` | `DeviceRGB` أو `DeviceCMYK` أو `Indexed`… |
| `image.page_number` | الصفحة التي فيها الصورة (ابتداءً من 1) |
| `image.bits_per_pixel` | عمق البتات |

> **الدقة فعلية**، تُحسب بقسمة البكسلات على المقاس المطبوع في الصفحة، لا القيمة
> الاسمية في البيانات الوصفية. وهذا هو الرقم الذي يقرّر جودة الطباعة: صورة
> بعرض 1000 بكسل ممدودة على 20 سم دقّتها منخفضة مهما ادّعت بياناتها الوصفية.

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
    // الأوفست يعمل بـ CMYK، وما كان RGB وجب تحويله
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

## 2.5 `region` — منطقة من الصفحة

المنطقة تحدّ جزءًا من الصفحة بمستطيل. تفيد في تدقيق التذييل والترويسة وموضع
الباركود والشريط التنظيمي.

الإنشاء: `region(x, y, العرض, الارتفاع [, "الاسم"])`، ونقطة الأصل (0,0) أسفل
اليسار كما في PDF.

| الخاصية | المحتوى | | التابع | الغرض |
|---|---|---|---|---|
| `region.name` | الاسم المعطى عند الإنشاء | | `contains_point(x, y)` | هل النقطة داخلها؟ |
| `region.x` / `region.y` | الزاوية السفلى اليسرى | | `intersects(other)` | هل تتداخل المنطقتان؟ |
| `region.width` / `region.height` | الأبعاد | | `expand(pt)` | منطقة جديدة أوسع من كل جهة |
| `region.right` / `region.top` | الحافة اليمنى / العليا (محسوبتان) | | `inset(pt)` | منطقة جديدة أضيق من كل جهة |
| `region.area` | المساحة (نقطة مربعة) | | `export_coordinates()` | `[x0, y0, x1, y1]` |

```pdfl
check "Working with regions" {
  footer = region(0, 0, 595, 60, "footer")

  require footer.name == "footer"
  require footer.top == 60.0
  require footer.right == 595.0
  require footer.area == 35700.0
  require footer.contains_point(300, 30)
  require !footer.contains_point(300, 500)

  // كشف التداخل: مفيد لاكتشاف عنصر يقتحم منطقة محجوزة
  header = region(0, 780, 595, 62)
  require !footer.intersects(header)

  // expand/inset تُرجع منطقة «جديدة» (والأصلية تبقى كما هي)
  require footer.expand(5mm).area > footer.area
  require footer.inset(3mm).area < footer.area
}

profile "medicine-label" {
  check "Prescription band" {
    // يجب أن يكون الشريط في الأعلى وأن يحمل النص الإلزامي
    band = region(0, 700, 595, 142, "band")
    assert text::extract_from_region(1, band).contains("PRESCRIPTION ONLY"),
      "band is missing the mandatory text"
  }

  check "Ink in the fold area" {
    // كثرة الحبر عند الطية تتشقق في التشطيب
    fold = region(290, 0, 15, 842, "center fold")
    measured = prepress::calculate_tac_by_region(1, fold)
    assert measured.first() < 240,
      "too much ink on the fold: #{measured.first()}%"
  }

  check "Barcode in the right place" {
    code_area = region(400, 20, 180, 80, "barcode area")
    assert codes::validate_barcode_position(code_area),
      "barcode outside the reserved area"
  }
}
```

---

[← اللغة](01-language.md) · [الفهرس](README.md) · [التالي: `text::` →](03-text.md)
