# 3. فضاء الأسماء `text::` — النص

[← الأنواع](02-types.md) · [الفهرس](README.md) · [التالي: `struct::` →](04-struct.md)

25 دالة لاستخراج نص المستند وتوحيده والبحث فيه وتدقيقه.

> في الدوال المؤشَّرة بـ `[text]` يكون الوسيط **اختياريًا**: بدونه تعمل الدالة
> على المستند كله، ومعه على النص الذي تمرّره.

---

## 3.1 الاستخراج

| الدالة | الغرض |
|---|---|
| `text::extract_all()` | نص المستند كله (الصفحات موصولة بأسطر جديدة) |
| `text::extract_from_page(page)` | نص صفحة واحدة (ابتداءً من 1) |
| `text::extract_from_region(page, region)` | نص منطقة محددة (نص فارغ إن لم يوجد) |
| `text::extract_with_normalization()` | نص المستند موحَّدًا سلفًا |

```pdfl
check "Extraction" {
  content = text::extract_all()
  assert content.trim() != "", "PDF has no extractable text"

  cover = text::extract_from_page(1)
  assert cover.contains("User Manual"), "cover lacks the expected title"

  // تذييلات الإنتاج (اسم ملف InDesign وتاريخ التصدير) تنجو أحيانًا
  // حتى الملف النهائي
  footer = region(0, 0, 467, 40, "footer")
  doc.pages.each { |page|
    line = text::extract_from_region(page.number, footer)
    assert !line.contains(".indd"),
      "page #{page.number} has a production mark in the footer: #{line.trim()}"
  }
}
```

---

## 3.2 التوحيد والتقسيم

| الدالة | الغرض |
|---|---|
| `text::normalize([text])` | تصغير الأحرف + ضغط المسافات |
| `text::split_words([text])` | التقسيم إلى كلمات (مع إزالة الترقيم من الأطراف) |
| `text::split_sentences([text])` | التقسيم إلى جمل |
| `text::split_paragraphs([text])` | التقسيم إلى فقرات (سطر فارغ) |
| `text::count_words([text])` | عدد الكلمات |
| `text::count_characters([text])` | عدد المحارف |
| `text::detect_language([text])` | `"pt"` أو `"en"` أو `"es"` أو `"unknown"` |

```pdfl
check "Normalization and splitting" {
  require text::normalize("  HELLO   World  ") == "hello world"

  words = text::split_words("Hello, world! (test)")
  require words.length == 3
  require words.first() == "Hello"

  // للنشرات الدوائية والعقود حدّ عملي للقراءة
  text::split_sentences().each { |sentence|
    assert sentence.length < 400,
      "sentence with #{sentence.length} characters — hard to read"
  }

  require text::count_words() > 100
  assert text::detect_language() == "en",
    "document should be in English, detected: #{text::detect_language()}"
}
```

---

## 3.3 البحث والمحتوى الإلزامي

| الدالة | الغرض |
|---|---|
| `text::require_text(term)` | صحيح إن وُجد المصطلح |
| `text::forbid_text(term)` | صحيح إن لم يوجد |
| `text::require_match(regex)` | صحيح إن وجد التعبير النمطي شيئًا |
| `text::forbid_match(regex)` | صحيح إن لم يجد شيئًا |
| `text::fuzzy_match(a, b)` | تشابه نصّين (من 0.0 إلى 1.0) |

المقارنة تتجاهل حالة الأحرف والمسافات.

```pdfl
profile "contract" {
  check "Mandatory clauses" {
    assert text::require_text("governing law"),
      "contract has no governing-law clause"
    assert text::require_match("\d{4}/\d{4}"), "contract number not found"
  }

  check "Forbidden terms" {
    assert text::forbid_text("DRAFT"), "document still marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text was not replaced"
    assert text::forbid_match("\d{2}-\d{2}-\d{4}"), "US-format date found"
  }

  check "Name with tolerance" {
    // مفيد حين تُتوقَّع أخطاء طباعية أو ضجيج OCR
    found = text::extract_from_region(1, region(50, 700, 300, 40))
    similarity = text::fuzzy_match("Paracetamol 750mg", found)
    assert similarity > 0.9,
      "product name differs from expected (#{round(similarity * 100)}% similar)"
  }
}
```

---

## 3.4 البيانات الشخصية

`text::detect_personal_data([text])` و`text::detect_pii([text])` مترادفتان، وتُرجعان
**قائمة** البيانات الشخصية الموجودة: CPF وCNPJ (رقما الضريبة البرازيليان)
والبريد الإلكتروني والهاتف.

> لا يدخل CPF أو CNPJ القائمة إلا إذا كان **رقم التحقق صحيحًا**. أما رقم يشبه
> CPF ظاهريًا فقط (مثل `111.111.111-12`) فلا يثير أي إنذار.

```pdfl
check "Public document must carry no personal data" {
  found = text::detect_personal_data()
  assert found.length == 0, "personal data exposed: #{found.join("; ")}"

  // كل مدخلة على هيئة "CPF: 529.982.247-25"
  text::detect_pii().each { |item| print("found:", item) }
}
```

---

## 3.5 تدقيقات الصيغة

| الدالة | الغرض |
|---|---|
| `text::validate_cpf(text)` | رقم تحقق CPF (mod 11) |
| `text::validate_cnpj(text)` | رقم تحقق CNPJ |
| `text::validate_date_format(text [, format])` | تاريخ صحيح فعلًا في التقويم |
| `text::validate_phone_format(text)` | صيغة الهاتف البرازيلي |
| `text::validate_format(text, regex)` | هل يطابق النص **كاملًا**؟ |

صيغ التاريخ المقبولة: `"dd/mm/aaaa"` و`"aaaa-mm-dd"`؛ وبلا وسيط ثانٍ تُقبل
الصيغتان.

```pdfl
check "Format validation" {
  require text::validate_cpf("529.982.247-25")
  require !text::validate_cpf("111.111.111-11")    // أرقام متطابقة
  require text::validate_cnpj("11.222.333/0001-81")

  require text::validate_date_format("29/02/2024")   // 2024 سنة كبيسة
  require !text::validate_date_format("29/02/2023")  // 2023 ليست كذلك
  require !text::validate_date_format("31/04/2026")  // أبريل 30 يومًا

  require text::validate_phone_format("(11) 98765-4321")

  // رمز الدفعة بصيغة المصنع
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  assert text::validate_format(batch, "L\d{4}-\d{2}"),
    "batch code does not follow the L0000-00 pattern: #{batch}"
}
```

---

## 3.6 المقارنة والتشخيص

`text::diff(a, b)` تسرد الأسطر المتغيرة (`-` محذوف، `+` مضاف).
و`text::detect_rasterized_text()` صحيحة إن وُجد نص حُوِّل إلى صورة.

```pdfl
check "Comparison and diagnostics" {
  changes = text::diff(text::extract_from_page(1), text::extract_from_page(2))
  print("changed lines:", changes.length)

  // الصفحة الممسوحة ضوئيًا أو المحوَّلة إلى مسارات لا يمكن البحث فيها
  // ولا يقرؤها قارئ الشاشة
  assert !text::detect_rasterized_text(),
    "there are pages with rasterized text (scanned or outlined)"
}
```

> لمقارنة **ملفين** استعمل الأمر `pdfl compare`، فهو يحاذي الصفحات تلقائيًا.
> انظر [الفصل 11](11-cli.md).

---

## 3.7 مثال كامل

```pdfl
// legal_document.pdfl — تدقيق عقد
profile "standard-contract" {

  check "Required content" tags: ["legal"] {
    assert text::require_text("governing law"), "no governing-law clause"
    assert text::require_text("term of agreement"), "no term clause"
    assert text::require_match("\d{4}/\d{4}"), "no contract number"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("XXX+"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    found = text::detect_personal_data()
    assert found.length == 0,
      "personal data in a public document: #{found.join("; ")}"
  }

  check "Text quality" tags: ["text"] {
    assert text::detect_language() == "en", "document is not in English"
    assert !text::detect_rasterized_text(), "rasterized text blocks search"
    require text::count_words() > 200
  }
}
```

---

[← الأنواع](02-types.md) · [الفهرس](README.md) · [التالي: `struct::` →](04-struct.md)
