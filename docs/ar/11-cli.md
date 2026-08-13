# 11. سطر الأوامر

[← المكتبة القياسية](10-stdlib.md) · [الفهرس](README.md) · [التالي: وصفات عملية →](12-recipes.md)

اثنا عشر أمرًا: أربعة لملفات PDF، وخمسة للنصوص البرمجية، واثنان للتوزيع، وواحد
للصدفة.

| الأمر | الغرض |
|---|---|
| [`run`](#pdfl-run) | يدقّق ملف PDF بنص برمجي |
| [`compare`](#pdfl-compare) | يقارن نسختين |
| [`watch`](#pdfl-watch) | يراقب مجلدًا ويدقّق ما يصل إليه |
| [`fix`](#pdfl-fix) | يطبّق تعديلات ويحفظ ملف PDF جديدًا |
| [`inspect`](#pdfl-inspect) | نظرة سريعة على ملف PDF |
| [`lint`](#pdfl-lint) | يحلّل نصًا برمجيًا دون تنفيذه |
| [`fmt`](#pdfl-fmt) | ينسّق نصًا برمجيًا |
| [`test`](#pdfl-test) | يشغّل نصًّا على مجلد من ملفات PDF ويقارن كل تقرير |
| [`doc`](#pdfl-doc) | يولّد توثيق نص برمجي |
| [`pack`](#pdfl-pack) | يحزم الملفات التعريفية والبيانات |
| [`add`](#pdfl-add) | يثبّت حزمة |
| [`completions`](#pdfl-completions) | يطبع نصّ الإكمال التلقائي لصدفتك |

---

## رموز الخروج

مشتركة بين كل الأوامر التي تدقّق.

| الرمز | المعنى |
|---|---|
| `0` | نجح كل شيء |
| `1` | تحذيرات فقط |
| `2` | أخطاء تدقيق |
| `3` | خطأ نحوي في النص البرمجي |
| `10` | تعذّرت قراءة المستند أو كتابة ملف — ولم يصدر أي حكم |

```bash
pdfl run profile.pdfl file.pdf > report.json
case $? in
  0) echo "approved" ;;
  1) echo "approved with warnings" ;;
  2) echo "rejected — see report.json" ;;
  3) echo "error in the validation script" ;;
esac
```

---

## خيارات عامة

| الخيار | الغرض |
|---|---|
| `--quiet` | يُسكت رسائل التقدّم والتأكيد على stderr |

يعمل `--quiet` قبل الأمر الفرعي أو بعده، ومع كل واحد منها. وهو يزيل السطور التي
يريدها الإنسان ولا يريدها خطّ الإنتاج — `report saved to …` و`watching …` ونتيجة
كل ملف في `watch`. لكنه **لا** يزيل الأخطاء: التشغيلة الصامتة إذا أخفقت تظلّ
تقول لماذا.

كما أنه لا يُسكت `print()`. فتلك مخرجات النص البرمجي نفسه، وابتلاعها يغيّر ما
يفعله. أعِد توجيه stderr إن أردت التخلّص منها.

و`--quiet` يغلب `--verbose`.

---

## `pdfl run`

يدقّق ملف PDF بنص برمجي.

```bash
pdfl run <script.pdfl> <input.pdf> [options]
```

| الخيار | الافتراضي | الغرض |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | صيغة التقرير |
| `--output-file <file>` | — | يكتب في ملف بدل المخرج القياسي |
| `--fail-on error\|warning` | `error` | مع `warning` يعطي التحذير أيضًا الرمز 2 |
| `--verbose` | — | معلومات إضافية على مخرج الأخطاء |
| `--var الاسم=القيمة` | — | قيمة يقرأها النص البرمجي بوصفها `vars.الاسم`؛ ويجوز تكرارها |
| `--tags TAG` | — | لا يشغّل إلا الفحوص الحاملة لهذه السمة؛ ويجوز تكرارها. والسمة التي لا يحملها أي فحص خطأ، لا نجاحًا فارغًا |

```bash
pdfl run prepress.pdfl magazine.pdf                                    # JSON في الطرفية
pdfl run prepress.pdfl magazine.pdf --output html --output-file report.html
pdfl run prepress.pdfl magazine.pdf --output pdf --output-file report.pdf
pdfl run prepress.pdfl magazine.pdf --output csv --output-file findings.csv
pdfl run prepress.pdfl magazine.pdf --fail-on warning                  # الوضع الصارم
```

### تقرير JSON

```json
{
  "schema_version": 1,
  "script_name": "prepress.pdfl",
  "input_file": "magazine.pdf",
  "profile": "offset-magazine",
  "status": "FAIL",
  "total_pages_analyzed": 120,
  "error_count": 2,
  "warning_count": 0,
  "info_count": 0,
  "diagnostics": [
    {
      "id": "PDFL-093751a2",
      "severity": "error",
      "check_name": "Ink coverage",
      "message": "page 7: 324% ink (limit 300%)",
      "line": 12
    }
  ],
  "checks_run": ["Ink coverage", "Fonts", "Bleed"]
}
```

ملف PDF نفسه مع النص البرمجي نفسه يعطي دائمًا **تقريرًا متطابقًا بايتًا بايت**:
يمكن حفظه في نظام الإصدارات ومقارنة الفروق في التكامل المستمر.

`schema_version` هو المفتاح الأول، ليقرر المستهلك قبل أن يحلّل ما بعده. ولا
يرتفع إلا إذا كان قارئ المخرَج السابق سينكسر؛ وإضافة حقل لا ترفعه.

### SARIF وJUnit

صيغتان إضافيتان، لتظهر النتيجة حيث ينظر الفريق أصلًا، لا في سجلّ لا يفتحه أحد.

```bash
# GitHub code scanning: تتحوّل الملاحظات إلى تعليقات على طلب السحب
pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif

# لوحة الاختبارات في أي تكامل مستمر: اختبار لكل فحص، بما فيها الناجحة
pdfl run prepress.pdfl magazine.pdf --output junit --output-file pdfl.xml
```

في SARIF تُربط الملاحظة بـ**النص البرمجي** لا بملف PDF: فالسطر الذي نعرفه هو
سطر الفحص، وملف PDF غالبًا أثرٌ عابر في خطّ التكامل لا ملفًّا في المستودع —
والإشارة إليه تعني التعليق على مسار غير موجود. أما الملف المفحوص فيسافر في
`properties.inputFile`، ومعرّف التشخيص في `partialFingerprints`، وهو ما يجعل
GitHub يتعرّف على ملاحظة رآها من قبل بدل أن يفتحها من جديد في كل تشغيلة.

وفي JUnit يصير كل فحص جرى حالة اختبار، بما في ذلك ما لم يجد شيئًا. فالصيغة التي
تسرد الإخفاقات وحدها تصف تشغيلة نظيفة بأنها صفر اختبارات، ويقرأ التكامل المستمر
ذلك على أنه تشغيلة لم تحدث. وملاحظة من نوع `info` لا تُسقِط حالتها، بل تُكتب في
`<system-out>`.

```yaml
- name: Preflight
  run: pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif
  # الرمز 2 يعني ملفًا مرفوضًا، والرفع يجب أن يتم على كل حال
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: pdfl.sarif
```

---

## `pdfl compare`

يقارن نسختين: النص والبنية والبيانات الوصفية.

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| الخيار | الافتراضي | الغرض |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | الصيغة |
| `--output-file <file>` | — | يكتب في ملف |
| `--normalize` | — | يتجاهل حالة الأحرف والمسافات |
| `--ignore-dates` | — | يحجب التواريخ قبل المقارنة |
| `--similarity-threshold <0-100>` | `100` | أدنى تشابه مقبول |

```bash
pdfl compare approved_v1.pdf new_v2.pdf --normalize --ignore-dates

# يسمح بفارق حتى 1 %، وما دونه خطأ
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file diff.html
```

### كيف يعمل

- تُحاذى الصفحات **بالمحتوى** لا بالرقم: فإدراج صفحة في الوسط لا يجعل كل ما
  بعدها فروقًا. ويعمل على مستندات تتجاوز ألف صفحة.
- يحصل كل زوج على درجة تشابه وعيّنة من الأسطر المتغيرة (`-` محذوف، `+` مضاف).
- تغيّر البيانات الوصفية **تحذير**؛ وتغيّر النص دون العتبة **خطأ**، وفوقها
  **تحذير**.
- الدرجة الإجمالية في الحقل `similarity` من التقرير.

```
page 4 → 4: similarity 97.8% | -original title | +revised title
```

---

## `pdfl watch`

يراقب مجلدًا ويدقّق كل ملف PDF يصل أو يتغيّر.

```bash
pdfl watch <folder> --script <script.pdfl> [options]
```

| الخيار | الافتراضي | الغرض |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | أي الملفات تُعالَج |
| `--exclude <glob>` | — | أيها يُستثنى |
| `--output-dir <folder>` | بجوار ملف PDF | أين تُكتب التقارير |
| `--depth <n>` | `1` | عمق المجلدات الفرعية |
| `--debounce <ms>` | `1000` | انتظار استقرار الملف |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | صيغة التقارير |
| `--fail-fast` | — | يتوقف عند أول خطأ |
| `--jobs <n>` | `1` | عدد الملفات التي تُفحص معًا؛ و`0` يعني واحدًا لكل معالج |
| `--once` | — | يعالج الموجود ثم يخرج |

```bash
# مجلد استلام في مطبعة، بلا انقطاع
pdfl watch inbox/ --script preflight.pdfl --output-dir reports/ --report html

# تشغيل دفعي للتكامل المستمر: يخرج بأسوأ رمز صادفه
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

ويسري `--jobs` على كل ما على الجولة أن تنجزه، في الدفعات وفي موجة الوصول سواء.
فكل ملف يفحصه `pdfl` في عملية خاصة به — للسبب نفسه في `pdfl test` — وهذه العملية
هي التي تصوغ التقارير، فالملف المكتوب واحد مهما كانت قيمة `--jobs`. في ثمانية
ملفات من 41 صفحة: 9.5 ثانية عند `--jobs 1`، و1.2 ثانية عند `--jobs 0`.

ومع `--fail-fast` لا يبدأ ملف جديد بعد أن يُخفق واحد؛ أما ما بدأ فعلًا فيُكمل،
لأن قتله يترك تقارير نصف مكتوبة. وتُكتب التقارير بترتيب العثور على الملفات، فتطبع
الدفعة السطور نفسها مهما كان عدد ما جرى معًا.

الـ **debounce** موجود لأن الملفات الكبيرة تصل قطعًا: فلا يُعالَج إلا ملف كفّ
عن التغيّر، وبذلك لا يُقرأ ملف PDF نصف مكتوب.

تُكتب التقارير باسم `<name>.report.json` (أو `.csv` أو `.html` أو `.pdf`).

---

## `pdfl fix`

يطبّق عمليات `fix::` ويحفظ ملف PDF جديدًا. التفاصيل في [الفصل 8](08-fix.md).

```bash
pdfl fix <input.pdf> <script.pdfl> --output <output.pdf> [options]
```

| الخيار | الافتراضي | الغرض |
|---|---|---|
| `--output <ملف>` | — | ملف PDF الناتج (إلزامي) |
| `--dry-run` | — | يسرد العمليات دون أن يحفظ |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | صيغة التقرير |
| `--report-file <ملف>` | — | يكتب التقرير في ملف |

```bash
pdfl fix original.pdf normalize.pdfl --output out.pdf --dry-run  # للمعاينة فقط
pdfl fix original.pdf normalize.pdfl --output fixed.pdf          # للتطبيق
```

---

## `pdfl inspect`

نظرة عامة على ملف PDF بلا نص برمجي.

```bash
pdfl inspect <file.pdf>
```

يعطي `--json` الملخّص نفسه بصيغة بيانات.

```
File:     magazine.pdf
Size:     26 KB (27284713 bytes)
SHA-256:  af1029842e5bfeae338ead82fb449ef851be742b1d63117c12596e3ea123a616

Pages:    120
Page size: 496 x 709 pt
Boxes:    MediaBox, TrimBox, BleedBox

Metadata:
  Title: Example Magazine
  Creator: Adobe InDesign 19.3

Fonts:    26
  ABCDEF+Helvetica — embedded
  Arial — NOT embedded
Images:   81 (minimum DPI 136, spaces: DeviceCMYK, Indexed)
Max. estimated TAC: 300% (RGB render approximation)

Warnings:
  ! there are non-embedded fonts
  ! 3 image(s) below 300 DPI
```

أول أمر يُشغَّل عند وصول ملف جديد: في ثوانٍ تعرف هل يستحق الفتح.

---

## `pdfl lint`

يحلّل نصًا برمجيًا دون تنفيذه ويبلّغ عن مشكلات الجودة.

```bash
pdfl lint <script.pdfl>
```

يعطي `--json` التحذيرات نفسها بصيغة بيانات.

ما يكشفه:

- المتغيرات ومعاملات الكتل والدوال المصرَّح بها و**غير المستعملة قط**
  (سبقها بـ `_` لكتم التحذير: `_page`)
- الفحوص **المكرّرة** أو **الفارغة**
- فضاءات الأسماء غير المعروفة (`text::` و`struct::` و`visual::` و`prepress::`
  و`codes::` و`fix::` و`data::`)
- `assert` / `require` خارج أي فحص
- استعمال `fix::` (وهو لا يعمل إلا مع `pdfl fix`)

```bash
$ pdfl lint profile.pdfl
profile.pdfl: warning: variable 'LIMIT' declared and never used
profile.pdfl: warning: check "Fonts" declared 2 times
```

وعند وجود تحذيرات يكون رمز الخروج `1` — صالح للتكامل المستمر.

---

## `pdfl fmt`

ينسّق النص البرمجي: إزاحة بمسافتين، ومسافات متسقة، وضغط الأسطر الفارغة.
وتبقى التعليقات والوحدات (`3mm` تظل `3mm`) كما هي.

```bash
pdfl fmt <script.pdfl>            # ينسّق في مكانه
pdfl fmt <script.pdfl> --check    # لا يغيّر شيئًا؛ الرمز 1 إن لم يكن منسَّقًا
```

```bash
# فرض معيار الفريق في التكامل المستمر
for f in profiles/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

يولّد التوثيق من النص البرمجي نفسه.

```bash
pdfl doc <script.pdfl> [--output markdown|html|json]
```

ويُخرج: الملف التعريفي، وجدول الثوابت، والدوال، والاستيرادات، ولكل فحص وسومه
وما يدقّقه (رسائل `assert` تصير الأوصاف).

```bash
pdfl doc prepress.pdfl > docs/prepress-profile.md
pdfl doc prepress.pdfl --output html > profile.html
```

وهو المُخرَج الذي يشرح لمسؤول الإنتاج الذي لا يقرأ الشيفرة ما الذي يدقّقه الملف
التعريفي.

---

## `pdfl pack`

يحزم النصوص البرمجية والبيانات في ملف `.pdflpkg` قابل للتوزيع.

```bash
pdfl pack <folder> [--name <name>] [--version <version>] [--output <file>]
```

يجمع تكراريًا ملفات `.pdfl` و`.csv` و`.txt` و`.json` من المجلد، ويرفق
`manifest.json` يسجّل بصمة SHA-256 لكل ملف. والحزم حتمي: المجلد نفسه يُنتج
البايتات نفسها.

أما ملفات الجداول (`.xlsx` و`.xls` و`.ods`) فلا تُحزم، و`pack` يقول أي ملف ترك.
إذ لا تفتحها أي دالة في `data::`، فحزمها يعني تسليم حزمة تُثبَّت بلا مشكلة ثم
تخفق عند أول بحث.

```bash
pdfl pack profiles/print-shop --name print-profile --version 1.0.0
```

---

## `pdfl add`

يثبّت حزمة محلية مع التحقق من بصمات البيان.

```bash
pdfl add print-profile.pdflpkg
# يثبّت في ./pdfl_profiles/print-profile@1.0.0/

pdfl run pdfl_profiles/print-profile@1.0.0/prepress.pdfl file.pdf
```

وإن لم تطابق بصمة أي ملف ما هو مسجَّل **رُفض التثبيت** — فالحزمة التالفة أو
المعبوث بها لا تدخل.

> المستودعات البعيدة والتواقيع الرقمية ليست ضمن هذا الإصدار: `add` يثبّت من ملف
> محلي.

---

## `pdfl test`

يشغّل النص البرمجي على كل ملف PDF في مجلد، ويقارن كل تقرير بالتقرير المسجَّل
بجانبه. فالملف التعريفي الذي يبدأ بإيجاد شيء مختلف يُسقِط اختبارًا، بدل أن
يفاجئ أحدًا في آخر السلسلة.

```bash
pdfl test <script.pdfl> [--dir <مجلد>] [--update]
```

| الخيار | الافتراضي | الغرض |
|---|---|---|
| `--dir <مجلد>` | `tests/` بجانب النص البرمجي | حيث توجد ملفات الحالات |
| `--update` | — | يسجّل التقارير المتوقعة بدل أن يقارنها |
| `--jobs <n>` | `1` | عدد الحالات المتزامنة؛ و`0` يعني واحدة لكل معالج |

الحالة الواحدة ملف PDF والتقرير المتوقع منه، جنبًا إلى جنب:

```
profiles/print-shop/
  prepress.pdfl
  tests/
    approved.pdf
    approved.expected.json
    heavy_ink.pdf
    heavy_ink.expected.json
```

```bash
# أول مرة: سجّل ما يجده النص البرمجي اليوم
pdfl test prepress.pdfl --update

# وبعدها
pdfl test prepress.pdfl
```

```
ok   approved.pdf
FAIL heavy_ink.pdf
     error_count: expected 1, got 0
     missing:    PDFL-093751a2 [error] Ink coverage (line 12): page 7: 324% ink (limit 300%)
1 passed, 1 failed
```

والإخفاق يسمّي ما تغيّر — الأعداد والحكم وأي الملاحظات ظهرت أو اختفت — بدل أن
يطبع ملفَّي JSON متجاورين.

والتسجيل فعلٌ مقصود دائمًا: التشغيلة التي تحدّث خطّ أساسها بنفسها لا تُخفق أبدًا.
اقرأ الفرق أولًا، ثم أعد التسجيل بـ `--update` إذا كان التغيير هو ما أردته.

والتقرير المتوقع هو ما ينتجه `pdfl run`، مع اختصار `input_file` إلى اسم الملف —
فخطُّ أساس يتغيّر بتغيّر المجلد الذي نُودي منه ليس خطَّ أساس. وملف PDF الذي لا
يُفتح يُسقِط حالته وحدها ويترك البقية تعمل.

رموز الخروج: `0` نجحت كلها، و`2` أخفقت واحدة على الأقل، و`10` تعذّرت قراءة
المجلد أو لا PDF فيه.

### تشغيل الحالات معًا

كل حالة تعمل في عملية `pdfl` خاصة بها، فـ`--jobs` يحوّل المجموعة إلى تواز حقيقي:
في ثمانية ملفات من 41 صفحة، استغرق `--jobs 1` نحو 8.9 ثانية و`--jobs 8` نحو 1.1
ثانية. أما الخيوط داخل عملية واحدة فما كانت لتنجح — إذ يُسلسِل pdfium كل نداء
خلف قفل واحد، وقد قِيست نسخة الخيوط *أبطأ* من التنفيذ المتسلسل.

والقيمة الافتراضية `1` لأن كل مَهمّة عملية تحمل مستندًا في الذاكرة، وهذه الأداة
موجودة لملفات قد تكون ضخمة. ارفعها إذا كانت حالاتك عادية: `--jobs 0` يعطي واحدة
لكل معالج.

ولا يتغيّر ترتيب المخرَج بتغيّر `--jobs`: تُحكَم الحالات بالترتيب الذي وُجدت به،
أيًّا كانت العملية التي انتهت أولًا.

والحالة التي لا يُقرأ ملفها تُحكَم كغيرها — إذ يحمل تقريرها السبب بوصفه ملاحظة،
فيصير «هذا الملف يجب أن يُرفض لتعذّر قراءته» اختبارًا بذاته. وذلك التقرير يسمّي
الملف كما مُرِّر، فسجّل خطوط الأساس بـ`--dir` **نسبي** إن كنت ستحفظها في
المستودع.

---

## `pdfl completions`

يطبع على stdout نصّ الإكمال التلقائي لصدفتك.

```bash
pdfl completions <bash|zsh|fish|elvish|powershell>
```

```bash
# bash، للمستخدم الحالي
pdfl completions bash > ~/.local/share/bash-completion/completions/pdfl

# zsh — في أي موضع ضمن ‎$fpath
pdfl completions zsh > ~/.zfunc/_pdfl

# fish
pdfl completions fish > ~/.config/fish/completions/pdfl.fish
```

لا يُكتب شيء آخر على stdout، فيمكن توجيه المخرَج مباشرة إلى مجلّد الإكمال. وأعِد
توليده بعد كل ترقية: فالنصّ يُبنى من أوامر الملف التنفيذي الذي طبعه ومن خياراته.

---

[← المكتبة القياسية](10-stdlib.md) · [الفهرس](README.md) · [التالي: وصفات عملية →](12-recipes.md)
