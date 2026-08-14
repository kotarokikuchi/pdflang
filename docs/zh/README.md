# PDFLang 文档 — 中文

`.pdfl` 语言和 `pdfl` 命令行工具的完整指南 — 版本 0.17.0。

本文档中的每个示例都是可运行、带注释的代码。如果你是第一次使用本语言，请从
第 1 章的手册开始，其余章节作为参考查阅。

> **关于工具语言。** `pdfl` 的消息（诊断、错误、命令行帮助、报告标签）使用
> **英文**，这是命令行工具的惯例。本文档为中文，但检查失败时会显示类似
> `page 7: 324% ink (limit 300%)` 的内容。你在脚本中**自己编写**的消息，
> 会按你书写的语言原样输出。

## 目录

| 章节 | 内容 |
|---|---|
| [1. 语言](01-language.md) | 完整手册：check、断言、类型、单位、块、函数、import、rule |
| [2. 文档类型](02-types.md) | `doc`、`page`、`font`、`image`、`region` — 全部属性和方法 |
| [3. `text::`](03-text.md) | 文本：提取、规范化、检索、巴西专用校验、个人信息 |
| [4. `struct::`](04-struct.md) | 结构与元数据：对象、XMP、安全、哈希 |
| [5. `visual::`](05-visual.md) | 图像：分辨率、视觉比对、pHash、SSIM、质量 |
| [6. `prepress::`](06-prepress.md) | 印前检查：油墨总量、分色、专色、字体、页面框 |
| [7. `codes::`](07-codes.md) | 条码与二维码：检测、解码、校验 |
| [8. `fix::`](08-fix.md) | 规范化：页面框、页面、水印、合并/拆分、优化 |
| [9. `data::`](09-data.md) | 外部数据：术语表、数据集、查询表 |
| [10. 标准库](10-stdlib.md) | 列表和字符串方法、全局函数 |
| [11. 命令行](11-cli.md) | `run`、`compare`、`pixelcompare`、`watch`、`fix`、`inspect`、`lint`、`fmt`、`doc`、`pack`、`add`、`test`、`completions` |
| [12. 实用范例](12-recipes.md) | 完整案例：印刷厂、法律出版社、制药实验室、CI/CD |
| [13. 变更记录](13-changelog.md) | 每个版本改了什么，以及可能破坏什么 |

## 30 秒上手

创建 `my_profile.pdfl`：

```pdfl
// 每个脚本都是一组 check。每个 check 汇集相关的校验，
// 并成为报告中的一个小节。
check "Basic structure" {
  // require：消息由表达式自动生成
  require doc.page_count > 0

  // assert：使用你自己编写的消息
  assert doc.title != "", "PDF has no title in its metadata"
}
```

运行：

```bash
pdfl run my_profile.pdfl document.pdf
```

报告以 JSON 形式输出到标准输出。退出码表明结果：
`0` 全部通过，`1` 仅有警告，`2` 校验错误，`3` 语法错误。

## 本文档的约定

- 每个函数都列出**签名**、**功能**、**返回值**和**带注释的示例**。
- 方括号中的参数是可选的：`calculate_tac([page])`。
- 「从 1 开始」指第一页是 `1` 而不是 `0` — 本语言按人们数页码的方式计数，
  而不是按程序员的方式。
- 尺寸始终以**点**为单位（1 pt = 1/72 英寸）。使用单位字面量（`3mm`、`1in`）
  即可自动换算。

---

其他语言： [English](../en/) · [Português (Brasil)](../pt-br/) ·
[日本語](../ja/) · [Français](../fr/) · [العربية](../ar/) · [Deutsch](../de/)
