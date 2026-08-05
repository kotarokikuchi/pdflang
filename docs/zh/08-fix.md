# 8. `fix::` 命名空间 — 规范化

[← `codes::`](07-codes.md) · [目录](README.md) · [下一章：`data::` →](09-data.md)

**修改** PDF 并保存为新文件的 19 项操作。原文件绝不会被改动。

---

## 8.1 使用方式

`fix::` 是唯一会写入的命名空间，因此使用独立的命令：

```bash
pdfl fix input.pdf script.pdfl --output fixed.pdf
```

| 选项 | 功能 |
|---|---|
| `--output <file>` | 输出 PDF（必填） |
| `--dry-run` | 只列出操作，不保存 |
| `--report json\|csv\|html\|pdf` | 报告格式 |
| `--report-file <file>` | 把报告写入文件 |

在 `pdfl run` 中调用 `fix::` 会报错并提示正确的命令，避免有人以为只是在校验
却实际改动了文件。

### 操作的执行方式

```pdfl
// 这个脚本不需要 check：它们是按顺序执行的命令。
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::add_page_numbers()
fix::add_watermark("DRAFT")
```

每次调用都会**当场校验**（页面不存在、旋转角非法、文件缺失），随后才应用。
报告中的 `fixes` 字段记录已完成的操作：

```json
"fixes": [
  "TrimBox set to [8.5, 8.5, 586.5, 833.5]",
  "page numbering added",
  "watermark \"DRAFT\" added"
]
```

在同一个脚本里混合校验与修改也没有问题：

```pdfl
// 先校验再修改 — 前提不成立时会体现在报告里
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(), "file is encrypted, cannot fix it"
}

fix::add_page_numbers()
```

---

## 8.2 页面框

| 操作 | 功能 |
|---|---|
| `fix::set_page_size(width, height)` | 设置所有页面的 MediaBox |
| `fix::set_crop_box(x0, y0, x1, y1)` | 设置所有页面的 CropBox |
| `fix::set_trim_box(x0, y0, x1, y1)` | 设置所有页面的 TrimBox |
| `fix::set_bleed_box(x0, y0, x1, y1)` | 设置所有页面的 BleedBox |

坐标以点为单位，从左下到右上。

```pdfl
// 用单位书写，换算自动完成
fix::set_page_size(210mm, 297mm)

// 出版社交来的文件没有制作用页面框：
// TrimBox = 成品尺寸，BleedBox = 含 3mm 出血
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)
```

---

## 8.3 页面

| 操作 | 功能 |
|---|---|
| `fix::rotate_page([page,] degrees)` | 旋转 90/180/270 度（省略页码则全部） |
| `fix::delete_page(n)` | 删除页面 |
| `fix::duplicate_page(n)` | 复制页面（副本插在其后） |
| `fix::reorder_pages([...])` | 重新排序（每页恰好使用一次） |
| `fix::split_document(from, to, "out.pdf")` | 把页面区间另存为文件 |
| `fix::merge_documents("other.pdf")` | 把另一个 PDF 的页面追加到末尾 |

试图删除唯一的页面会被明确拒绝。

```pdfl
fix::rotate_page(90)        // 全部页面
fix::rotate_page(3, 180)    // 仅第 3 页
fix::delete_page(1)         // 删掉草稿封面
fix::reorder_pages([4, 1, 2, 3])

// 封面和正文分别交给不同供应商
fix::split_document(1, 2, "cover.pdf")
fix::split_document(3, 50, "body.pdf")

fix::merge_documents("attachments/warranty.pdf")
```

---

## 8.4 内容

| 操作 | 功能 |
|---|---|
| `fix::add_watermark("text")` | 所有页面加斜向灰色水印 |
| `fix::add_stamps("text")` | 每页右上角加红色印章 |
| `fix::add_page_numbers()` | 页脚加上 `n / total` |
| `fix::remove_annotations()` | 删除所有批注 |
| `fix::remove_attachments()` | 删除所有附件 |
| `fix::flatten_layers()` | 解除可选内容（OCG）结构 |

```pdfl
fix::add_watermark("DRAFT — DO NOT PRINT")
fix::add_stamps("APPROVED 2026-08-02")
fix::add_page_numbers()

// 送印前：校对批注不能出现，附件也只会让文件变大
fix::remove_annotations()
fix::remove_attachments()

// 防止关闭的「英文版」图层在印刷厂被误开启
fix::flatten_layers()
```

---

## 8.5 优化

> 本节的操作**只在文件变小时才写入**。若重写后反而更大，则保留原文件。

| 操作 | 功能 |
|---|---|
| `fix::remove_unused_resources()` | 丢弃从 trailer 无法到达的对象 |
| `fix::downsample_images([dpi])` | 对超过目标 DPI（默认 300）的图像重采样 |
| `fix::compress_images([quality])` | 以 JPEG 重新编码（1–100，默认 85） |

DPI 依据图像在页面上的**实际印刷尺寸**计算。

> **CMYK 图像会被保留。** 对其重采样需要转换为 RGB，那会破坏印前分色。
> 在印刷厂的文件里，体积的节省来自 RGB 图像。

```pdfl
// 用于邮件审批的版本不需要 300 DPI
fix::downsample_images(96)
fix::compress_images(70)
fix::remove_unused_resources()
```

### 不提供的操作

`subset_fonts` 和 `linearize_document` **不作为** `fix::` 操作存在，调用会
报「未知函数」错误。

- **subset_fonts**：曾实现并实测。专业生成工具本就只嵌入用到的字形，实测
  收益最好情况仅 0.5%，其余为零，不值得承担损坏字体的风险。若要*检查*字体
  是否为子集，请使用 [`prepress::subset_fonts()`](06-prepress.md)。
- **linearize_document**：需要生成提示表（PDF 规范 §7.14）。没有任何 Rust
  库能做到，部分实现也不会被阅读器识别为「Fast Web View」。

---

## 8.6 完整示例

```pdfl
// prepare_for_print.pdfl — 把出版社的文件整理为送印版本
// 用法: pdfl fix publisher.pdf prepare_for_print.pdfl --output print.pdf

check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask the publisher for the open version"
}

// 出版社未设置的制作用页面框
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// 清理：校对批注和附件不进入印刷
fix::remove_annotations()
fix::remove_attachments()
fix::flatten_layers()
fix::remove_unused_resources()
```

```pdfl
// email_version.pdfl — 邮件审批用的轻量版本
// 用法: pdfl fix final.pdf email_version.pdfl --output approval.pdf

fix::downsample_images(96)
fix::compress_images(70)
fix::add_watermark("PROOF — NOT THE FINAL VERSION")
fix::add_page_numbers()
```

用 `pdfl` 自己确认结果：

```bash
pdfl fix final.pdf email_version.pdfl --output approval.pdf
pdfl inspect approval.pdf          # 新文件的大小、DPI 和警告
```

---

[← `codes::`](07-codes.md) · [目录](README.md) · [下一章：`data::` →](09-data.md)
