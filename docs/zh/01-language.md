# 1. PDFLang 语言

[← 目录](README.md) · [下一章：文档类型 →](02-types.md)

PDFLang 的设计目标是让不写程序的人也能读懂。没有类、没有继承、没有类型声明、
也没有分号。一个脚本就是一组用近乎自然语言写成的检查。

---

## 1.1 脚本的结构

```pdfl
// 注释以两个斜杠开头，直到行尾。

profile "profile-name" {         // profile 可选：为整组命名并分组，
                                 // 名称会出现在报告中。

  const LIMIT = 300%             // 常量：习惯用大写

  check "Check Name" {           // 每个 check 成为报告的一个小节
    require doc.page_count > 0   // 一条校验
  }

  check "Another Check" {        // check 可以写任意多个
    require doc.title != ""
  }
}
```

`profile` 可以省略 — 脚本也可以只是一串 check：

```pdfl
check "Simple" {
  require doc.page_count > 0
}
```

### check 的标签

标签用于在报告中归类和筛选 check：

```pdfl
check "Ink within limit" tags: ["prepress", "colors"] {
  require prepress::validate_tac_limits(300)
}
```

### check 的严重级别

默认情况下，失败的 check 是**错误**，运行以 2 退出。check 可以声明自己只是
建议性的：

```pdfl
check "图像分辨率" severity: warning {
  require !visual::detect_low_resolution(300)
}
```

共三级：`error`（默认）、`warning`、`info`。警告和信息不会让运行失败——它们以
1 和 0 退出——除非传入 `--fail-on warning`，CI 由此决定严格程度而无需改动脚本。

`tags:` 与 `severity:` 的先后顺序不限。

> check 内部的运行时错误——变量拼写错误、文件缺失——无论 check 声明了什么，
> 都仍然是错误。脚本坏了不是建议。

---

## 1.2 两种校验写法

所有校验都使用 `require` 或 `assert`。区别只在于失败时报告中显示的消息。

```pdfl
check "Comparing both forms" {

  // require：消息由表达式本身生成。
  // 失败时报告显示：
  //   "requirement not met: doc.page_count > 0"
  require doc.page_count > 0

  // assert：由你编写最终读者看到的消息。
  // 失败时原样显示：
  //   "PDF has no title in its metadata"
  assert doc.title != "", "PDF has no title in its metadata"
}
```

**实用原则：** 表达式本身足够清楚时用 `require`；当阅读报告的人不了解脚本
也需要理解问题时，用 `assert`。

### 一条失败不会中断其他检查

```pdfl
check "Three independent validations" {
  assert doc.page_count > 100, "too few pages"    // 失败
  assert doc.title != "", "no title"              // 仍然执行
  assert doc.author != "", "no author"            // 这条也会执行
}
```

报告会一次性列出**所有**问题。这是有意为之：收到文件的人需要的是完整的
修改清单，而不是一次一条。

check 之间也一样 — 某个 check 遇到运行时错误（例如未定义的变量），它会变成
一条诊断，其余 check 继续运行。

---

## 1.3 值与类型

### 数字和单位

```pdfl
check "Numbers" {
  x = 42          // 整数
  y = 2.5         // 小数

  // 长度单位会自动换算为点（1 pt = 1/72 英寸）：
  a = 3mm         // 8.5039... pt
  b = 2.5cm       // 70.866... pt
  c = 1in         // 72 pt
  d = 10pt        // 10 pt

  // 百分号保留数值本身：
  limit = 300%    // 300

  require a < b            // 全部都是点，可以直接比较
  require c == 72.0
  require limit == 300
}
```

能写 `3mm` 而不是 `8.504`，正是关键所在：对以毫米思考的人来说读起来自然，
换算也不会出错。

### 文本

```pdfl
check "Strings" {
  simple = "plain text"

  // 插值：#{...} 会嵌入任意表达式的值
  name = "document.pdf"
  message = "Analyzing #{name} with #{doc.page_count} pages"

  // 转义：\n（换行）、\t（制表）、\"（引号）、\\（反斜杠）
  quoted = "he said \"hello\""

  // 未知的反斜杠原样保留 — 这样写正则表达式
  // 不需要双重转义：
  pattern = "\d{3}\.\d{3}\.\d{3}-\d{2}"

  require message.contains("pages")
}
```

### 布尔值与「真」的判定

```pdfl
check "True and false" {
  yes = true
  no = false

  // 只有 false 和 null 为假。其他一切皆为真 —
  // 包括 0、空字符串和空列表。
  require 0        // 通过（0 为真）
  require ""       // 通过（空字符串为真）

  // 因此要检查内容时，请显式比较：
  require doc.title != ""              // 正确
  require doc.pages.length > 0         // 正确
}
```

这在返回 `null` 的函数上很有用：

```pdfl
check "Taking advantage of null" {
  description = data::lookup_value("batches.csv", "L2026-08")
  // null 为假，所以可以直接这样写：
  assert description, "batch not found in the table"
}
```

### 列表

```pdfl
check "Lists" {
  numbers = [1, 2, 3]
  words = ["a", "b", "c"]
  mixed = [1, "two", true]

  require numbers.length == 3
  require numbers.contains(2)
  require words.join(", ") == "a, b, c"

  // 访问从 1 开始：第一个元素是第 1 个
  require numbers.get(1) == 1
  require numbers.first() == 1
  require numbers.last() == 3
}
```

---

## 1.4 运算符

```pdfl
check "Operators" {
  // 比较
  require 10 > 5
  require 10 >= 10
  require 3 < 4
  require 3 <= 3
  require "a" == "a"
  require "a" != "b"

  // 算术
  require 2 + 3 == 5
  require 10 - 4 == 6
  require 3 * 4 == 12
  require 10 / 4 == 2.5        // 除不尽时得到小数
  require 10 / 5 == 2          // 整除时仍为整数

  // 逻辑（短路求值：右侧仅在需要时计算）
  require true && true
  require false || true
  require !false

  // 短路求值的实际用途：没有页面时右侧不会被求值，
  // 空文档也不会报错。
  require doc.page_count == 0 || doc.pages.first().width > 0
}
```

---

## 1.5 块：对每个元素重复

块是写在花括号里的代码，参数放在两个竖线之间。读起来就像「对每一页，做……」。

```pdfl
check "Walking through pages" {

  // each：对每个元素执行块
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  // each_with_index：同时得到位置（0、1、2……）
  doc.fonts.each_with_index { |font, i|
    print("font", i, ":", font.name)
  }

  // all：所有元素都满足条件时为真
  require doc.fonts.all { |f| f.is_embedded }

  // any：任一元素满足条件时为真
  require doc.pages.any { |p| p.extract_text() != "" }

  // filter：只保留满足条件的元素
  blank = doc.pages.filter { |p| p.extract_text() == "" }
  assert blank.length == 0, "#{blank.length} blank page(s)"

  // map：把每个元素变换成新列表
  names = doc.fonts.map { |f| f.name }
  print("fonts in use:", names.join(", "))
}
```

块可以串联 — 但必须写在**同一行**，点号前不能换行：

```pdfl
check "Chaining" {
  // 未嵌入的字体，只取名称，用逗号连接
  problems = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  assert problems.length == 0,
    "fonts not embedded: #{problems.join(", ")}"
}
```

如果一行太长，请拆成有名字的步骤，而不是断开串联 — 这样反而更好读：

```pdfl
check "Named steps" {
  loose = doc.fonts.filter { |f| !f.is_embedded }
  names = loose.map { |f| f.name }
  assert names.length == 0, "fonts not embedded: #{names.join(", ")}"
}
```

---

## 1.6 函数：给规则起名字

当同一段校验反复出现时，给它起个名字：

```pdfl
// 函数的值就是「最后一个表达式」的值 — 没有 return。
function is_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}

function exceeds_ink(page, limit) {
  page.tac > limit
}

check "Format and ink" {
  // 这样 check 读起来几乎就是一句话
  require doc.pages.all { |p| is_a4(p) }

  doc.pages.each { |page|
    assert !exceeds_ink(page, 300), "page #{page.number} has too much ink"
  }
}
```

函数的规则：

- 参数只在函数内部有效。
- 函数可以调用其他函数。
- 允许递归，但上限为 200 次调用（防止失控的脚本卡住进程）。

---

## 1.7 import：在配置之间复用

把通用规则放进一个文件，在需要的地方引入。

`library.pdfl`：

```pdfl
// 团队共享的常量和函数
const OFFSET_TAC = 300%
const DEFAULT_BLEED = 3mm

function a4_page(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}
```

`magazine.pdfl`：

```pdfl
// 路径相对于「本文件」
import "library.pdfl"

check "Format" {
  // OFFSET_TAC 和 a4_page 来自 import
  require doc.pages.all { |p| a4_page(p) }
  require prepress::validate_tac_limits(OFFSET_TAC)
}
```

同一个文件**只会加载一次**，即使多个脚本都引入它 — 因此循环引入不会卡死。

---

## 1.8 rule：逐页校验

`rule` 是对每一页执行一次的 check，页面已绑定到 `page` 变量：

```pdfl
// 没有 "on"：在所有页面上执行
rule "Every page has text" {
  assert page.extract_text().trim() != "",
    "page #{page.number} is blank"
}
```

加上 `on` 可以选择适用的页面：

```pdfl
rule "Body pages numbered" on doc.pages.filter { |p| p.number > 2 } {
  footer = region(0, 0, page.width, 60)
  assert text::extract_from_region(page.number, footer) != "",
    "page #{page.number} has no page number in the footer"
}
```

> **语法注意：** 如果 `on` 的选择表达式以属性结尾（例如 `on doc.pages`），
> 请用括号包起来；否则主体的 `{` 会被当作该调用的块：
>
> ```pdfl
> rule "Example" on (doc.pages) {     // 需要括号
>   require page.width > 0
> }
> ```

---

## 1.9 变量与作用域

```pdfl
const GLOBAL = 100          // 整个文件可见

check "Scope" {
  local = 42                // 只在这个 check 内可见

  doc.pages.each { |page|
    inner = page.width      // 只在这个块内可见
    require inner > 0
  }

  require local == 42       // 仍然可见
  require GLOBAL == 100     // 仍然可见
}
```

习惯上常量用大写、变量用小写。语言并不强制，但示例和随附的配置都遵循这一约定。

---

### 来自命令行的值

`pdfl run`、`pdfl test`、`pdfl watch` 的 `--var 名称=值` 都会以 `vars.名称` 的形式
抵达脚本，始终是文本。`test` 和 `watch` 会把同一个值转给每一个用例或文件——整次
运行共用一个客户名称，而不是每个文件各自一个。正是
它让一个配置文件不必变成五份几乎一样的副本：

```pdfl
check "作业与订单相符" {
  assert doc.title.contains(vars.order),
    "文件写的是 \"#{doc.title}\"，订单是 #{vars.order}"
}
```

```bash
pdfl run intake.pdfl received.pdf --var order=SO-4471
```

没有传入的名称是**一个错误，并且错误会点出应当提供它的那个参数**，而不是空字符
串：与空值比较的 check 会直接通过，报告出一份根本没人校验过的文件。

---

## 1.10 让收件人受益的消息

报告的质量取决于你写的消息。对比一下：

```pdfl
check "Poor messages" {
  require doc.pages.all { |p| p.tac <= 300 }
  // 报告："requirement not met: doc.pages.all() { ... }"
  // — 收件人无从得知是哪一页、超出多少
}

check "Good messages" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "Page #{page.number}: ink coverage #{page.tac}% (max 300%)"
  }
  // 报告："Page 7: ink coverage 324% (max 300%)"
  // — 操作员立刻知道要改什么
}
```

对于不属于错误的补充信息，请使用 `print()`。它输出到标准错误，不会污染报告：

```pdfl
check "Context" {
  print("Analyzing", doc.page_count, "pages")
  print("Fonts:", prepress::list_fonts().join(", "))
  require doc.page_count > 0
}
```

---

## 1.11 常见错误

| 消息 | 原因 | 处理 |
|---|---|---|
| `expected end of line after statement` | 一行写了两条语句 | 一行一条语句 |
| `unknown variable: x` | 赋值前使用，或超出作用域 | 在同一层级先声明 |
| `unknown function: text::xyz` | 名称错误或函数不存在 | 查阅对应命名空间的章节 |
| `fix:: is only available in the 'pdfl fix' command` | 在 `pdfl run` 中使用 `fix::` | 改用 `pdfl fix input.pdf script.pdfl --output out.pdf` |
| `unknown unit: 'kg'` | 单位无效 | 使用 `pt`、`mm`、`cm`、`in` 或 `%` |
| `expected '{' with the rule body` | `on` 的选择表达式以属性结尾 | 用括号包住选择表达式 |
| `unexpected expression: Dot` | 串联被换行截断 | 把 `.method` 放在同一行，或使用中间变量 |

运行之前，做这两件事总是值得的：

```bash
pdfl lint my_profile.pdfl    # 未使用的变量、重复的 check……
pdfl fmt my_profile.pdfl     # 统一格式
```

---

[← 目录](README.md) · [下一章：文档类型 →](02-types.md)
