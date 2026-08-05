# 12. وصفات عملية

[← سطر الأوامر](11-cli.md) · [الفهرس](README.md)

حالات كاملة تُنقل كما هي. كل واحدة تحلّ مشكلة حقيقية من الميدان.

---

## 12.1 مطبعة: تدقيق ما قبل الطبع لمجلة أوفست

**المشكلة:** يسلّم العميل ملفه، وقبل حرق الألواح يجب التحقق من الأحبار والخطوط
والصور والفيض. والخطأ الذي يُكتشف لاحقًا يضيّع الطبعة كلها.

`profiles/offset.pdfl`:

```pdfl
profile "offset-magazine" {

  const TAC_LIMIT = 300%       // حدّ الحبر على الورق المطلي
  const BLEED = 3mm            // مقتضى التركيب
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress"] {
    // الـ TAC الدقيق يقرأ الألوان المعلَنة في الملف؛ أما التقدير
    // بالتصيير فيقلّل من الأسود الغني ويفوّت التجاوزات
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMIT,
        "page #{page.number}: #{tac}% ink (limit #{TAC_LIMIT}%)"
    }
  }

  check "Colors" tags: ["prepress"] {
    assert prepress::detect_color_mode() != "RGB",
      "document is in RGB — convert to CMYK"

    spots = prepress::detect_spot_colors()
    assert spots.length == 0, "unquoted special ink: #{spots.join(", ")}"

    assert !prepress::detect_rich_black(),
      "rich black detected — use 0/0/0/100 for text"
  }

  check "Fonts" tags: ["fonts"] {
    loose = prepress::detect_text_substitution()
    assert loose.length == 0,
      "fonts not embedded (text will change at the RIP): #{loose.join(", ")}"
    assert prepress::validate_font_size(6),
      "there is text below 6 pt — illegible once printed"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25),
      "strokes below 0.25 pt disappear in print"
    assert !prepress::detect_hairlines_exact(),
      "there is a stroke with 0 width — set a real thickness"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI (minimum #{MIN_DPI})"
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number}"
    }
  }

  check "Geometry" tags: ["prepress"] {
    assert prepress::validate_trim_box(),
      "no TrimBox — imposition cannot know where to trim"
    assert prepress::validate_bleed_box(), "no BleedBox — no bleed is defined"
    assert prepress::check_page_geometry(BLEED),
      "bleed smaller than 3 mm on some page"
  }
}
```

**عند الاستقبال:**

```bash
# تقرير HTML يُعاد إلى العميل
pdfl run profiles/offset.pdfl client.pdf --output html --output-file report.html
```

**كمجلد مراقَب:** يضع المشغّل الملف فيظهر التقرير بجواره.

```bash
pdfl watch inbox/ --script profiles/offset.pdfl \
  --output-dir reports/ --report html
```

---

## 12.2 دار نشر قانونية: تدقيق عقد قبل النشر

**المشكلة:** يجب أن تحمل العقود والوثائق البنود الإلزامية، وألا يبقى فيها نص
مسودّة، وألا تكشف بيانات شخصية، وأن يظل نصها قابلًا للبحث.

`profiles/legal.pdfl`:

```pdfl
profile "standard-contract" {

  check "Mandatory clauses" tags: ["legal"] {
    // مسرد تديره الإدارة القانونية
    missing = data::validate_against_reference("terms/clauses.txt")
    assert missing.length == 0, "missing clauses: #{missing.join("; ")}"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "document marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("X{3,}"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    // أرقام الضريبة لا تُحتسب إلا بخانة تحقق صحيحة،
    // فأرقام الأمثلة لا تثير إنذارًا كاذبًا
    found = text::detect_personal_data()
    assert found.length == 0, "personal data in the document: #{found.join("; ")}"
  }

  check "Numbering and initials" tags: ["legal"] {
    doc.pages.each { |page|
      footer = region(0, 0, page.width, 60, "footer")
      content = text::extract_from_region(page.number, footer).trim()
      assert content != "",
        "page #{page.number} has no numbering/initials in the footer"
    }
  }

  check "Searchable text" tags: ["accessibility"] {
    assert !text::detect_rasterized_text(),
      "there are scanned pages — text cannot be searched or read by screen readers"
  }
}
```

---

## 12.3 مختبر أدوية: نشرة برمز الدفعة

**المشكلة:** يجب أن تحمل النشرة النصوص التي تطلبها الجهة التنظيمية، وأن يشير
الباركود إلى المنتج الصحيح. وتبديل الرموز بين المنتجات أغلى أخطاء هذا القطاع.

`profiles/insert.pdfl`:

```pdfl
profile "regulated-insert" {

  check "Mandatory texts" tags: ["regulatory"] {
    missing = data::validate_against_reference("databases/regulatory_texts.txt")
    assert missing.length == 0, "mandatory texts missing: #{missing.join("; ")}"
  }

  check "Legibility" tags: ["regulatory"] {
    assert prepress::validate_font_size(6), "there is text below 6 pt"
  }

  check "Barcode" tags: ["codes", "critical"] {
    assert codes::detect_barcodes(), "insert has no barcode"

    code = codes::decode_barcode(1)
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"

    // هذا الفحص يلتقط أغلى خطأ:
    // رمز منتج مع نص منتج آخر
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Approved product" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} is not in the product database"

    name = product.get(2)
    assert text::require_text(name),
      "the name '#{name}' does not appear on the insert"
    print("product verified:", name)
  }

  check "Code position" tags: ["layout"] {
    area = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(area),
      "code outside the reserved area — risk of being trimmed off"
  }
}
```

```bash
PDFL_DATA_DIR=./databases pdfl run profiles/insert.pdfl insert_v3.pdf
```

---

## 12.4 الموافقة: المقارنة بالنسخة المعتمدة

**المشكلة:** اعتمد العميل النسخة الأولى. ثم تصل الثانية بقول «غيّرنا كلمة
واحدة». وتصديق ذلك مكلف.

```bash
# HTML يُظهر ما تغيّر فعلًا
pdfl compare approved/catalogue_v1.pdf received/catalogue_v2.pdf \
  --normalize --ignore-dates \
  --output html --output-file differences.html

echo "exit: $?"   # 0 متطابقان · 1 بيانات وصفية فقط · 2 تغيّر المحتوى
```

وللتحقق من **الشكل** أيضًا لا من النص وحده:

```pdfl
// profiles/fidelity.pdfl
profile "visual-fidelity" {

  const APPROVED = "approved/catalogue_v1.pdf"

  check "Pages visually identical" tags: ["approval"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APPROVED)
      assert ssim > 0.99,
        "page #{page.number} changed visually (SSIM #{ssim}, #{visual::pixel_diff(page.number, APPROVED)}% of pixels)"
    }
  }

  check "No image replaced" tags: ["approval"] {
    doc.pages.each { |page|
      assert !visual::detect_image_replacement(page.number, APPROVED),
        "page #{page.number}: image swapped compared to the approved version"
    }
  }
}
```

---

## 12.5 CI/CD: التدقيق بالجملة

**المشكلة:** كل ملف يدخل المستودع يجب أن يجتاز تدقيق ما قبل الطبع، دون أن
يشغّله أحد يدويًا.

`.github/workflows/preflight.yml`:

```yaml
name: PDF preflight

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install pdfl
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # رمز Actions التلقائي، دون إعداد
        run: |
          gh release download --repo kotarokikuchi/pdflang \
            --pattern 'pdfl_*_amd64.deb'
          sudo dpkg -i pdfl_*_amd64.deb

      - name: Check the scripts themselves
        run: |
          for f in profiles/*.pdfl; do
            pdfl lint "$f"
            pdfl fmt "$f" --check
          done

      - name: Preflight every PDF
        run: |
          pdfl watch files/ --script profiles/offset.pdfl \
            --output-dir reports/ --once

      - name: Publish the reports
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: reports
          path: reports/
```

---

## 12.6 تهيئة ملف ناشر للمطبعة

```pdfl
// profiles/prepare.pdfl
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask for the open version"
}

// أُطر الإنتاج التي لم يضبطها الناشر
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// التنظيف
fix::remove_annotations()      // تعليقات المراجعة
fix::remove_attachments()      // مرفقات لا تزيد الملف إلا ثقلًا
fix::flatten_layers()          // يمنع تشغيل طبقة عن طريق الخطأ
fix::remove_unused_resources()
```

```bash
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf --dry-run  # للتحقق
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf            # للتطبيق
pdfl run profiles/offset.pdfl print.pdf                                    # للتدقيق
```

---

## 12.7 توزيع ملف تعريفي على الفريق

**المشكلة:** خمسة أجهزة يجب أن تستعمل الملف التعريفي نفسه والبيانات نفسها تمامًا،
دون أن يعبث بها أحد.

```bash
# على الجهاز الذي يصون الملف التعريفي
pdfl pack profiles/ --name print-profile --version 1.2.0

# على أجهزة الإنتاج
pdfl add print-profile.pdflpkg
# يثبّت في ./pdfl_profiles/print-profile@1.2.0/ مع التحقق من كل بصمة

pdfl run pdfl_profiles/print-profile@1.2.0/offset.pdfl file.pdf
```

وإن عُدِّلت الحزمة في الطريق **رفض** `add` التثبيت.

---

## 12.8 استقصاء ملف فيه مشكلة

خطوات عملية حين لا تعرف من أين أتت المشكلة:

```bash
# 1. صورة عامة في ثوانٍ
pdfl inspect suspect.pdf

# 2. نص برمجي للاستقصاء، بـ print() فقط
cat > investigate.pdfl <<'EOF'
check "X-ray" {
  print("exact TAC:", prepress::calculate_exact_tac(), "%")
  print("estimated TAC:", prepress::calculate_tac(), "%")
  print("spots:", prepress::detect_spot_colors().join(", "))
  print("rich black?", prepress::detect_rich_black())
  print("overprint ok?", prepress::validate_overprint_settings())
  print("loose fonts:", prepress::detect_text_substitution().join(", "))

  doc.images.each { |img|
    print("image page", img.page_number, ":", img.width, "x", img.height,
          "@", round(img.dpi), "DPI", img.color_space)
  }
}
EOF

pdfl run investigate.pdfl suspect.pdf > /dev/null
# print() يكتب على مخرج الأخطاء، فيمكن رمي التقرير
# والاكتفاء بنتائج الاستقصاء
```

---

[← سطر الأوامر](11-cli.md) · [الفهرس](README.md)
