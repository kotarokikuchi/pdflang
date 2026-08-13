# 1. PDFLang 言語

[← 目次](README.md) · [次：ドキュメント型 →](02-types.md)

PDFLang はプログラミングをしない人が読めるように設計されています。クラスも
継承も型宣言もセミコロンもありません。スクリプトは、ほぼ自然な言葉で書かれた
チェックのリストです。

---

## 1.1 スクリプトの構造

```pdfl
// コメントは2つのスラッシュで始まり、行末まで続きます。

profile "profile-name" {         // profile は任意：セットに名前を付けて
                                 // まとめます。名前はレポートに表示されます。

  const LIMIT = 300%             // 定数：慣例として大文字

  check "Check Name" {           // 各 check はレポートの1セクションになります
    require doc.page_count > 0   // 検証1つ
  }

  check "Another Check" {        // check はいくつでも書けます
    require doc.title != ""
  }
}
```

`profile` は省略できます — スクリプトは check の列挙だけでも構いません：

```pdfl
check "Simple" {
  require doc.page_count > 0
}
```

### check のタグ

タグはレポート内で check を分類・絞り込むために使います：

```pdfl
check "Ink within limit" tags: ["prepress", "colors"] {
  require prepress::validate_tac_limits(300)
}
```

### check の重大度

既定では失敗した check は**エラー**で、実行は 2 で終了します。check は助言的
であると宣言できます。

```pdfl
check "画像の解像度" severity: warning {
  require !visual::detect_low_resolution(300)
}
```

`error`（既定）、`warning`、`info` の3つ。警告と情報は実行を失敗させず、1 と
0 で終了します。ただし `--fail-on warning` を渡した場合は別で、これによりスク
リプトを変えずに CI 側で厳しさを決められます。

`tags:` と `severity:` はどちらの順序でも書けます。

> check の中で起きた実行時エラー——変数の綴り間違い、ファイルの欠落——は、
> check が何を宣言していてもエラーのままです。壊れたスクリプトは助言では
> ありません。

---

## 1.2 検証の2つの書き方

すべての検証は `require` か `assert` を使います。違いは、失敗したときに
レポートへ出るメッセージだけです。

```pdfl
check "Comparing both forms" {

  // require: 式そのものからメッセージが生成されます。
  // 失敗するとレポートには次のように出ます：
  //   "requirement not met: doc.page_count > 0"
  require doc.page_count > 0

  // assert: 読み手に見せたいメッセージを自分で書きます。
  // 失敗するとそのまま表示されます：
  //   "PDF has no title in its metadata"
  assert doc.title != "", "PDF has no title in its metadata"
}
```

**使い分けの目安：** 式が自明なときは `require`、レポートを読む人が
スクリプトを知らなくても問題を理解できる必要があるときは `assert`。

### 1つ失敗しても他は止まりません

```pdfl
check "Three independent validations" {
  assert doc.page_count > 100, "too few pages"    // 失敗
  assert doc.title != "", "no title"              // それでも実行される
  assert doc.author != "", "no author"            // これも実行される
}
```

レポートには**すべて**の問題が一度に並びます。これは意図的です。ファイルを
返される側は、修正すべき点の完全なリストを求めているからです。

check どうしでも同じです。ある check が実行時エラー（未定義の変数など）に
なっても、それは診断として記録され、残りの check は動き続けます。

---

## 1.3 値と型

### 数値と単位

```pdfl
check "Numbers" {
  x = 42          // 整数
  y = 2.5         // 小数

  // 長さの単位は自動的にポイントへ変換されます（1 pt = 1/72 インチ）：
  a = 3mm         // 8.5039... pt
  b = 2.5cm       // 70.866... pt
  c = 1in         // 72 pt
  d = 10pt        // 10 pt

  // パーセントは数値をそのまま保ちます：
  limit = 300%    // 300

  require a < b            // すべてポイントなので直接比較できます
  require c == 72.0
  require limit == 300
}
```

`8.504` ではなく `3mm` と書けることが要点です。ミリで考える人にとって自然に
読め、変換ミスも起きません。

### 文字列

```pdfl
check "Strings" {
  simple = "plain text"

  // 補間：#{...} は任意の式の値を埋め込みます
  name = "document.pdf"
  message = "Analyzing #{name} with #{doc.page_count} pages"

  // エスケープ：\n（改行）、\t（タブ）、\"（引用符）、\\（バックスラッシュ）
  quoted = "he said \"hello\""

  // 未知のバックスラッシュはそのまま通ります — 正規表現を
  // 二重エスケープなしで書けます：
  pattern = "\d{3}\.\d{3}\.\d{3}-\d{2}"

  require message.contains("pages")
}
```

### 真偽値と「真」とみなされる値

```pdfl
check "True and false" {
  yes = true
  no = false

  // 偽なのは false と null だけです。それ以外はすべて真 —
  // 0 も、空文字列も、空リストも真です。
  require 0        // 通ります（0 は真）
  require ""       // 通ります（空文字列は真）

  // したがって内容を確認するには明示的に比較します：
  require doc.title != ""              // 正しい
  require doc.pages.length > 0         // 正しい
}
```

これは、見つからないときに `null` を返す関数で効いてきます：

```pdfl
check "Taking advantage of null" {
  description = data::lookup_value("batches.csv", "L2026-08")
  // null は偽なので、そのまま書けます：
  assert description, "batch not found in the table"
}
```

### リスト

```pdfl
check "Lists" {
  numbers = [1, 2, 3]
  words = ["a", "b", "c"]
  mixed = [1, "two", true]

  require numbers.length == 3
  require numbers.contains(2)
  require words.join(", ") == "a, b, c"

  // アクセスは1起点：最初の要素は1番です
  require numbers.get(1) == 1
  require numbers.first() == 1
  require numbers.last() == 3
}
```

---

## 1.4 演算子

```pdfl
check "Operators" {
  // 比較
  require 10 > 5
  require 10 >= 10
  require 3 < 4
  require 3 <= 3
  require "a" == "a"
  require "a" != "b"

  // 算術
  require 2 + 3 == 5
  require 10 - 4 == 6
  require 3 * 4 == 12
  require 10 / 4 == 2.5        // 割り切れない除算は小数になります
  require 10 / 5 == 2          // 割り切れる場合は整数のまま

  // 論理（短絡評価：右辺は必要なときだけ評価されます）
  require true && true
  require false || true
  require !false

  // 短絡評価の実例：ページが無ければ右辺は評価されず、
  // 空のドキュメントでもエラーになりません。
  require doc.page_count == 0 || doc.pages.first().width > 0
}
```

---

## 1.5 ブロック：各要素に対する繰り返し

ブロックは波かっこで囲み、縦棒の間に引数を書きます。「各ページについて〜する」
と読めます。

```pdfl
check "Walking through pages" {

  // each: 各要素についてブロックを実行します
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  // each_with_index: 位置（0, 1, 2...）も受け取ります
  doc.fonts.each_with_index { |font, i|
    print("font", i, ":", font.name)
  }

  // all: すべての要素が条件を満たせば真
  require doc.fonts.all { |f| f.is_embedded }

  // any: いずれかの要素が条件を満たせば真
  require doc.pages.any { |p| p.extract_text() != "" }

  // filter: 条件を満たす要素だけを残します
  blank = doc.pages.filter { |p| p.extract_text() == "" }
  assert blank.length == 0,
    "#{blank.length} blank page(s)"

  // map: 各要素を変換して新しいリストにします
  names = doc.fonts.map { |f| f.name }
  print("fonts in use:", names.join(", "))
}
```

ブロックは連結できます — ただし**同じ行**に書き、ドットの前で改行しないで
ください：

```pdfl
check "Chaining" {
  // 埋め込まれていないフォントの名前だけをカンマで連結
  problems = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  assert problems.length == 0,
    "fonts not embedded: #{problems.join(", ")}"
}
```

行が長くなりすぎる場合は、連結を切るのではなく名前付きの段階に分けます。
そのほうが読みやすくもあります：

```pdfl
check "Named steps" {
  loose = doc.fonts.filter { |f| !f.is_embedded }
  names = loose.map { |f| f.name }
  assert names.length == 0, "fonts not embedded: #{names.join(", ")}"
}
```

---

## 1.6 関数：ルールに名前を付ける

同じ検証が何度も出てくるなら、名前を付けましょう：

```pdfl
// 関数の値は「最後の式」の値です — return はありません。
function is_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}

function exceeds_ink(page, limit) {
  page.tac > limit
}

check "Format and ink" {
  // これで check がほとんど文章のように読めます
  require doc.pages.all { |p| is_a4(p) }

  doc.pages.each { |page|
    assert !exceeds_ink(page, 300), "page #{page.number} has too much ink"
  }
}
```

関数の決まり：

- 引数は関数の中だけで有効です。
- 関数から別の関数を呼べます。
- 再帰は可能ですが200回までです（暴走したスクリプトがプロセスを止めないため）。

---

## 1.7 import：プロファイル間での共有

共通のルールを1つのファイルにまとめ、必要な場所で読み込みます。

`library.pdfl`:

```pdfl
// チーム内で共有する定数と関数
const OFFSET_TAC = 300%
const DEFAULT_BLEED = 3mm

function a4_page(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}
```

`magazine.pdfl`:

```pdfl
// パスは「このファイル」からの相対です
import "library.pdfl"

check "Format" {
  // OFFSET_TAC と a4_page は import から来ています
  require doc.pages.all { |p| a4_page(p) }
  require prepress::validate_tac_limits(OFFSET_TAC)
}
```

同じファイルは**一度だけ**読み込まれます。複数のスクリプトが読み込んでも、
循環 import で止まることはありません。

---

## 1.8 rule：ページごとの検証

`rule` は各ページに対して1回ずつ実行される check です。ページは `page`
変数に入っています：

```pdfl
// "on" なし：すべてのページで実行されます
rule "Every page has text" {
  assert page.extract_text().trim() != "",
    "page #{page.number} is blank"
}
```

`on` を付けると、対象ページを選べます：

```pdfl
rule "Body pages numbered" on doc.pages.filter { |p| p.number > 2 } {
  footer = region(0, 0, page.width, 60)
  assert text::extract_from_region(page.number, footer) != "",
    "page #{page.number} has no page number in the footer"
}
```

> **構文上の注意：** `on` の選択式がプロパティで終わる場合（例：`on doc.pages`）
> は、かっこで囲んでください。囲まないと本体の `{` がそのプロパティ呼び出しの
> ブロックとして解釈されます：
>
> ```pdfl
> rule "Example" on (doc.pages) {     // かっこが必要
>   require page.width > 0
> }
> ```

---

## 1.9 変数とスコープ

```pdfl
const GLOBAL = 100          // ファイル全体で有効

check "Scope" {
  local = 42                // この check の中だけ

  doc.pages.each { |page|
    inner = page.width      // このブロックの中だけ
    require inner > 0
  }

  require local == 42       // まだ有効
  require GLOBAL == 100     // まだ有効
}
```

慣例として定数は大文字、変数は小文字です。言語が強制するわけではありませんが、
例と配布プロファイルはこれに従っています。

---

### コマンドラインから渡す値

`pdfl run` の `--var 名前=値` は、スクリプトからは `vars.名前` として、常に文字列
として読めます。1つのプロファイルがほとんど同じ5つのコピーに増えるのを防ぐのが
これです。

```pdfl
check "指示書と一致するか" {
  assert doc.title.contains(vars.order),
    "ファイルには \"#{doc.title}\"、指示書は #{vars.order}"
}
```

```bash
pdfl run intake.pdfl received.pdf --var order=SO-4471
```

渡されていない名前は空文字列ではなく、**それを与えるフラグの名前を示すエラー**に
なります。何もない値と比較する check は素通りしてしまい、誰も検証していない
ファイルを合格として報告してしまうからです。

---

## 1.10 受け取る人に役立つメッセージ

レポートの質は、あなたが書くメッセージで決まります。比べてみましょう：

```pdfl
check "Poor messages" {
  require doc.pages.all { |p| p.tac <= 300 }
  // レポート: "requirement not met: doc.pages.all() { ... }"
  // — どのページがどれだけ超えたのか受け取る側にはわかりません
}

check "Good messages" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "Page #{page.number}: ink coverage #{page.tac}% (max 300%)"
  }
  // レポート: "Page 7: ink coverage 324% (max 300%)"
  // — オペレーターは何を直せばよいか正確にわかります
}
```

エラーではない補足情報には `print()` を使います。標準エラー出力に出るので、
レポートを汚しません：

```pdfl
check "Context" {
  print("Analyzing", doc.page_count, "pages")
  print("Fonts:", prepress::list_fonts().join(", "))
  require doc.page_count > 0
}
```

---

## 1.11 よくあるエラー

| メッセージ | 原因 | 対処 |
|---|---|---|
| `expected end of line after statement` | 1行に2つの文 | 1行に1つの文 |
| `unknown variable: x` | 代入前の使用、またはスコープ外 | 同じ階層で先に宣言する |
| `unknown function: text::xyz` | 名前の誤りか存在しない関数 | 該当する名前空間の章を確認 |
| `fix:: is only available in the 'pdfl fix' command` | `pdfl run` で `fix::` を使用 | `pdfl fix input.pdf script.pdfl --output out.pdf` を使う |
| `unknown unit: 'kg'` | 不正な単位 | `pt`、`mm`、`cm`、`in`、`%` を使う |
| `expected '{' with the rule body` | `on` の選択式がプロパティで終わっている | 選択式をかっこで囲む |
| `unexpected expression: Dot` | 連結が複数行に分かれている | `.method` を同じ行に置くか、中間変数を使う |

実行前には常にこれを行う価値があります：

```bash
pdfl lint my_profile.pdfl    # 未使用変数、重複した check など
pdfl fmt my_profile.pdfl     # 書式を統一
```

---

[← 目次](README.md) · [次：ドキュメント型 →](02-types.md)
