# 11. 命令行

[← 标准库](10-stdlib.md) · [目录](README.md) · [下一章：实用范例 →](12-recipes.md)

共 11 个命令：4 个处理 PDF，4 个处理脚本，2 个用于分发，1 个用于 shell。

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
| [`completions`](#pdfl-completions) | 输出所用 shell 的补全脚本 |

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

## 全局选项

| 选项 | 功能 |
|---|---|
| `--quiet` | 关闭 stderr 上的进度与确认信息 |

`--quiet` 放在子命令之前或之后都有效，且对每个子命令都有效。它去掉的是人想看、
流水线不想看的那些行——`report saved to …`、`watching …`、`watch` 的逐文件结果。
它**不会**去掉错误：安静的一次运行若失败，仍然会说明原因。

它也不会关闭 `print()`。那是脚本自己的输出，吞掉它就改变了脚本的行为。若不需要，
请重定向 stderr。

`--quiet` 优先于 `--verbose`。

---

## `pdfl run`

用脚本校验 PDF。

```bash
pdfl run <script.pdfl> <input.pdf> [options]
```

| 选项 | 默认 | 功能 |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | 报告格式 |
| `--output-file <file>` | — | 写入文件而非标准输出 |
| `--fail-on error\|warning` | `error` | 设为 `warning` 时警告也退出码 2 |
| `--verbose` | — | 标准错误输出附加信息 |
| `--var 名称=值` | — | 脚本以 `vars.名称` 读取的值；可重复 |
| `--tags TAG` | — | 仅运行带该标签的 check；可重复。没有任何 check 带的标签会报错，而不是空通过 |

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

同一个 PDF 配同一个脚本，总是产生**逐字节相同的报告**，可用于版本管理和
CI 中的差异比对。

`schema_version` 是第一个键，消费方可以先据此分支，再去解析其余内容。它仅在读取
旧输出的一方会被破坏时才提升；新增字段不会提升它。

### SARIF 与 JUnit

再加两种格式，让结果出现在团队本来就会看的地方，而不是无人打开的日志里。

```bash
# GitHub code scanning：发现会成为拉取请求上的注释
pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif

# 任何 CI 的测试面板：每个 check 一个测试，通过的也算
pdfl run prepress.pdfl magazine.pdf --output junit --output-file pdfl.xml
```

在 SARIF 中，发现锚定在**脚本**上，而不是 PDF 上：我们知道的行号是 check 的行号，
而 PDF 通常是流经 CI 的产物，并不是仓库里的文件——指向它只会在一个不存在的路径上
加注释。受检文件放在 `properties.inputFile` 里，诊断标识符放在
`partialFingerprints` 里，正是后者让 GitHub 认得出自己已经见过的发现，而不是每次
运行都重新开一条。

在 JUnit 中，每个运行过的 check 都是一个测试用例，包括那些什么都没发现的。只列出
失败的格式会把一次干净的运行报告成零个测试，而 CI 会把它读作从未发生的运行。
`info` 级别的发现不会让用例失败，它写入 `<system-out>`。

```yaml
- name: Preflight
  run: pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif
  # 退出码 2 表示文件被拒，但上传仍然要进行
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: pdfl.sarif
```

---

## `pdfl compare`

比较两个版本：文本、结构和元数据。

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| 选项 | 默认 | 功能 |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | 格式 |
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
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | 报告格式 |
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
pdfl fix <input.pdf> <script.pdfl> --output <output.pdf> [options]
```

| 选项 | 默认 | 功能 |
|---|---|---|
| `--output <文件>` | — | 输出的 PDF（必填） |
| `--dry-run` | — | 只列出将执行的操作，不保存 |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | 报告格式 |
| `--report-file <文件>` | — | 把报告写入文件 |

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

`--json` 以数据形式返回同样的摘要。

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

`--json` 以数据形式返回同样的警告。

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
pdfl doc <script.pdfl> [--output markdown|html|json]
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

递归收集文件夹中的 `.pdfl`、`.csv`、`.txt`、`.json`，并附带记录了各文件
SHA-256 的 `manifest.json`。打包是确定性的：同一文件夹生成完全相同的字节。

电子表格（`.xlsx`、`.xls`、`.ods`）**不会**打包，`pack` 会说明留下了哪个文件。
没有任何 `data::` 函数能打开它们，装进去只会得到一个安装顺利、却在第一次查询时
失败的软件包。

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

## `pdfl completions`

把所用 shell 的补全脚本输出到 stdout。

```bash
pdfl completions <bash|zsh|fish|elvish|powershell>
```

```bash
# bash，当前用户
pdfl completions bash > ~/.local/share/bash-completion/completions/pdfl

# zsh —— 放在 $fpath 上的任意位置
pdfl completions zsh > ~/.zfunc/_pdfl

# fish
pdfl completions fish > ~/.config/fish/completions/pdfl.fish
```

stdout 上不会有别的内容，因此可以直接重定向进补全目录。升级之后请重新生成：脚本
是由输出它的那个可执行文件的命令与参数构建出来的。

---

[← 标准库](10-stdlib.md) · [目录](README.md) · [下一章：实用范例 →](12-recipes.md)
