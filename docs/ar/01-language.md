# 1. لغة PDFLang

[← الفهرس](README.md) · [التالي: أنواع المستند →](02-types.md)

صُمِّمت PDFLang لكي يقرأها من لا يكتبون البرامج. لا أصناف، ولا وراثة، ولا
تصريح بالأنواع، ولا فواصل منقوطة. النص البرمجي مجموعة فحوص مكتوبة بما يشبه
اللغة الطبيعية.

---

## 1.1 بنية النص البرمجي

```pdfl
// التعليق يبدأ بشرطتين مائلتين ويمتد إلى آخر السطر.

profile "profile-name" {         // profile اختياري: يسمّي المجموعة ويجمعها،
                                 // ويظهر اسمه في التقرير.

  const LIMIT = 300%             // الثوابت: بالأحرف الكبيرة عرفًا

  check "Check Name" {           // كل فحص يصبح قسمًا في التقرير
    require doc.page_count > 0   // تدقيق واحد
  }

  check "Another Check" {        // يمكن كتابة أي عدد من الفحوص
    require doc.title != ""
  }
}
```

يمكن الاستغناء عن `profile` — فالنص البرمجي قد يكون سلسلة فحوص فحسب:

```pdfl
check "Simple" {
  require doc.page_count > 0
}
```

### وسوم الفحوص

تُستعمل الوسوم لتصنيف الفحوص وتصفيتها في التقرير:

```pdfl
check "Ink within limit" tags: ["prepress", "colors"] {
  require prepress::validate_tac_limits(300)
}
```

### درجة خطورة الفحص

افتراضيًا يُعدّ الفحص الفاشل **خطأ** وينتهي التشغيل بالرمز 2. ويمكن للفحص أن
يُعلن نفسه إرشاديًا:

```pdfl
check "دقة الصور" severity: warning {
  require !visual::detect_low_resolution(300)
}
```

ثلاث درجات: `error` (الافتراضية) و`warning` و`info`. لا يُفشل التحذير ولا
المعلومة التشغيل — بل ينتهيان بالرمز 1 و0 — إلا مع `--fail-on warning`، وبها
يقرّر التكامل المستمر مقدار الصرامة دون تعديل النص البرمجي.

ويجوز ورود `tags:` و`severity:` بأي ترتيب.

> أما خطأ التشغيل داخل الفحص — اسم متغيّر مكتوب خطأً، ملف مفقود — فيبقى خطأً
> مهما أعلن الفحص. فالنص البرمجي المعطوب ليس إرشاديًا.

---

## 1.2 طريقتان للتدقيق

كل تدقيق يُكتب بـ `require` أو `assert`. والفارق الوحيد هو الرسالة التي تظهر في
التقرير عند الإخفاق.

```pdfl
check "Comparing both forms" {

  // require: الرسالة تُصاغ من التعبير نفسه.
  // عند الإخفاق يعرض التقرير:
  //   "requirement not met: doc.page_count > 0"
  require doc.page_count > 0

  // assert: أنت من يكتب الرسالة التي يقرؤها المستلم.
  // عند الإخفاق تظهر كما هي:
  //   "PDF has no title in its metadata"
  assert doc.title != "", "PDF has no title in its metadata"
}
```

**قاعدة عملية:** استعمل `require` حين يكون التعبير واضحًا بذاته، و`assert` حين
يحتاج قارئ التقرير إلى فهم المشكلة دون معرفة النص البرمجي.

### إخفاق واحد لا يوقف بقية الفحوص

```pdfl
check "Three independent validations" {
  assert doc.page_count > 100, "too few pages"    // يخفق
  assert doc.title != "", "no title"              // يُنفَّذ رغم ذلك
  assert doc.author != "", "no author"            // وهذا أيضًا
}
```

يسرد التقرير **كل** المشكلات دفعة واحدة. وهذا مقصود: من يستلم الملف يريد قائمة
التصحيحات كاملة، لا تصحيحًا واحدًا في كل مرة.

والأمر نفسه بين الفحوص — إذا صادف فحصٌ خطأ زمن التنفيذ (متغير غير معروف مثلًا)
تحوّل إلى تشخيص، وتابعت بقية الفحوص عملها.

---

## 1.3 القيم والأنواع

### الأعداد والوحدات

```pdfl
check "Numbers" {
  x = 42          // عدد صحيح
  y = 2.5         // عدد عشري

  // وحدات الطول تُحوَّل إلى نقاط (1 pt = 1/72 بوصة):
  a = 3mm         // 8.5039... pt
  b = 2.5cm       // 70.866... pt
  c = 1in         // 72 pt
  d = 10pt        // 10 pt

  // النسبة المئوية تحتفظ بالقيمة العددية:
  limit = 300%    // 300

  require a < b            // كل شيء بالنقاط، فالمقارنة مباشرة
  require c == 72.0
  require limit == 300
}
```

القدرة على كتابة `3mm` بدل `8.504` هي بيت القصيد: تُقرأ طبيعيةً لمن يفكر
بالمليمترات، والتحويل لا يخطئ.

### النصوص

```pdfl
check "Strings" {
  simple = "نص بسيط"

  // الإدماج: ‎#{...}‎ يُدرج قيمة أي تعبير
  name = "document.pdf"
  message = "Analyzing #{name} with #{doc.page_count} pages"

  // الهروب: \n سطر جديد، \t جدولة، \" علامة اقتباس، \\ شرطة عكسية
  quoted = "he said \"hello\""

  // الشرطة العكسية غير المعروفة تبقى كما هي — وبذلك تُكتب التعابير
  // النمطية دون هروب مزدوج:
  pattern = "\d{3}\.\d{3}\.\d{3}-\d{2}"

  require message.contains("pages")
}
```

### القيم المنطقية وما يُعدّ «صحيحًا»

```pdfl
check "True and false" {
  yes = true
  no = false

  // false و null وحدهما خاطئان. وكل ما عداهما صحيح —
  // بما في ذلك 0 والنص الفارغ والقائمة الفارغة.
  require 0        // ينجح (0 صحيح)
  require ""       // ينجح (النص الفارغ صحيح)

  // لذا للتحقق من المحتوى قارِن صراحةً:
  require doc.title != ""              // صواب
  require doc.pages.length > 0         // صواب
}
```

وهذا مفيد مع الدوال التي تُرجع `null`:

```pdfl
check "Taking advantage of null" {
  description = data::lookup_value("batches.csv", "L2026-08")
  // null خاطئ، فيمكن كتابة هذا مباشرة:
  assert description, "batch not found in the table"
}
```

### القوائم

```pdfl
check "Lists" {
  numbers = [1, 2, 3]
  words = ["a", "b", "c"]
  mixed = [1, "two", true]

  require numbers.length == 3
  require numbers.contains(2)
  require words.join(", ") == "a, b, c"

  // الوصول يبدأ من 1: العنصر الأول هو رقم 1
  require numbers.get(1) == 1
  require numbers.first() == 1
  require numbers.last() == 3
}
```

---

## 1.4 المعاملات

```pdfl
check "Operators" {
  // المقارنة
  require 10 > 5
  require 10 >= 10
  require 3 < 4
  require 3 <= 3
  require "a" == "a"
  require "a" != "b"

  // الحساب
  require 2 + 3 == 5
  require 10 - 4 == 6
  require 3 * 4 == 12
  require 10 / 4 == 2.5        // القسمة غير التامة تعطي عددًا عشريًا
  require 10 / 5 == 2          // القسمة التامة تبقى صحيحة

  // المنطق (تقييم قصير: الطرف الأيمن لا يُقيَّم إلا عند الحاجة)
  require true && true
  require false || true
  require !false

  // الفائدة العملية للتقييم القصير: بلا صفحات لا يُقيَّم الطرف الأيمن،
  // فلا يُخطئ المستند الفارغ.
  require doc.page_count == 0 || doc.pages.first().width > 0
}
```

---

## 1.5 الكتل: التكرار على كل عنصر

الكتلة شيفرة بين قوسين معقوفين، ومعاملاتها بين شرطتين رأسيتين. تُقرأ هكذا:
«لكل صفحة، افعل…».

```pdfl
check "Walking through pages" {

  // each: ينفّذ الكتلة على كل عنصر
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  // each_with_index: يعطي الموضع أيضًا (0، 1، 2…)
  doc.fonts.each_with_index { |font, i|
    print("font", i, ":", font.name)
  }

  // all: صحيح إذا حقّق كل العناصر الشرط
  require doc.fonts.all { |f| f.is_embedded }

  // any: صحيح إذا حقّقه عنصر واحد على الأقل
  require doc.pages.any { |p| p.extract_text() != "" }

  // filter: يُبقي العناصر التي تحقّق الشرط فقط
  blank = doc.pages.filter { |p| p.extract_text() == "" }
  assert blank.length == 0, "#{blank.length} blank page(s)"

  // map: يحوّل كل عنصر إلى قائمة جديدة
  names = doc.fonts.map { |f| f.name }
  print("fonts in use:", names.join(", "))
}
```

تتسلسل الكتل — لكن **في السطر نفسه**: لا سطر جديد قبل النقطة.

```pdfl
check "Chaining" {
  // الخطوط غير المضمَّنة، الأسماء فقط، موصولة بالفواصل
  problems = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  assert problems.length == 0,
    "fonts not embedded: #{problems.join(", ")}"
}
```

وإن طال السطر فقسّمه إلى خطوات مسمّاة بدل كسر السلسلة — فهو أوضح على أي حال:

```pdfl
check "Named steps" {
  loose = doc.fonts.filter { |f| !f.is_embedded }
  names = loose.map { |f| f.name }
  assert names.length == 0, "fonts not embedded: #{names.join(", ")}"
}
```

---

## 1.6 الدوال: تسمية القاعدة

حين يتكرر التدقيق نفسه مرارًا، أعطه اسمًا:

```pdfl
// قيمة الدالة هي قيمة آخر تعبير فيها — لا return.
function is_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}

function exceeds_ink(page, limit) {
  page.tac > limit
}

check "Format and ink" {
  // عندئذٍ يُقرأ الفحص وكأنه جملة
  require doc.pages.all { |p| is_a4(p) }

  doc.pages.each { |page|
    assert !exceeds_ink(page, 300), "page #{page.number} has too much ink"
  }
}
```

قواعد الدوال:

- المعاملات لا توجد إلا داخل الدالة.
- يمكن للدالة أن تستدعي دوالّ أخرى.
- الاستدعاء الذاتي مسموح بحدّ 200 استدعاء (كي لا يوقف نص برمجي جامح العملية).

---

## 1.7 import: إعادة الاستعمال بين الملفات التعريفية

ضع القواعد المشتركة في ملف واستوردها حيث تحتاجها.

`library.pdfl`:

```pdfl
// ثوابت ودوال يتشاركها الفريق
const OFFSET_TAC = 300%
const DEFAULT_BLEED = 3mm

function a4_page(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}
```

`magazine.pdfl`:

```pdfl
// المسار نسبةً إلى «هذا» الملف
import "library.pdfl"

check "Format" {
  // OFFSET_TAC و a4_page أتيا من الاستيراد
  require doc.pages.all { |p| a4_page(p) }
  require prepress::validate_tac_limits(OFFSET_TAC)
}
```

الملف نفسه يُحمَّل **مرة واحدة فقط** ولو استوردته عدة نصوص — فالاستيراد الدائري
لا يوقف شيئًا.

---

## 1.8 rule: التدقيق صفحةً صفحة

الـ `rule` فحص يُنفَّذ مرة لكل صفحة، والصفحة مربوطة سلفًا بالمتغير `page`:

```pdfl
// بلا "on": يُنفَّذ على كل الصفحات
rule "Every page has text" {
  assert page.extract_text().trim() != "",
    "page #{page.number} is blank"
}
```

ومع `on` تختار الصفحات المعنية:

```pdfl
rule "Body pages numbered" on doc.pages.filter { |p| p.number > 2 } {
  footer = region(0, 0, page.width, 60)
  assert text::extract_from_region(page.number, footer) != "",
    "page #{page.number} has no page number in the footer"
}
```

> **ملاحظة نحوية:** إذا انتهى التعبير بعد `on` بخاصية (مثل `on doc.pages`)
> فضعه بين قوسين، وإلا حُسب القوس المعقوف للجسم كتلةً لذلك الاستدعاء:
>
> ```pdfl
> rule "Example" on (doc.pages) {     // القوسان ضروريان
>   require page.width > 0
> }
> ```

---

## 1.9 المتغيرات والنطاق

```pdfl
const GLOBAL = 100          // مرئي في الملف كله

check "Scope" {
  local = 42                // مرئي داخل هذا الفحص فقط

  doc.pages.each { |page|
    inner = page.width      // مرئي داخل هذه الكتلة فقط
    require inner > 0
  }

  require local == 42       // ما زال مرئيًا
  require GLOBAL == 100     // ما زال مرئيًا
}
```

جرى العرف على الأحرف الكبيرة للثوابت والصغيرة للمتغيرات. اللغة لا تفرض ذلك،
لكن الأمثلة والملفات التعريفية المرفقة تلتزم به.

---

## 1.10 رسائل تنفع من يستلم الملف

جودة التقرير رهن الرسائل التي تكتبها. قارِن:

```pdfl
check "Poor messages" {
  require doc.pages.all { |p| p.tac <= 300 }
  // التقرير: "requirement not met: doc.pages.all() { ... }"
  // — المستلم لا يعرف أي صفحة ولا بكم تجاوزت.
}

check "Good messages" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "Page #{page.number}: ink coverage #{page.tac}% (max 300%)"
  }
  // التقرير: "Page 7: ink coverage 324% (max 300%)"
  // — يعرف المشغّل فورًا ما ينبغي تصحيحه.
}
```

وللمعلومات الإضافية التي ليست أخطاءً استعمل `print()`. مخرجها يذهب إلى مخرج
الأخطاء فلا يلوّث التقرير:

```pdfl
check "Context" {
  print("Analyzing", doc.page_count, "pages")
  print("Fonts:", prepress::list_fonts().join(", "))
  require doc.page_count > 0
}
```

---

## 1.11 أخطاء شائعة

| الرسالة | السبب | العلاج |
|---|---|---|
| `expected end of line after statement` | تعليمتان في سطر واحد | تعليمة واحدة لكل سطر |
| `unknown variable: x` | استُعمل قبل الإسناد أو خارج النطاق | صرّح به في المستوى نفسه |
| `unknown function: text::xyz` | اسم خاطئ أو دالة غير موجودة | راجع فصل فضاء الأسماء |
| `fix:: is only available in the 'pdfl fix' command` | استعمال `fix::` مع `pdfl run` | `pdfl fix input.pdf script.pdfl --output out.pdf` |
| `unknown unit: 'kg'` | وحدة غير صالحة | استعمل `pt` أو `mm` أو `cm` أو `in` أو `%` |
| `expected '{' with the rule body` | التعبير بعد `on` ينتهي بخاصية | ضعه بين قوسين |
| `unexpected expression: Dot` | سلسلة قُطعت بسطر جديد | أبقِ `.method` في السطر نفسه أو استعمل متغيرًا وسيطًا |

قبل التنفيذ يستحق هذان الأمران العناء دائمًا:

```bash
pdfl lint my_profile.pdfl    # متغيرات غير مستعملة، فحوص مكرّرة…
pdfl fmt my_profile.pdfl     # تنسيق موحّد
```

---

[← الفهرس](README.md) · [التالي: أنواع المستند →](02-types.md)
