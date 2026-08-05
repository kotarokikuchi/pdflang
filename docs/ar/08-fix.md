# 8. فضاء الأسماء `fix::` — التوحيد

[← `codes::`](07-codes.md) · [الفهرس](README.md) · [التالي: `data::` →](09-data.md)

19 عملية **تعدّل** ملف PDF وتحفظه باسم جديد. أما الملف الأصلي فلا يُمسّ أبدًا.

---

## 8.1 كيفية الاستعمال

`fix::` هو فضاء الأسماء الوحيد الذي يكتب، ولذلك له أمره الخاص:

```bash
pdfl fix input.pdf script.pdfl --output fixed.pdf
```

| الخيار | الغرض |
|---|---|
| `--output <file>` | ملف PDF الناتج (إلزامي) |
| `--dry-run` | يسرد العمليات دون حفظ |
| `--report json\|csv\|html\|pdf` | صيغة التقرير |
| `--report-file <file>` | يكتب التقرير في ملف |

استدعاء `fix::` من `pdfl run` ينتج خطأً يذكر الأمر الصحيح — كي لا يعدّل أحدٌ
ملفًا وهو يظن أنه يدقّقه فحسب.

### كيف تُنفَّذ العمليات

```pdfl
// هذا النص البرمجي لا يحتاج فحوصًا: هذه أوامر
// تُنفَّذ بالترتيب.
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::add_page_numbers()
fix::add_watermark("DRAFT")
```

يُدقَّق كل استدعاء **في موضعه** (صفحة غير موجودة، دوران غير صالح، ملف مفقود)
قبل تطبيقه. ويحتفظ التقرير بما جرى في الحقل `fixes`:

```json
"fixes": [
  "TrimBox set to [8.5, 8.5, 586.5, 833.5]",
  "page numbering added",
  "watermark \"DRAFT\" added"
]
```

ولا بأس بمزج التدقيقات والتعديلات في النص البرمجي نفسه:

```pdfl
// دقّق قبل التعديل — وإن لم يتحقق الشرط ظهر ذلك في التقرير
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(), "file is encrypted, cannot fix it"
}

fix::add_page_numbers()
```

---

## 8.2 أُطر الصفحة

| العملية | الغرض |
|---|---|
| `fix::set_page_size(width, height)` | يضبط MediaBox لكل الصفحات |
| `fix::set_crop_box(x0, y0, x1, y1)` | يضبط CropBox لكل الصفحات |
| `fix::set_trim_box(x0, y0, x1, y1)` | يضبط TrimBox لكل الصفحات |
| `fix::set_bleed_box(x0, y0, x1, y1)` | يضبط BleedBox لكل الصفحات |

الإحداثيات بالنقاط، من أسفل اليسار إلى أعلى اليمين.

```pdfl
// اكتب بالوحدات، والتحويل يجري من تلقائه
fix::set_page_size(210mm, 297mm)

// ملف الناشر بلا أُطر إنتاج:
// TrimBox = المقاس النهائي، BleedBox = مع فيض 3 مم
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)
```

---

## 8.3 الصفحات

| العملية | الغرض |
|---|---|
| `fix::rotate_page([page,] degrees)` | تدوير 90/180/270 درجة (بلا رقم: كل الصفحات) |
| `fix::delete_page(n)` | يحذف صفحة |
| `fix::duplicate_page(n)` | ينسخ صفحة (والنسخة تليها مباشرة) |
| `fix::reorder_pages([...])` | يعيد الترتيب (كل صفحة مرة واحدة بالضبط) |
| `fix::split_document(from, to, "out.pdf")` | يحفظ مدى صفحات في ملف |
| `fix::merge_documents("other.pdf")` | يُلحق صفحات ملف PDF آخر في النهاية |

حذف الصفحة الوحيدة في المستند مرفوض صراحةً.

```pdfl
fix::rotate_page(90)        // كل الصفحات
fix::rotate_page(3, 180)    // الصفحة 3 فقط
fix::delete_page(1)         // يزيل غلاف المسودّة
fix::reorder_pages([4, 1, 2, 3])

// الغلاف والمتن يذهبان إلى مورّدين مختلفين
fix::split_document(1, 2, "cover.pdf")
fix::split_document(3, 50, "body.pdf")

fix::merge_documents("attachments/warranty.pdf")
```

---

## 8.4 المحتوى

| العملية | الغرض |
|---|---|
| `fix::add_watermark("text")` | علامة مائية رمادية مائلة على كل الصفحات |
| `fix::add_stamps("text")` | ختم أحمر أعلى يمين كل صفحة |
| `fix::add_page_numbers()` | يضع `n / الإجمالي` في التذييل |
| `fix::remove_annotations()` | يزيل كل التعليقات التوضيحية |
| `fix::remove_attachments()` | يزيل كل المرفقات |
| `fix::flatten_layers()` | يفكّ بنية المحتوى الاختياري (OCG) |

```pdfl
fix::add_watermark("DRAFT — DO NOT PRINT")
fix::add_stamps("APPROVED 2026-08-02")
fix::add_page_numbers()

// قبل الطباعة: تعليقات المراجعة لا تمرّ،
// والمرفقات لا تزيد الملف إلا ثقلًا
fix::remove_annotations()
fix::remove_attachments()

// يمنع إعادة تشغيل طبقة «النسخة الإنجليزية» المطفأة في المطبعة
fix::flatten_layers()
```

---

## 8.5 التحسين

> عمليات هذا القسم **لا تكتب إلا إذا صغر الملف**. وإن جاءت إعادة الكتابة أكبر
> بقي الأصل.

| العملية | الغرض |
|---|---|
| `fix::remove_unused_resources()` | يطرح الكائنات التي لا تُبلغ من الـ trailer |
| `fix::downsample_images([dpi])` | يعيد معاينة الصور فوق الدقة المستهدفة (300 افتراضًا) |
| `fix::compress_images([quality])` | يعيد الترميز إلى JPEG (من 1 إلى 100، و85 افتراضًا) |

تُحسب الدقة من **المقاس المطبوع فعلًا** في الصفحة.

> **صور CMYK تبقى كما هي.** فإعادة معاينتها تقتضي المرور بـ RGB، وذلك يُتلف
> الفصل اللوني لما قبل الطبع. وفي ملف المطبعة يأتي التوفير من صور RGB.

```pdfl
// نسخة الموافقة بالبريد الإلكتروني لا تحتاج 300 DPI
fix::downsample_images(96)
fix::compress_images(70)
fix::remove_unused_resources()
```

### ما ليس موجودًا هنا

`subset_fonts` و`linearize_document` **ليستا** من عمليات `fix::`، واستدعاؤهما
يعطي خطأ «دالة غير معروفة».

- **subset_fonts**: نُفِّذت ثم قيست. فالأدوات الاحترافية لا تضمّن أصلًا إلا
  الرموز المستعملة، والمكسب المقيس كان 0.5 % في أحسن الأحوال وصفرًا فيما عداه
  — وهذا لا يستحق خطر إتلاف خط. أما *للتحقق* من كون الخطوط مجموعات جزئية
  فاستعمل [`prepress::subset_fonts()`](06-prepress.md).
- **linearize_document**: تقتضي توليد جداول التلميح (البند 7.14 من مواصفة PDF).
  ولا مكتبة Rust تقوم بذلك، والتنفيذ الجزئي لا تعترف به البرامج القارئة على أنه
  «Fast Web View».

---

## 8.6 أمثلة كاملة

```pdfl
// prepare_for_print.pdfl — تهيئة ملف ناشر للمطبعة
// الاستعمال: pdfl fix publisher.pdf prepare_for_print.pdfl --output print.pdf

check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask the publisher for the open version"
}

// أُطر الإنتاج التي لم يضبطها الناشر
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// التنظيف: لا تعليقات مراجعة ولا مرفقات إلى الطباعة
fix::remove_annotations()
fix::remove_attachments()
fix::flatten_layers()
fix::remove_unused_resources()
```

```pdfl
// email_version.pdfl — نسخة خفيفة للموافقة بالبريد
// الاستعمال: pdfl fix final.pdf email_version.pdfl --output approval.pdf

fix::downsample_images(96)
fix::compress_images(70)
fix::add_watermark("PROOF — NOT THE FINAL VERSION")
fix::add_page_numbers()
```

تحقّق من النتيجة بـ `pdfl` نفسه:

```bash
pdfl fix final.pdf email_version.pdfl --output approval.pdf
pdfl inspect approval.pdf          # حجم الملف الجديد ودقّته وتحذيراته
```

---

[← `codes::`](07-codes.md) · [الفهرس](README.md) · [التالي: `data::` →](09-data.md)
