# 7. فضاء الأسماء `codes::` — الباركود ورمز QR

[← `prepress::`](06-prepress.md) · [الفهرس](README.md) · [التالي: `fix::` →](08-fix.md)

13 دالة لكشف الباركود ورموز QR في المستند وفكّ ترميزها والتحقق منها.

> القراءة تصيّر الصفحات بدقة عالية وتجري **مرة واحدة فقط** عند أول استدعاء
> لدالة من `codes::`. والنص البرمجي الذي لا يستعمل هذا الفضاء لا يدفع هذه
> الكلفة.

الصيغ المدعومة: EAN-8/13 وUPC-A/E وCode 128 وCode 39 وITF وQR Code وData
Matrix وAztec وPDF417.

---

## 7.1 الكشف

| الدالة | الغرض |
|---|---|
| `codes::detect_barcodes()` | صحيحة إن وُجد باركود |
| `codes::detect_qrcodes()` | صحيحة إن وُجد رمز QR |
| `codes::count_barcodes()` | العدد الكلي للرموز المقروءة |
| `codes::get_barcode_type(n)` | صيغة الرمز رقم n (`"EAN_13"` و`"QR_CODE"`…) |
| `codes::get_barcode_location(n)` | الموضع `[الصفحة, x, y]` بالنقاط، والأصل أسفل اليسار |

```pdfl
check "Codes present" {
  assert codes::detect_barcodes(), "no barcode found in the artwork"
  assert codes::detect_qrcodes(), "the traceability QR code is missing"

  total = codes::count_barcodes()
  assert total == 2, "expected 2 codes (EAN + QR), found #{total}"

  kind = codes::get_barcode_type(1)
  assert kind == "EAN_13", "the main code should be EAN-13, it is #{kind}"

  spot = codes::get_barcode_location(1)
  assert spot.first() == 1, "barcode is not on the cover"
}
```

---

## 7.2 فكّ الترميز والتحقق

| الدالة | الغرض |
|---|---|
| `codes::decode_barcode(n)` | محتوى الرمز رقم n |
| `codes::validate_barcode_checksum(n)` | رقم تحقق GTIN للرمز رقم n |
| `codes::validate_gtin(text)` / `codes::validate_ean(text)` | رقم تحقق GTIN لنصّ |
| `codes::validate_code128()` | صحيحة إن فُكّ ترميز Code 128 بنجاح |

```pdfl
check "Code integrity" {
  code = codes::decode_barcode(1)
  print("code read:", code)

  // رقم GTIN بخانة تحقق خاطئة يُرفض عند الصندوق
  assert codes::validate_barcode_checksum(1),
    "invalid check digit in code #{code}"
  assert code.starts_with("789"),
    "GTIN is not Brazilian (should start with 789): #{code}"

  // مطابقة الرقم المطبوع تحت الرمز
  printed = text::extract_from_region(1, region(400, 20, 180, 15)).trim()
  assert codes::validate_ean(printed),
    "the printed number is not a valid GTIN: #{printed}"
}
```

---

## 7.3 المطابقات المتقاطعة

| الدالة | الغرض |
|---|---|
| `codes::compare_barcode_with_text()` | صحيحة إن ظهر محتوى كل رمز في النص |
| `codes::validate_barcode_format(regex)` | صحيحة إن طابقت كل المحتويات التعبير |
| `codes::validate_barcode_position(region)` أو `(x0, y0, x1, y1)` | صحيحة إن وقعت كل الرموز داخل المنطقة |

تلتقط `compare_barcode_with_text` أغلى أخطاء القطاع: الرمز يشير إلى منتج والنص
المطبوع يذكر منتجًا آخر.

```pdfl
check "Cross-checks" {
  assert codes::compare_barcode_with_text(),
    "the barcode number does not appear in the text — artwork with swapped data?"

  // EAN-13 وحده مسموح: 13 رقمًا بالضبط
  assert codes::validate_barcode_format("^\d{13}$"),
    "there is a code outside the EAN-13 pattern"

  // المنطقة المسمّاة أوضح في القراءة
  area = region(400, 20, 180, 80, "barcode area")
  assert codes::validate_barcode_position(area),
    "barcode outside the reserved area of the packaging"
}
```

---

## 7.4 مثال كامل

```pdfl
// package_insert.pdfl — تدقيق رموز نشرة دواء
// الاستعمال: pdfl run package_insert.pdfl insert.pdf
profile "medicine-insert" {

  check "Codes present" tags: ["codes"] {
    assert codes::detect_barcodes(), "insert has no barcode"
    assert codes::count_barcodes() >= 1, "expected at least the product EAN"
  }

  check "Code integrity" tags: ["codes"] {
    code = codes::decode_barcode(1)
    kind = codes::get_barcode_type(1)
    print("code:", kind, "=", code)

    assert kind == "EAN_13", "main code is not EAN-13 (it is #{kind})"
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"
    assert code.starts_with("789"), "GTIN is not Brazilian: #{code}"
  }

  check "Cross-check with the text" tags: ["codes", "critical"] {
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Position in the artwork" tags: ["codes", "layout"] {
    reserved = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(reserved),
      "code outside the reserved area — risk of being trimmed off"
  }

  check "Cross-check with the product database" tags: ["data"] {
    // بالاشتراك مع data:: — انظر الفصل 9
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} is not in the approved product database"
    print("product:", product.get(2))
  }
}
```

---

[← `prepress::`](06-prepress.md) · [الفهرس](README.md) · [التالي: `fix::` →](08-fix.md)
