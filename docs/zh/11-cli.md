# 11. 命令行

[← 标准库](10-stdlib.md) · [目录](README.md) · [下一章：实用范例 →](12-recipes.md)

共 10 个命令：4 个处理 PDF，4 个处理脚本，2 个用于分发。

| 命令 | 功能 |
|---|---|
| [`run`](#pdfl-run) | 用脚本校验 PDF |
| [`compare`](#pdfl-compare) | 比较两个版本 |
| [`watch`](#pdfl-watch) | 监视文件夹并校验新到的文件 |
| [`fix`](#pdfl-fix) | 应用修改并保存新的 PDF |
| [`inspect`](#pdfl-inspect) | 快速查看 PDF 概要 |
| [`lint`](#pdfl-lint) | 不执行地分析脚本 |
| [`fmt`](#pdfl-fmt) | 格式化脚本 |
| [`doc`](#pdfl-doc) | 由脚本生成文档 |
| [`pack`](#pdfl-pack) | 打包配置与数据 |
| [`add`](#pdfl-add) | 安装软件包 |

---

## 退出码

所有执行校验的命令都遵循同一约定。

| 代码 | 含义 |
|---|---|
| `0` | 全部通过 |
| `1` | 仅有警告 |
| `2` | 校验错误 |
| `3` | 脚本语法错误 |
| `10` | 无法读取文档，或无法写入文件——尚未做出判定 |

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

## `pdfl run`

用脚本校验 PDF。

```bash
pdfl run <script.pdfl> <input.pdf> [options]
```

| 选项 | 默认 | 功能 |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | 报告格式 |
| `--output-file <file>` | — | 写入文件而非标准输出 |
| `--fail-on error\|warning` | `error` | 设为 `warning` 时警告也退出码 2 |
| `--verbose` | — | 标准错误输出附加信息 |

```bash
pdfl run prepress.pdfl magazine.pdf                                    # 终端 JSON
pdfl run prepress.pdfl magazine.pdf --output html --output-file report.html
pdfl run prepress.pdfl magazine.pdf --output pdf --output-file report.pdf
pdfl run prepress.pdfl magazine.pdf --output csv --output-file findings.csv
pdfl run prepress.pdfl magazine.pdf --fail-on warning                  # 严格模式
```

### JSON 报告

```json
{
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
  ]
}
```

同一个 PDF 配同一个脚本，总是产生**逐字节相同的报告**，可用于版本管理和
CI 中的差异比对。

---

## `pdfl compare`

比较两个版本：文本、结构和元数据。

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| 选项 | 默认 | 功能 |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | 格式 |
| `--output-file <file>` | — | 写入文件 |
| `--normalize` | — | 忽略大小写和空白 |
| `--ignore-dates` | — | 比较前遮蔽日期 |
| `--similarity-threshold <0-100>` | `100` | 可接受的最低相似度 |

```bash
pdfl compare approved_v1.pdf new_v2.pdf --normalize --ignore-dates

# 允许 1% 以内的差异，低于该值即为错误
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file diff.html
```

### 工作原理

- 页面按**内容**而非页码对齐：中间插入了一页时，不会把其后的全部页面都报为
  差异。可处理超过一千页的文档。
- 每个对齐的页面都会得到相似度分数，以及变化行的样本（`-` 删除、`+` 新增）。
- 元数据变化记为**警告**；文本变化低于阈值记为**错误**，高于阈值记为**警告**。
- 报告中的 `similarity` 字段给出总体分数。

```
page 4 → 4: similarity 97.8% | -original title | +revised title
```

---

## `pdfl watch`

监视文件夹，校验每一个新到或被修改的 PDF。

```bash
pdfl watch <folder> --script <script.pdfl> [options]
```

| 选项 | 默认 | 功能 |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | 处理哪些文件 |
| `--exclude <glob>` | — | 排除哪些文件 |
| `--output-dir <folder>` | 与 PDF 同目录 | 报告输出位置 |
| `--depth <n>` | `1` | 子目录深度 |
| `--debounce <ms>` | `1000` | 等待文件稳定的时间 |
| `--report json\|csv\|html\|pdf` | `json` | 报告格式 |
| `--fail-fast` | — | 遇到第一个错误即停止 |
| `--once` | — | 处理现有文件后退出 |

```bash
# 印刷厂的收件夹，持续运行
pdfl watch inbox/ --script preflight.pdfl --output-dir reports/ --report html

# CI 的批处理：处理完毕后以最差的退出码退出
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

**debounce** 的存在是因为大文件是逐步写入的：只有文件不再变化才处理，
因此不会读到写了一半的 PDF。

报告写为 `<name>.report.json`（或 `.csv`、`.html`、`.pdf`）。

---

## `pdfl fix`

应用 `fix::` 操作并保存新的 PDF。详见[第 8 章](08-fix.md)。

```bash
pdfl fix original.pdf normalize.pdfl --output out.pdf --dry-run  # 只看会做什么
pdfl fix original.pdf normalize.pdfl --output fixed.pdf          # 实际执行
```

---

## `pdfl inspect`

无需脚本，快速查看 PDF 概要。

```bash
pdfl inspect <file.pdf>
```

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

新文件到达时第一个该运行的命令：几秒钟就能判断是否值得打开。

---

## `pdfl lint`

不执行脚本，分析并报告质量问题。

```bash
pdfl lint <script.pdfl>
```

检出内容：

- 声明后**从未使用**的变量、块参数和函数（加前缀 `_` 可抑制：`_page`）
- **重复**或**空**的 check
- 未知的命名空间（`text::`、`struct::`、`visual::`、`prepress::`、`codes::`、
  `fix::`、`data::`）
- 位于 check 之外的 `assert` / `require`
- 使用了 `fix::`（只能在 `pdfl fix` 中运行）

```bash
$ pdfl lint profile.pdfl
profile.pdfl: warning: variable 'LIMIT' declared and never used
profile.pdfl: warning: check "Fonts" declared 2 times
```

存在警告时退出码为 `1`，可用于 CI。

---

## `pdfl fmt`

格式化脚本：两个空格缩进、统一空白、压缩空行。注释和单位（`3mm` 仍为 `3mm`）
都会保留。

```bash
pdfl fmt <script.pdfl>            # 原地格式化
pdfl fmt <script.pdfl> --check    # 不改动；未格式化时退出码 1
```

```bash
# 在 CI 中强制团队规范
for f in profiles/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

由脚本自身生成文档。

```bash
pdfl doc <script.pdfl> [--output markdown|html]
```

输出内容：配置名、常量表、函数、import，以及每个 check 的标签和校验内容
（`assert` 的消息成为说明）。

```bash
pdfl doc prepress.pdfl > docs/prepress-profile.md
pdfl doc prepress.pdfl --output html > profile.html
```

这是让不读代码的生产管理者了解配置在校验什么的交付物。

---

## `pdfl pack`

把脚本和数据打包成可分发的 `.pdflpkg`。

```bash
pdfl pack <folder> [--name <name>] [--version <version>] [--output <file>]
```

递归收集文件夹中的 `.pdfl`、`.csv`、`.txt`、`.json`、`.xlsx`，并附带记录了
各文件 SHA-256 的 `manifest.json`。打包是确定性的：同一文件夹生成完全相同的
字节。

```bash
pdfl pack profiles/print-shop --name print-profile --version 1.0.0
```

---

## `pdfl add`

安装本地软件包，并校验清单中的哈希。

```bash
pdfl add print-profile.pdflpkg
# 安装到 ./pdfl_profiles/print-profile@1.0.0/

pdfl run pdfl_profiles/print-profile@1.0.0/prepress.pdfl file.pdf
```

若任一文件的哈希与记录不符，安装会被**拒绝** — 损坏或被篡改的包不会进入。

> 远程仓库和数字签名不在本版本范围内：`add` 从本地文件安装。

---

[← 标准库](10-stdlib.md) · [目录](README.md) · [下一章：实用范例 →](12-recipes.md)
