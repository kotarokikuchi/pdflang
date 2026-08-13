# 11. CLI コマンド

[← 標準ライブラリ](10-stdlib.md) · [目次](README.md) · [次：レシピ →](12-recipes.md)

10のコマンド：PDF を扱うもの4つ、スクリプトを扱うもの4つ、配布用が2つ。

| コマンド | 動作 |
|---|---|
| [`run`](#pdfl-run) | スクリプトで PDF を検証 |
| [`compare`](#pdfl-compare) | 2つのバージョンを比較 |
| [`watch`](#pdfl-watch) | フォルダを監視して到着したファイルを検証 |
| [`fix`](#pdfl-fix) | 修正を適用して新しい PDF を保存 |
| [`inspect`](#pdfl-inspect) | PDF の概要を素早く表示 |
| [`lint`](#pdfl-lint) | スクリプトを実行せずに解析 |
| [`fmt`](#pdfl-fmt) | スクリプトを整形 |
| [`doc`](#pdfl-doc) | スクリプトからドキュメントを生成 |
| [`pack`](#pdfl-pack) | プロファイルとデータを1つにまとめる |
| [`add`](#pdfl-add) | パッケージをインストール |

---

## 終了コード

検証を行うすべてのコマンドで共通です。

| コード | 意味 |
|---|---|
| `0` | すべて合格 |
| `1` | 警告のみ |
| `2` | 検証エラー |
| `3` | スクリプトの構文エラー |
| `10` | 文書を読めなかった、またはファイルを書けなかった — 判定に至っていません |

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

スクリプトで PDF を検証します。

```bash
pdfl run <script.pdfl> <input.pdf> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | レポート形式 |
| `--output-file <file>` | — | 標準出力ではなくファイルへ書き出す |
| `--fail-on error\|warning` | `error` | `warning` にすると警告でも終了コード2 |
| `--verbose` | — | 標準エラー出力に追加情報 |

```bash
# 端末に JSON レポート
pdfl run prepress.pdfl magazine.pdf

# 顧客に渡す HTML
pdfl run prepress.pdfl magazine.pdf --output html --output-file report.html

# 監査用 PDF（pdf 形式は常にファイルに出力されます）
pdfl run prepress.pdfl magazine.pdf --output pdf --output-file report.pdf

# 表計算用の CSV
pdfl run prepress.pdfl magazine.pdf --output csv --output-file findings.csv

# 厳格モード：警告も不合格にする
pdfl run prepress.pdfl magazine.pdf --fail-on warning
```

### JSON レポート

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

同じ PDF に同じスクリプトを適用すれば、常に**バイト単位で同一のレポート**が
得られます。バージョン管理や CI での差分比較に使えます。

---

## `pdfl compare`

2つのバージョンを比較します：テキスト、構造、メタデータ。

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--output json\|csv\|html\|pdf` | `json` | 形式 |
| `--output-file <file>` | — | ファイルへ書き出す |
| `--normalize` | — | 大文字小文字と空白を無視 |
| `--ignore-dates` | — | 日付を伏せてから比較 |
| `--similarity-threshold <0-100>` | `100` | 許容する最小類似度 |

```bash
pdfl compare approved_v1.pdf new_v2.pdf --normalize --ignore-dates

# 1% までの差を許容し、それを下回るとエラー
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file diff.html
```

### 動作の仕組み

- ページは番号ではなく**内容で対応付け**られます。途中にページが挿入されても、
  それ以降すべてを差分として報告することはありません。1000ページ超の文書でも
  動きます。
- 対応付いた各ページに類似度スコアと、変化した行のサンプル（`-` 削除、
  `+` 追加）が付きます。
- メタデータの変更は**警告**、テキストの変更はしきい値未満なら**エラー**、
  しきい値以上なら**警告**になります。
- レポートには全体スコアが `similarity` として入ります。

```
page 4 → 4: similarity 97.8% | -original title | +revised title
```

---

## `pdfl watch`

フォルダを監視し、到着または変更された PDF を検証します。

```bash
pdfl watch <folder> --script <script.pdfl> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | 処理対象のファイル |
| `--exclude <glob>` | — | 除外するファイル |
| `--output-dir <folder>` | PDF と同じ場所 | レポートの出力先 |
| `--depth <n>` | `1` | サブフォルダの深さ |
| `--debounce <ms>` | `1000` | ファイルが安定するまでの待ち時間 |
| `--report json\|csv\|html\|pdf` | `json` | レポート形式 |
| `--fail-fast` | — | 最初のエラーで停止 |
| `--once` | — | 既にあるファイルを処理して終了 |

```bash
# 印刷所の受付フォルダを常時監視
pdfl watch inbox/ --script preflight.pdfl --output-dir reports/ --report html

# CI 向けのバッチ実行：処理後、最悪の終了コードで終了
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

**debounce** があるのは、大きなファイルが少しずつ届くためです。ファイルの
変化が止まってから処理するので、途中まで書かれた PDF を読むことがありません。

レポートは `<name>.report.json`（または `.csv`、`.html`、`.pdf`）として
書き出されます。

---

## `pdfl fix`

`fix::` の操作を適用し、新しい PDF を保存します。詳細は[第8章](08-fix.md)。

```bash
pdfl fix <input.pdf> <script.pdfl> --output <output.pdf> [options]
```

```bash
# 何が行われるかだけ確認（保存しない）
pdfl fix original.pdf normalize.pdfl --output out.pdf --dry-run

# 実際に適用
pdfl fix original.pdf normalize.pdfl --output fixed.pdf
```

---

## `pdfl inspect`

スクリプト無しで PDF の概要を表示します。

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

新しいファイルが届いたら最初に実行するコマンドです。数秒で、開く価値があるか
判断できます。

---

## `pdfl lint`

スクリプトを実行せずに解析し、品質上の問題を報告します。

```bash
pdfl lint <script.pdfl>
```

検出する内容：

- 宣言されて**一度も使われない**変数・ブロック引数・関数（`_` を前置すると
  抑制されます：`_page`）
- **重複**または**空**の check
- 未知の名前空間（`text::`、`struct::`、`visual::`、`prepress::`、`codes::`、
  `fix::`、`data::`）
- check の外にある `assert` / `require`
- `fix::` の使用（`pdfl fix` でのみ動作します）

```bash
$ pdfl lint profile.pdfl
profile.pdfl: warning: variable 'LIMIT' declared and never used
profile.pdfl: warning: check "Fonts" declared 2 times
```

警告があれば終了コード `1` になります。CI で使えます。

---

## `pdfl fmt`

スクリプトを整形します：2スペースのインデント、一貫した空白、空行の圧縮。
コメントと単位（`3mm` は `3mm` のまま）は保持されます。

```bash
pdfl fmt <script.pdfl>            # その場で整形
pdfl fmt <script.pdfl> --check    # 書き換えず、未整形なら終了コード1
```

```bash
# CI でチーム標準を強制する
for f in profiles/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

スクリプト自身からドキュメントを生成します。

```bash
pdfl doc <script.pdfl> [--output markdown|html]
```

出力内容：プロファイル、定数の表、関数、import、そして各 check のタグと
検証内容（`assert` のメッセージが説明になります）。

```bash
pdfl doc prepress.pdfl > docs/prepress-profile.md
pdfl doc prepress.pdfl --output html > profile.html
```

コードを読まない制作管理者に、プロファイルが何を検証しているかを伝えるための
成果物です。

---

## `pdfl pack`

スクリプトとデータを配布可能な `.pdflpkg` にまとめます。

```bash
pdfl pack <folder> [--name <name>] [--version <version>] [--output <file>]
```

フォルダ内の `.pdfl`、`.csv`、`.txt`、`.json`、`.xlsx` を再帰的に収集し、
各ファイルの SHA-256 を記録した `manifest.json` を付けます。パッケージは
決定的です：同じフォルダからは同一のバイト列が生成されます。

```bash
pdfl pack profiles/print-shop --name print-profile --version 1.0.0
```

---

## `pdfl add`

ローカルのパッケージをインストールし、マニフェストのハッシュを検証します。

```bash
pdfl add <package.pdflpkg> [--dir <folder>]
```

```bash
pdfl add print-profile.pdflpkg
# ./pdfl_profiles/print-profile@1.0.0/ にインストールされます

pdfl run pdfl_profiles/print-profile@1.0.0/prepress.pdfl file.pdf
```

いずれかのファイルのハッシュが記録と異なる場合、インストールは**拒否**され
ます。改ざんや破損したパッケージは入りません。

> リモートリポジトリと電子署名はこのバージョンには含まれません。`add` は
> ローカルファイルからインストールします。

---

[← 標準ライブラリ](10-stdlib.md) · [目次](README.md) · [次：レシピ →](12-recipes.md)
