# 10. 标准库

[← `data::`](09-data.md) · [目录](README.md) · [下一章：命令行 →](11-cli.md)

列表和字符串的方法，以及在脚本任何位置都可用的全局函数。

---

## 10.1 列表方法

| 方法 | 功能 |
|---|---|
| `list.each { \|item\| ... }` | 对每个元素执行块 |
| `list.each_with_index { \|item, i\| ... }` | 同时得到位置（从 **0** 开始） |
| `list.all { \|item\| ... }` | 全部满足条件则为真（空列表为真） |
| `list.any { \|item\| ... }` | 任一满足条件则为真（空列表为假） |
| `list.filter { \|item\| ... }` | 只保留满足条件的元素 |
| `list.map { \|item\| ... }` | 变换后的新列表 |
| `list.length` | 元素个数（`length()` 亦可） |
| `list.contains(value)` | 是否包含该值 |
| `list.get(n)` | 第 n 个元素（从 **1** 开始） |
| `list.first()` / `list.last()` | 首 / 末元素（空列表返回 `null`） |
| `list.join([separator])` | 连接为字符串（默认分隔符 `", "`） |

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

  // get 从 1 开始：get(1) 是第一个元素
  row = data::load_dataset("data/batches.csv").get(2)
  print("first column:", row.get(1))

  // 空列表也安全：null 为假
  spots = prepress::detect_spot_colors()
  assert !spots.first() || spots.first() == "Varnish",
    "unexpected special ink: #{spots.first()}"
}
```

---

## 10.2 字符串方法

| 方法 | 功能 |
|---|---|
| `text.contains(sub)` | 是否包含子串 |
| `text.starts_with(sub)` | 是否以其开头 |
| `text.ends_with(sub)` | 是否以其结尾 |
| `text.trim()` | 去掉两端空白 |
| `text.to_uppercase()` | 全部大写 |
| `text.to_lowercase()` | 全部小写 |
| `text.length` | 字符数 |

```pdfl
check "String methods" {
  title = doc.title
  require title.length > 0
  require title.trim() == title          // 没有多余空白
  assert !title.to_lowercase().contains("draft"),
    "title still marked as draft"

  code = codes::decode_barcode(1)
  assert code.starts_with("789"), "GTIN is not Brazilian"
  assert doc.filename.ends_with(".pdf"), "unexpected extension"
}

check "contains on each type" {
  // 字符串：查找文本中的「片段」
  require "final document".contains("final")

  // 列表：查找完整的「元素」
  require ["a", "b"].contains("a")
  require !["ab"].contains("a")      // "a" 不是该列表的元素
}
```

---

## 10.3 全局函数

| 函数 | 功能 |
|---|---|
| `min(a, b)` / `max(a, b)` | 较小 / 较大者 |
| `abs(x)` | 绝对值 |
| `round(x)` | 四舍五入到最近的整数 |
| `print(...)` | 以空格分隔输出（**标准错误**） |
| `region(x, y, w, h [, name])` | 创建区域（[第 2 章](02-types.md)） |

`print` 输出到标准错误，因此 `> report.json` 只会得到报告本身。

```pdfl
check "Global functions" {
  const A4_WIDTH = 595.0
  const TOLERANCE = 5.0

  // abs 是带容差比较尺寸的关键
  doc.pages.each { |page|
    assert abs(page.width - A4_WIDTH) < TOLERANCE,
      "page #{page.number} is outside A4: #{page.width}pt"
  }

  // round 让消息更易读
  // 不用 round："217.4453125 DPI"；用了："217 DPI"
  doc.images.each { |img|
    assert img.dpi >= 300,
      "image on page #{img.page_number}: #{round(img.dpi)} DPI"
  }

  print("document:", doc.filename)
  print("pages:", doc.page_count, "| fonts:", doc.fonts.length)
}
```

---

## 10.4 常用写法

```pdfl
// 统计有多少元素不合格
check "Problem count" {
  bad = doc.images.filter { |i| i.dpi < 300 }
  assert bad.length == 0,
    "#{bad.length} of #{doc.images.length} images below 300 DPI"
}

// 在消息中列出不合格的元素
check "List in the message" {
  // 串联写在同一行：点号前不要换行
  problems = doc.pages.filter { |p| !p.has_trim_box }.map { |p| p.number }
  assert problems.length == 0,
    "pages without a TrimBox: #{problems.join(", ")}"
}

// 带容差的校验
function close_to(value, target, tolerance) {
  abs(value - target) < tolerance
}

check "With tolerance" {
  doc.pages.each { |page|
    assert close_to(page.width, 595.0, 2.0),
      "page #{page.number}: width #{page.width}pt (expected 595 ± 2)"
  }
}

// 避免在空文档上报错
check "Defensive" {
  // 短路求值使得不会在空列表上调用 first()
  assert doc.pages.length == 0 || doc.pages.first().width > 0,
    "the first page has no width"
}
```

---

[← `data::`](09-data.md) · [目录](README.md) · [下一章：命令行 →](11-cli.md)
