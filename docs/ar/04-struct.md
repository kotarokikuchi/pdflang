# 4. فضاء الأسماء `struct::` — البنية والبيانات الوصفية

[← `text::`](03-text.md) · [الفهرس](README.md) · [التالي: `visual::` →](05-visual.md)

23 دالة تخصّ الملف نفسه: البيانات الوصفية، والكائنات الداخلية، والأمان،
وإمكان التتبّع.

> الدوال ابتداءً من `list_objects` تقرأ البنية الداخلية للملف. وهذا التحليل
> يجري **مرة واحدة فقط** عند أول استعمال ثم يُخزَّن مؤقتًا.

---

## 4.1 البيانات الوصفية

| الدالة | تُرجع |
|---|---|
| `struct::get_title()` | العنوان |
| `struct::get_author()` | المؤلف |
| `struct::get_subject()` | الموضوع |
| `struct::get_keywords()` | الكلمات المفتاحية |
| `struct::get_creator()` | البرنامج الذي أنشأ المستند الأصلي |
| `struct::get_producer()` | البرنامج الذي أنتج ملف PDF |
| `struct::get_creation_date()` | تاريخ الإنشاء (`YYYY-MM-DD HH:MM:SS`) |
| `struct::get_modification_date()` | تاريخ التعديل (بالصيغة نفسها) |
| `struct::list_metadata_entries()` | قائمة المدخلات غير الفارغة (`"المفتاح: القيمة"`) |
| `struct::extract_xmp()` | بيانات XMP الوصفية من الفهرس |

جميعها تُرجع نصًا فارغًا إذا غاب الحقل.

```pdfl
check "Required metadata" {
  assert struct::get_title() != "", "PDF has no title"
  assert struct::get_author() != "", "PDF has no author"

  // Producer يكشف الأداة الأصل — مفيد لتتبّع المشكلات
  print("produced by:", struct::get_producer())

  created = struct::get_creation_date()
  assert created != "", "PDF has no creation date"
  // مقارنة النصوص تعمل لأن الصيغة تُرتَّب ترتيبًا صحيحًا
  assert created > "2026-01-01", "file is too old for this campaign"

  xmp = struct::extract_xmp()
  assert xmp.contains("pdfaid"), "no PDF/A identification in the XMP"
}
```

---

## 4.2 الملف وإمكان التتبّع

| الدالة | الغرض |
|---|---|
| `struct::file_size()` | الحجم بالبايت |
| `struct::calculate_sha256()` | بصمة SHA-256 للملف |
| `struct::detect_file_bloat([kb_per_page])` | صحيحة فوق الحدّ لكل صفحة (1024 ك.ب افتراضًا) |

```pdfl
check "File size and traceability" {
  mb = struct::file_size() / 1024 / 1024
  assert mb < 10, "file is #{round(mb)} MB (10 MB e-mail limit)"

  // البصمة تثبت أي ملف تحديدًا جرت الموافقة عليه
  print("SHA-256:", struct::calculate_sha256())

  assert !struct::detect_file_bloat(1024),
    "heavy file: #{struct::file_size() / 1024} KB for #{doc.page_count} pages"
}
```

---

## 4.3 الكائنات الداخلية

| الدالة | الغرض |
|---|---|
| `struct::count_objects()` | عدد كائنات المحتوى في الصفحات |
| `struct::list_objects()` | كل الكائنات (`"الرقم: النوع"`) |
| `struct::detect_unreferenced_objects()` | كائنات لا يمكن بلوغها من الـ trailer |
| `struct::detect_orphaned_resources()` | موارد لا يمكن بلوغها (خطوط، صور) |
| `struct::measure_object_size(number)` | حجم كائن معيّن تقريبًا بالبايت |

> كائنات البنية التحتية (`ObjStm` و`XRef`) مستثناة: فهي بحكم تعريفها لا
> يُشار إليها من الـ trailer، والإبلاغ عنها إنذار كاذب.

```pdfl
check "File hygiene" {
  require struct::count_objects() > 0

  loose = struct::detect_unreferenced_objects()
  assert loose.length == 0,
    "#{loose.length} unreferenced object(s): #{loose.join(", ")}"

  orphans = struct::detect_orphaned_resources()
  assert orphans.length == 0,
    "unused embedded resources: #{orphans.join(", ")} — run 'pdfl fix' with remove_unused_resources()"
}
```

---

## 4.4 الأمان

| الدالة | الغرض |
|---|---|
| `struct::detect_javascript()` | صحيحة إن وُجد JavaScript مضمَّن |
| `struct::detect_suspicious_actions()` | قائمة الإجراءات الخطرة |
| `struct::check_encryption()` | صحيحة إن كان المستند مشفَّرًا |
| `struct::validate_permissions()` | صحيحة إن لم تكن ثمة قيود |
| `struct::validate_signatures()` | صحيحة إن وُجدت حقول توقيع |

تكشف `detect_suspicious_actions` عن `JavaScript` و`Launch` (تشغيل برنامج)
و`URI` و`SubmitForm` و`ImportData` و`GoToR`.

> `validate_signatures` تتحقق من **وجود** تلك الحقول. أما التحقق التعمياني من
> سلسلة الشهادات فليس في هذا الإصدار.

```pdfl
check "Security" {
  // JavaScript داخل PDF مسلك هجوم شائع
  // ولا لزوم له في مستند معدّ للطباعة
  assert !struct::detect_javascript(), "PDF contains embedded JavaScript"

  actions = struct::detect_suspicious_actions()
  assert actions.length == 0,
    "suspicious actions in the PDF: #{actions.join("; ")}"

  // ملف PDF المشفَّر قد يفشل على RIP المطبعة
  assert !struct::check_encryption(),
    "PDF is encrypted — remove protection before sending it to print"
  assert struct::validate_permissions(),
    "PDF has permission restrictions that may block processing"
}
```

---

## 4.5 مثال كامل

```pdfl
// audit.pdfl — تدقيق الامتثال والأمان
profile "file-audit" {

  check "Identification" tags: ["metadata"] {
    assert struct::get_title() != "", "no title"
    assert struct::get_author() != "", "no author"
    assert struct::get_creation_date() != "", "no creation date"
    print("produced by:", struct::get_producer())
  }

  check "Traceability" tags: ["audit"] {
    print("SHA-256:", struct::calculate_sha256())
    print("size:", struct::file_size() / 1024, "KB")
  }

  check "Security" tags: ["security"] {
    assert !struct::detect_javascript(), "embedded JavaScript"
    assert !struct::check_encryption(), "encrypted file"
    actions = struct::detect_suspicious_actions()
    assert actions.length == 0, "suspicious actions: #{actions.join("; ")}"
  }

  check "File hygiene" tags: ["optimization"] {
    orphans = struct::detect_orphaned_resources()
    assert orphans.length == 0, "unused resources: #{orphans.join(", ")}"
    assert !struct::detect_file_bloat(1024), "bloated file"
  }
}
```

---

[← `text::`](03-text.md) · [الفهرس](README.md) · [التالي: `visual::` →](05-visual.md)
