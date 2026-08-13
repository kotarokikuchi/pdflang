# 9. فضاء الأسماء `data::` — البيانات الخارجية

[← `fix::`](08-fix.md) · [الفهرس](README.md) · [التالي: المكتبة القياسية →](10-stdlib.md)

8 دوال لمطابقة محتوى ملف PDF بقوائمك وجداولك. وكل شيء يجري محليًا: لا تخرج أي
بيانات.

---

## 9.1 أين توضع الملفات

تقبل المسارد ومجموعات البيانات مسارًا **نسبةً إلى مجلد التنفيذ**:

```pdfl
data::load_glossary("terms/legal.txt")
data::load_dataset("data/batches.csv")
```

أما جداول البحث (`query_gtin` و`query_medicamento` و`query_postal_code`) فتستعمل
أسماء ملفات ثابتة وتبحث عنها بهذا الترتيب:

1. `$PDFL_DATA_DIR` (متغير البيئة)
2. `./dados/`
3. `./`
4. الملفات التعريفية المثبَّتة بـ `pdfl add` (`pdfl_profiles/*/dados/`)
5. مجلد ملف PDF المُحلَّل

```bash
PDFL_DATA_DIR=/opt/databases pdfl run profile.pdfl document.pdf
```

وإن لم يُعثر على شيء دلّت رسالة الخطأ على مكان وضع الملف. ولتوزيع البيانات مع
الملف التعريفي استعمل `pdfl pack` ([الفصل 11](11-cli.md)).

---

## 9.2 المسارد ومجموعات البيانات

| الدالة | الغرض |
|---|---|
| `data::load_glossary(file)` | قائمة مصطلحات (واحد في كل سطر، و`#` تعليق) |
| `data::validate_against_reference(file)` | قائمة المصطلحات **الغائبة** عن المستند |
| `data::load_dataset(file)` | يقرأ ملف CSV قائمةَ صفوف |
| `data::lookup_value(file, key)` | العمود الثاني من الصف الذي عموده الأول هو المفتاح (`null` إن لم يوجد) |

المقارنة تتجاهل حالة الأحرف والمسافات.

`terms/required.txt`:

```
# مصطلحات يجب أن تحتويها كل وثيقة تأمين
waiting period
covered benefits
general conditions
```

```pdfl
check "Glossary and dataset" {
  terms = data::load_glossary("terms/required.txt")
  print("terms in the glossary:", terms.length)

  // أبسط الاستعمالات
  missing = data::validate_against_reference("terms/required.txt")
  assert missing.length == 0,
    "clauses missing from the policy: #{missing.join("; ")}"

  rows = data::load_dataset("data/batches.csv")
  print("columns:", rows.first().join(" | "))   // الصف الأول هو الترويسة
  print("records:", rows.length - 1)

  // null خاطئ، فيمكن كتابة التدقيق مباشرة
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  description = data::lookup_value("data/batches.csv", batch)
  assert description, "batch #{batch} is not in the approved list"
}
```

### مجموعات البيانات بصيغة JSON

الملف المنتهي بـ `.json` يُقرأ على أنه JSON — في `load_dataset` و`lookup_value`
سواء. وتُقبل صورتان، لأنهما الصورتان اللتان تُكتب بهما مجموعة البيانات فعلًا.

مصفوفة من المصفوفات هي الصفوف كما هي:

```json
[["batch", "description"],
 ["L2026-08", "Approved batch August/2026"]]
```

ومصفوفة من الكائنات تصير صفَّ عناوين وصفًّا لكل كائن. وترتيب الأعمدة هو الترتيب
الذي كتبه **أول** كائن، لا الترتيب الأبجدي، فيبقى المفتاح الأول هو ما يبحث عنه
`lookup_value`:

```json
[{"batch": "L2026-08", "description": "Approved batch August/2026"},
 {"batch": "L2026-09", "description": "Approved batch September/2026"}]
```

والمفتاح الغائب من كائن لاحق يترك **خلية فارغة**، لا صفًّا مُزاحًا: فالفراغ يظهر
في التقرير، والإزاحة لا تظهر. والأرقام تحتفظ بالأرقام التي كُتبت بها، و`null`
خلية فارغة — وهو ما يعنيه الحقل الفارغ في CSV.

وخلط الصورتين في ملف واحد خطأ، والخطأ يذكر رقم الصف.

---

## 9.3 جداول البحث

ملفات بأسماء ثابتة يُبحث عنها بالترتيب المذكور في 9.1. وتُرجع **الصف كاملًا**
قائمةً، أو `null` إن لم يوجد شيء.

| الدالة | ملف المرجع | الغرض |
|---|---|---|
| `data::query_gtin(code)` | `gtin.csv` | البحث بالـ GTIN (الترقيم لا يهم) |
| `data::query_medicamento(reg_or_name)` | `medicamentos.csv` | برقم التسجيل أو بجزء من الاسم |
| `data::query_postal_code(code)` | `ceps.csv` | بالرمز البريدي (8 أرقام) |
| `data::validate_address(code, "fragment")` | `ceps.csv` | هل يحتوي عنوان هذا الرمز على الجزء؟ |

`dados/gtin.csv`:

```csv
gtin,description,manufacturer
7891234567895,Dipyrone 500mg 20 tablets,Example Labs
```

```pdfl
check "Lookup tables" {
  // المطابقة مع الباركود المقروء على العبوة
  code = codes::decode_barcode(1)
  product = data::query_gtin(code)
  assert product, "GTIN #{code} is not in the product database"
  print("product:", product.get(2), "| manufacturer:", product.get(3))

  // بيانات الدواء عبر رقم التسجيل
  registration = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
  medicine = data::query_medicamento(registration)
  assert medicine, "registration #{registration} not found"

  // الدواء الموصوف يقتضي العبارة الإلزامية
  band = medicine.get(4)
  assert band != "prescription" || text::require_text("PRESCRIPTION ONLY"),
    "prescription medicine without the mandatory text"

  // هل يوافق العنوان المطبوع الرمز البريدي؟
  assert data::validate_address("01310100", "Avenida Paulista"),
    "printed address does not match the given postal code"
}
```

---

## 9.4 مثال كامل

```pdfl
// insert_with_databases.pdfl — المطابقة ببيانات محلية
// الاستعمال: PDFL_DATA_DIR=./databases pdfl run insert_with_databases.pdfl insert.pdf
profile "insert-with-references" {

  check "Mandatory regulatory terms" tags: ["glossary"] {
    missing = data::validate_against_reference("databases/regulatory_terms.txt")
    assert missing.length == 0, "mandatory texts missing: #{missing.join("; ")}"
  }

  check "Product in the database" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} not approved"

    // الاسم المسجَّل يجب أن يظهر في المطبوع
    name = product.get(2)
    assert text::require_text(name),
      "the name '#{name}' from the database does not appear on the insert"
    print("product verified:", name)
  }

  check "Registration and band" tags: ["regulatory"] {
    registration = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
    med = data::query_medicamento(registration)
    assert med, "registration #{registration} not found"
    assert med.get(4) != "prescription" || text::require_text("PRESCRIPTION ONLY"),
      "prescription band requires the prescription notice"
  }

  check "Manufacturer address" tags: ["data"] {
    assert data::validate_address("01310100", "Avenida Paulista"),
      "manufacturer address does not match the postal code"
  }
}
```

---

[← `fix::`](08-fix.md) · [الفهرس](README.md) · [التالي: المكتبة القياسية →](10-stdlib.md)
