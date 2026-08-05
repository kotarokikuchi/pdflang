# 4. `struct::` 命名空间 — 结构与元数据

[← `text::`](03-text.md) · [目录](README.md) · [下一章：`visual::` →](05-visual.md)

关于文件本身的 23 个函数：元数据、内部对象、安全性和可追溯性。

> 从 `list_objects` 起的函数会读取文件的内部结构。该分析在首次使用时
> **只执行一次**，之后被缓存。

---

## 4.1 元数据

| 函数 | 返回 |
|---|---|
| `struct::get_title()` | 标题 |
| `struct::get_author()` | 作者 |
| `struct::get_subject()` | 主题 |
| `struct::get_keywords()` | 关键词 |
| `struct::get_creator()` | 创建原始文档的程序 |
| `struct::get_producer()` | 生成 PDF 的程序 |
| `struct::get_creation_date()` | 创建时间（`YYYY-MM-DD HH:MM:SS`） |
| `struct::get_modification_date()` | 修改时间（同一格式） |
| `struct::list_metadata_entries()` | 非空条目列表（`"键: 值"`） |
| `struct::extract_xmp()` | 目录中的 XMP 元数据 |

字段缺失时返回空字符串。

```pdfl
check "Required metadata" {
  assert struct::get_title() != "", "PDF has no title"
  assert struct::get_author() != "", "PDF has no author"

  // Producer 表明来源工具 — 便于追踪问题
  print("produced by:", struct::get_producer())

  created = struct::get_creation_date()
  assert created != "", "PDF has no creation date"
  // 格式可排序，因此字符串比较有效
  assert created > "2026-01-01", "file is too old for this campaign"

  xmp = struct::extract_xmp()
  assert xmp.contains("pdfaid"), "no PDF/A identification in the XMP"
}
```

---

## 4.2 文件与可追溯性

| 函数 | 功能 |
|---|---|
| `struct::file_size()` | 大小（字节） |
| `struct::calculate_sha256()` | 文件的 SHA-256 哈希 |
| `struct::detect_file_bloat([kb_per_page])` | 超过每页上限（默认 1024 KB）则为真 |

```pdfl
check "File size and traceability" {
  mb = struct::file_size() / 1024 / 1024
  assert mb < 10, "file is #{round(mb)} MB (10 MB e-mail limit)"

  // 哈希可以证明究竟是哪一个文件被批准
  print("SHA-256:", struct::calculate_sha256())

  assert !struct::detect_file_bloat(1024),
    "heavy file: #{struct::file_size() / 1024} KB for #{doc.page_count} pages"
}
```

---

## 4.3 内部对象

| 函数 | 功能 |
|---|---|
| `struct::count_objects()` | 页面中的内容对象数量 |
| `struct::list_objects()` | 全部对象列表（`"编号: 类型"`） |
| `struct::detect_unreferenced_objects()` | 从 trailer 无法到达的对象 |
| `struct::detect_orphaned_resources()` | 无法到达的资源（字体、图像） |
| `struct::measure_object_size(number)` | 指定对象的大致大小（字节） |

> 基础设施对象（`ObjStm`、`XRef`）被排除在外：按定义它们本就不被 trailer
> 引用，报告它们属于误报。

```pdfl
check "File hygiene" {
  require struct::count_objects() > 0

  loose = struct::detect_unreferenced_objects()
  assert loose.length == 0,
    "#{loose.length} unreferenced object(s): #{loose.join(", ")}"

  orphans = struct::detect_orphaned_resources()
  assert orphans.length == 0,
    "unused embedded resources: #{orphans.join(", ")} — run 'pdfl fix' with remove_unused_resources()"
}
```

---

## 4.4 安全

| 函数 | 功能 |
|---|---|
| `struct::detect_javascript()` | 含有嵌入的 JavaScript 则为真 |
| `struct::detect_suspicious_actions()` | 危险动作列表 |
| `struct::check_encryption()` | 文档被加密则为真 |
| `struct::validate_permissions()` | 没有权限限制则为真 |
| `struct::validate_signatures()` | 存在数字签名字段则为真 |

`detect_suspicious_actions` 会检出 `JavaScript`、`Launch`（运行程序）、`URI`、
`SubmitForm`、`ImportData`、`GoToR`。

> `validate_signatures` 检查字段的**存在**。本版本不做证书链的密码学验证。

```pdfl
check "Security" {
  // PDF 中的 JavaScript 是常见的攻击途径，
  // 对印刷用文档也毫无必要
  assert !struct::detect_javascript(), "PDF contains embedded JavaScript"

  actions = struct::detect_suspicious_actions()
  assert actions.length == 0,
    "suspicious actions in the PDF: #{actions.join("; ")}"

  // 加密的 PDF 可能在印刷厂的 RIP 上失败
  assert !struct::check_encryption(),
    "PDF is encrypted — remove protection before sending it to print"
  assert struct::validate_permissions(),
    "PDF has permission restrictions that may block processing"
}
```

---

## 4.5 完整示例

```pdfl
// audit.pdfl — 合规与安全检查
profile "file-audit" {

  check "Identification" tags: ["metadata"] {
    assert struct::get_title() != "", "no title"
    assert struct::get_author() != "", "no author"
    assert struct::get_creation_date() != "", "no creation date"
    print("produced by:", struct::get_producer())
  }

  check "Traceability" tags: ["audit"] {
    print("SHA-256:", struct::calculate_sha256())
    print("size:", struct::file_size() / 1024, "KB")
  }

  check "Security" tags: ["security"] {
    assert !struct::detect_javascript(), "embedded JavaScript"
    assert !struct::check_encryption(), "encrypted file"
    actions = struct::detect_suspicious_actions()
    assert actions.length == 0, "suspicious actions: #{actions.join("; ")}"
  }

  check "File hygiene" tags: ["optimization"] {
    orphans = struct::detect_orphaned_resources()
    assert orphans.length == 0, "unused resources: #{orphans.join(", ")}"
    assert !struct::detect_file_bloat(1024), "bloated file"
  }
}
```

---

[← `text::`](03-text.md) · [目录](README.md) · [下一章：`visual::` →](05-visual.md)
