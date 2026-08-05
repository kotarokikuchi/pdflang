# 10. المكتبة القياسية

[← `data::`](09-data.md) · [الفهرس](README.md) · [التالي: سطر الأوامر →](11-cli.md)

توابع القوائم والنصوص، والدوال العامة المتاحة في كل مكان من النص البرمجي.

---

## 10.1 توابع القوائم

| التابع | الغرض |
|---|---|
| `list.each { \|item\| ... }` | ينفّذ الكتلة على كل عنصر |
| `list.each_with_index { \|item, i\| ... }` | يعطي الموضع أيضًا (ابتداءً من **0**) |
| `list.all { \|item\| ... }` | صحيح إن حقّق الجميع الشرط (صحيح على القائمة الفارغة) |
| `list.any { \|item\| ... }` | صحيح إن حقّقه واحد على الأقل (خاطئ على الفارغة) |
| `list.filter { \|item\| ... }` | يُبقي من يحقّق الشرط فقط |
| `list.map { \|item\| ... }` | قائمة جديدة محوَّلة |
| `list.length` | عدد العناصر (و`length()` تعمل أيضًا) |
| `list.contains(value)` | هل القيمة في القائمة؟ |
| `list.get(n)` | العنصر رقم n (ابتداءً من **1**) |
| `list.first()` / `list.last()` | الأول / الأخير (`null` على القائمة الفارغة) |
| `list.join([separator])` | يصل العناصر نصًّا (الفاصل الافتراضي `", "`) |

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

  bad = doc.images.filter { |img| img.dpi < 300 }
  assert bad.length == 0, "#{bad.length} image(s) with low resolution"

  print("fonts:", doc.fonts.map { |f| f.name }.join(", "))

  // get تبدأ من 1: get(1) هو العنصر الأول
  row = data::load_dataset("data/batches.csv").get(2)
  print("first column:", row.get(1))

  // آمن حتى على القائمة الفارغة: null خاطئ
  spots = prepress::detect_spot_colors()
  assert !spots.first() || spots.first() == "Varnish",
    "unexpected special ink: #{spots.first()}"
}
```

---

## 10.2 توابع النصوص

| التابع | الغرض |
|---|---|
| `text.contains(sub)` | هل يحتوي هذا الجزء؟ |
| `text.starts_with(sub)` | هل يبدأ به؟ |
| `text.ends_with(sub)` | هل ينتهي به؟ |
| `text.trim()` | يزيل المسافات من الطرفين |
| `text.to_uppercase()` | كل الأحرف كبيرة |
| `text.to_lowercase()` | كل الأحرف صغيرة |
| `text.length` | عدد المحارف |

```pdfl
check "String methods" {
  title = doc.title
  require title.length > 0
  require title.trim() == title          // لا مسافات زائدة
  assert !title.to_lowercase().contains("draft"),
    "title still marked as draft"

  code = codes::decode_barcode(1)
  assert code.starts_with("789"), "GTIN is not Brazilian"
  assert doc.filename.ends_with(".pdf"), "unexpected extension"
}

check "contains on each type" {
  // النص: يبحث عن «جزء» داخل النص
  require "final document".contains("final")

  // القائمة: تبحث عن «عنصر» كامل
  require ["a", "b"].contains("a")
  require !["ab"].contains("a")      // "a" ليست عنصرًا في هذه القائمة
}
```

---

## 10.3 الدوال العامة

| الدالة | الغرض |
|---|---|
| `min(a, b)` / `max(a, b)` | الأصغر / الأكبر |
| `abs(x)` | القيمة المطلقة |
| `round(x)` | يقرّب إلى أقرب عدد صحيح |
| `print(...)` | يطبع مفصولًا بمسافات على **مخرج الأخطاء** |
| `region(x, y, w, h [, name])` | ينشئ منطقة ([الفصل 2](02-types.md)) |

`print` يكتب على مخرج الأخطاء، فلا يستقبل `> report.json` سوى التقرير.

```pdfl
check "Global functions" {
  const A4_WIDTH = 595.0
  const TOLERANCE = 5.0

  // abs مفتاح مقارنة المقاسات بتسامح
  doc.pages.each { |page|
    assert abs(page.width - A4_WIDTH) < TOLERANCE,
      "page #{page.number} is outside A4: #{page.width}pt"
  }

  // round يجعل الرسائل مقروءة
  // بدونه: "217.4453125 DPI"، ومعه: "217 DPI".
  doc.images.each { |img|
    assert img.dpi >= 300,
      "image on page #{img.page_number}: #{round(img.dpi)} DPI"
  }

  print("document:", doc.filename)
  print("pages:", doc.page_count, "| fonts:", doc.fonts.length)
}
```

---

## 10.4 صيغ شائعة

```pdfl
// عدّ العناصر التي لا تجتاز الشرط
check "Problem count" {
  bad = doc.images.filter { |i| i.dpi < 300 }
  assert bad.length == 0,
    "#{bad.length} of #{doc.images.length} images below 300 DPI"
}

// سرد العناصر المخالفة في الرسالة
check "List in the message" {
  // السلسلة في السطر نفسه: لا سطر جديد قبل النقطة
  problems = doc.pages.filter { |p| !p.has_trim_box }.map { |p| p.number }
  assert problems.length == 0,
    "pages without a TrimBox: #{problems.join(", ")}"
}

// تدقيق بتسامح
function close_to(value, target, tolerance) {
  abs(value - target) < tolerance
}

check "With tolerance" {
  doc.pages.each { |page|
    assert close_to(page.width, 595.0, 2.0),
      "page #{page.number}: width #{page.width}pt (expected 595 ± 2)"
  }
}

// ألا يتعطل على مستند فارغ
check "Defensive" {
  // التقييم القصير يمنع استدعاء first() على قائمة فارغة
  assert doc.pages.length == 0 || doc.pages.first().width > 0,
    "the first page has no width"
}
```

---

[← `data::`](09-data.md) · [الفهرس](README.md) · [التالي: سطر الأوامر →](11-cli.md)
