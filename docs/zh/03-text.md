# 3. `text::` 命名空间 — 文本

[← 类型](02-types.md) · [目录](README.md) · [下一章：`struct::` →](04-struct.md)

用于提取、规范化、检索和校验文档文本的 25 个函数。

> 标注 `[text]` 的参数是**可选的**：省略时作用于整个文档，给出时作用于
> 你传入的字符串。

---

## 3.1 提取

| 函数 | 功能 |
|---|---|
| `text::extract_all()` | 整个文档的文本（页面以换行分隔） |
| `text::extract_from_page(page)` | 指定页的文本（从 1 开始） |
| `text::extract_from_region(page, region)` | 指定区域内的文本（没有则返回空串） |
| `text::extract_with_normalization()` | 已规范化的文档文本 |

```pdfl
check "Extraction" {
  content = text::extract_all()
  assert content.trim() != "", "PDF has no extractable text"

  cover = text::extract_from_page(1)
  assert cover.contains("User Manual"), "cover lacks the expected title"

  // 制作用页脚（InDesign 文件名、导出时间）有时会残留到成品中
  footer = region(0, 0, 467, 40, "footer")
  doc.pages.each { |page|
    line = text::extract_from_region(page.number, footer)
    assert !line.contains(".indd"),
      "page #{page.number} has a production mark in the footer: #{line.trim()}"
  }
}
```

---

## 3.2 规范化与切分

| 函数 | 功能 |
|---|---|
| `text::normalize([text])` | 转小写 + 压缩空白 |
| `text::split_words([text])` | 切分为单词（去掉两端标点） |
| `text::split_sentences([text])` | 切分为句子 |
| `text::split_paragraphs([text])` | 切分为段落（空行分隔） |
| `text::count_words([text])` | 单词数 |
| `text::count_characters([text])` | 字符数 |
| `text::detect_language([text])` | `"pt"`、`"en"`、`"es"` 或 `"unknown"` |

```pdfl
check "Normalization and splitting" {
  require text::normalize("  HELLO   World  ") == "hello world"

  words = text::split_words("Hello, world! (test)")
  require words.length == 3
  require words.first() == "Hello"

  // 说明书和合同在可读性上有实际的长度上限
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

## 3.3 检索与必需内容

| 函数 | 功能 |
|---|---|
| `text::require_text(term)` | 包含该词句则为真 |
| `text::forbid_text(term)` | 不包含该词句则为真 |
| `text::require_match(regex)` | 匹配正则则为真 |
| `text::forbid_match(regex)` | 不匹配正则则为真 |
| `text::fuzzy_match(a, b)` | 两个字符串的相似度（0.0–1.0） |

比较时忽略大小写和空白。

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
    // 适用于可能存在错字或 OCR 误差的场合
    found = text::extract_from_region(1, region(50, 700, 300, 40))
    similarity = text::fuzzy_match("Paracetamol 750mg", found)
    assert similarity > 0.9,
      "product name differs from expected (#{round(similarity * 100)}% similar)"
  }
}
```

---

## 3.4 个人信息

`text::detect_personal_data()` 与 `text::detect_pii()` 是同义函数，返回找到的
个人信息列表：CPF、CNPJ（巴西纳税人号码）、电子邮件和电话号码。

> CPF 与 CNPJ **仅在校验位正确时**才会列出。只是长得像的号码
> （例如 `111.111.111-12`）不会误报。

```pdfl
check "Public document must carry no personal data" {
  found = text::detect_personal_data()
  assert found.length == 0, "personal data exposed: #{found.join("; ")}"

  // 每一项形如 "CPF: 529.982.247-25"
  text::detect_pii().each { |item| print("found:", item) }
}
```

---

## 3.5 格式校验

| 函数 | 功能 |
|---|---|
| `text::validate_cpf(text)` | CPF 校验位（mod 11） |
| `text::validate_cnpj(text)` | CNPJ 校验位 |
| `text::validate_date_format(text [, format])` | 是否为日历上有效的日期 |
| `text::validate_phone_format(text)` | 巴西电话号码格式 |
| `text::validate_format(text, regex)` | **整个**字符串是否匹配正则 |

日期格式支持 `"dd/mm/aaaa"` 和 `"aaaa-mm-dd"`；省略第二个参数时两者都接受。

```pdfl
check "Format validation" {
  require text::validate_cpf("529.982.247-25")
  require !text::validate_cpf("111.111.111-11")    // 全为相同数字
  require text::validate_cnpj("11.222.333/0001-81")

  require text::validate_date_format("29/02/2024")   // 2024 是闰年
  require !text::validate_date_format("29/02/2023")  // 2023 不是
  require !text::validate_date_format("31/04/2026")  // 四月只有 30 天

  require text::validate_phone_format("(11) 98765-4321")

  // 工厂的批号格式
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  assert text::validate_format(batch, "L\d{4}-\d{2}"),
    "batch code does not follow the L0000-00 pattern: #{batch}"
}
```

---

## 3.6 比较与诊断

`text::diff(a, b)` 返回变化的行（`-` 删除、`+` 新增）。
`text::detect_rasterized_text()` 在存在被图像化的文字时返回真。

```pdfl
check "Comparison and diagnostics" {
  changes = text::diff(text::extract_from_page(1), text::extract_from_page(2))
  print("changed lines:", changes.length)

  // 扫描或已转曲的页面无法检索，屏幕阅读器也读不了
  assert !text::detect_rasterized_text(),
    "there are pages with rasterized text (scanned or outlined)"
}
```

> 要比较两个**文件**，请使用 `pdfl compare`，它会自动对齐页面。
> 参见[第 11 章](11-cli.md)。

---

## 3.7 完整示例

```pdfl
// legal_document.pdfl — 合同校验
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

[← 类型](02-types.md) · [目录](README.md) · [下一章：`struct::` →](04-struct.md)
