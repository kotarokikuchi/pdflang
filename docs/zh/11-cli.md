# 11. 命令行

[← 标准库](10-stdlib.md) · [目录](README.md) · [下一章：实用范例 →](12-recipes.md)

共 12 个命令：4 个处理 PDF，5 个处理脚本，2 个用于分发，1 个用于 shell。

| 命令 | 功能 |
|---|---|
| [`run`](#pdfl-run) | 用脚本校验 PDF |
| [`compare`](#pdfl-compare) | 比较两个版本 |
| [`watch`](#pdfl-watch) | 监视文件夹并校验新到的文件 |
| [`fix`](#pdfl-fix) | 应用修改并保存新的 PDF |
| [`inspect`](#pdfl-inspect) | 快速查看 PDF 概要 |
| [`lint`](#pdfl-lint) | 不执行地分析脚本 |
| [`fmt`](#pdfl-fmt) | 格式化脚本 |
| [`test`](#pdfl-test) | 用脚本跑一整个文件夹的 PDF，并比对每份报告 |
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
| `--events` | — | 用系统通知唤醒，而不是定时器——不适用于网络共享 |
| `--journal <文件>` | — | 只追加的已校验记录；再次运行会跳过它覆盖的文件 |
| `--timeout <秒>` | — | 超过这个秒数就杀掉该文件的分析，并报告为被拒 |
| `--jobs <n>` | `1` | 同时校验的文件数；`0` 表示每个 CPU 一个 |
| `--once` | — | 处理现有文件后退出 |

```bash
# 印刷厂的收件夹，持续运行
pdfl watch inbox/ --script preflight.pdfl --output-dir reports/ --report html

# CI 的批处理：处理完毕后以最差的退出码退出
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

`--jobs` 对这一轮要处理的一切都有效，批量模式和一批文件同时到达时都一样。每个
文件由它自己的 `pdfl` 进程校验（与 `pdfl test` 的理由相同），而渲染报告的是当前
这个进程，所以无论 `--jobs` 取多少，写出的文件都完全相同。8 个 41 页的文件：
`--jobs 1` 用 9.5 秒，`--jobs 0` 用 1.2 秒。

加上 `--fail-fast` 后，一旦有文件失败就不再启动新的；已经在跑的会跑完，因为中途
杀掉会留下写了一半的报告。报告按文件被发现的顺序写出，所以不管同时跑了多少个，
一批打印出来的行都一样。

等待恰好在最新的那个文件稳定下来时结束，所以在等待期间到达的文件不会再被多押
一整个间隔。

默认按定时器列目录；加上 `--events` 则改为等待操作系统的通知。默认用定时器是量过
的结论：每 200 毫秒列一万个文件不产生可测量的 CPU 占用，而且无论哪种方式，延迟都
由稳定等待时间主导——在本地文件夹上，两种模式的完成时间相差不到百分之一秒。

不要在网络共享上使用 `--events`。在 NFS 或 SMB 挂载上，inotify 只报告本机写入的
内容，别的机器送来的文件永远不会被发现，而且 watch 对此一声不吭。真正划算的场景
是一台机器监视很多文件夹，或者列目录本身很贵。如果监视器建不起来，watch 会说明
情况并退回定时器，而不是就此沉默。

**debounce** 的存在是因为大文件是逐步写入的：只有文件不再变化才处理，
因此不会读到写了一半的 PDF。

### journal：把被打断的批处理跑完

五千个文件，跑到第四千台机器重启了。没有记录，下一次就得从第一个重来。

```bash
pdfl watch inbox/ --script offset.pdfl --once --journal batch.jsonl
```

每个文件一行 JSON，校验一个就追加一行：

```json
{"input":"inbox/cover.pdf","sha256":"9f2b…","status":"FAIL","errors":2,"warnings":0,"exit":2}
```

用同一个 journal 再跑一次，它覆盖到的文件会被跳过。但结论不会被跳过：续跑的批处理
即便跳过了一个被拒的文件，退出码仍然是 `2`——journal 是这一批的记录，退出码是这一
批的结论。因为"早就见过这个失败"而报告干净，会是这个工具能犯的最严重的错误。

文件是按**字节**匹配的，不看名字，也不看时间戳。把 `cover.pdf` 换成另一个
`cover.pdf`，它会被重新校验：哈希和记录里的不一样。

不加 `--journal` 就什么都不写。这个工具不保存自己的状态；这是一个你指名要的文件，
和报告一样。行里也没有时间戳：journal 回答一个文件*是否*被校验过、结果如何，旁边
的报告回答*是什么*，文件系统回答*什么时候*——这样重跑一次和第一次逐字节相同，与
这里的其他一切一致。

每行都是逐条写入的，所以崩溃留下的内容在它覆盖的范围内是真的。读不懂的 journal
会指出是第几行并停止运行：根据读错的记录去跳过文件，比从头再来更糟。

### `--timeout`：一个坏文件不能拖住整批

```bash
pdfl watch inbox/ --script offset.pdfl --once --timeout 60
```

分析超过 `60` 秒的文件会被杀掉，报告方式和无法读取的 PDF 一样——带一条发现的报告，
`check_name: "timeout"`——因此它会打印、写入磁盘，并像其他任何结论一样进入
journal。没有什么会被悄悄跳过，批处理会转向下一个文件，而不是卡在这一个上。

```json
{"input":"inbox/adversarial.pdf","sha256":"7a1c…","status":"FAIL","errors":1,"warnings":0,"exit":2}
```

`.pdfl` 语言里没有任何东西能让脚本故意卡死解释器——递归有深度限制——所以
`--timeout` 是为脚本造不成的情况而存在的：pdfium 在畸形或恶意的 PDF 上死循环或
卡住。不加这个参数，一个文件的分析会一直等下去，这也是这个选项出现之前唯一的
行为。

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

## `pdfl test`

用脚本跑遍文件夹里的每个 PDF，并把每份报告与记录在旁边的那份比对。配置文件一旦
开始查出不一样的东西，失败的是测试，而不是下游某个人的一天。

```bash
pdfl test <script.pdfl> [--dir <文件夹>] [--update]
```

| 选项 | 默认 | 功能 |
|---|---|---|
| `--dir <文件夹>` | 脚本旁边的 `tests/` | 用例 PDF 所在的位置 |
| `--update` | — | 记录预期报告，而不是比对 |
| `--jobs <n>` | `1` | 同时运行的用例数；`0` 表示每个 CPU 一个 |

一个用例，就是一个 PDF 和它应有的报告，并排放着：

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
# 第一次：把脚本现在查到的东西记录下来
pdfl test prepress.pdfl --update

# 此后
pdfl test prepress.pdfl
```

```
ok   approved.pdf
FAIL heavy_ink.pdf
     error_count: expected 1, got 0
     missing:    PDFL-093751a2 [error] Ink coverage (line 12): page 7: 324% ink (limit 300%)
1 passed, 1 failed
```

失败时给出的是变化本身——计数、结论，以及哪些发现出现了、哪些消失了——而不是把
两份 JSON 并排打印出来。

记录始终是一个有意的动作：会自动刷新自身基线的运行永远不会失败。先读差异，确认
这正是你想要的改动，再用 `--update` 重新记录。

预期报告就是 `pdfl run` 生成的那份，只是把 `input_file` 缩成文件名——会随调用目录
变化的基线不算基线。打不开的 PDF 只让它自己的用例失败，其余照常运行。

退出码：`0` 全部通过，`2` 至少一个失败，`10` 文件夹读不了或里面没有 PDF。

### 同时运行多个用例

每个用例都作为独立的 `pdfl` 进程运行，因此 `--jobs` 带来的是真正的并行：在 8 个
41 页的文件上，`--jobs 1` 用了 8.9 秒，`--jobs 8` 用了 1.1 秒。同一进程内的线程
做不到这一点——pdfium 用一把互斥锁把所有调用串行化，实测线程版本比顺序执行还*慢*。

默认值是 `1`，因为每个任务都是一个把文档装进内存的进程，而这个工具本就是为可能
极大的文件而存在的。用例规模普通时可以调高：`--jobs 0` 表示每个 CPU 一个。

输出顺序不会因 `--jobs` 而改变：无论哪个子进程先结束，用例都按被发现的顺序判定。

PDF 打不开的用例与其他用例一样被判定——它的报告把原因作为一条发现，因此"这个文件
应当以无法读取为由被拒"本身就可以是一个测试。该报告按传入的样子记录文件名，所以
如果基线要提交进版本库，请用**相对**的 `--dir` 记录。

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
