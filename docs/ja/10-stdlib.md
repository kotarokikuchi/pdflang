# 10. 標準ライブラリ

[← `data::`](09-data.md) · [目次](README.md) · [次：CLI コマンド →](11-cli.md)

リストと文字列のメソッド、およびスクリプトのどこでも使えるグローバル関数。

---

## 10.1 リストのメソッド

| メソッド | 動作 |
|---|---|
| `list.each { \|item\| ... }` | 各要素についてブロックを実行 |
| `list.each_with_index { \|item, i\| ... }` | 位置（**0** 起点）も受け取る |
| `list.all { \|item\| ... }` | すべてが条件を満たせば真（空リストは真） |
| `list.any { \|item\| ... }` | いずれかが条件を満たせば真（空リストは偽） |
| `list.filter { \|item\| ... }` | 条件を満たす要素だけの新しいリスト |
| `list.map { \|item\| ... }` | 各要素を変換した新しいリスト |
| `list.length` | 要素数（`length()` でも同じ） |
| `list.contains(value)` | その値が含まれていれば真 |
| `list.get(n)` | n番目の要素（**1** 起点） |
| `list.first()` / `list.last()` | 最初／最後の要素（空リストでは `null`） |
| `list.join([separator])` | 文字列に連結（既定の区切りは `", "`） |

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

  names = doc.fonts.map { |f| f.name }
  print("fonts:", names.join(", "))

  // get は1起点：get(1) が最初の要素
  row = data::load_dataset("data/batches.csv").get(2)
  print("first column:", row.get(1))

  // 空リストでも安全：null は偽
  spots = prepress::detect_spot_colors()
  assert !spots.first() || spots.first() == "Varnish",
    "unexpected special ink: #{spots.first()}"
}
```

---

## 10.2 文字列のメソッド

| メソッド | 動作 |
|---|---|
| `text.contains(sub)` | 部分文字列を含むか |
| `text.starts_with(sub)` | それで始まるか |
| `text.ends_with(sub)` | それで終わるか |
| `text.trim()` | 前後の空白を削除 |
| `text.to_uppercase()` | すべて大文字 |
| `text.to_lowercase()` | すべて小文字 |
| `text.length` | 文字数 |

```pdfl
check "String methods" {
  title = doc.title
  require title.length > 0
  require title.trim() == title          // 余分な空白が無い
  assert !title.to_lowercase().contains("draft"),
    "title still marked as draft"

  code = codes::decode_barcode(1)
  assert code.starts_with("789"), "GTIN is not Brazilian"
  assert doc.filename.ends_with(".pdf"), "unexpected extension"
}

check "contains on each type" {
  // 文字列：テキスト内の「部分」を探します
  require "final document".contains("final")

  // リスト：「要素そのもの」を探します
  require ["a", "b"].contains("a")
  require !["ab"].contains("a")      // "a" はリストの要素ではありません
}
```

---

## 10.3 グローバル関数

| 関数 | 動作 |
|---|---|
| `min(a, b)` / `max(a, b)` | 小さい方／大きい方 |
| `abs(x)` | 絶対値 |
| `round(x)` | 最も近い整数に丸める |
| `print(...)` | 値を空白区切りで出力（**標準エラー出力**） |
| `region(x, y, w, h [, name])` | 領域を作成（[第2章](02-types.md#25-region--ページ上の領域)） |

`print` が標準エラー出力に出るため、`> report.json` でレポートだけを
取り出せます。

```pdfl
check "Global functions" {
  const A4_WIDTH = 595.0
  const TOLERANCE = 5.0

  // abs は寸法を許容差付きで比較するのに欠かせません
  doc.pages.each { |page|
    assert abs(page.width - A4_WIDTH) < TOLERANCE,
      "page #{page.number} is outside A4: #{page.width}pt"
  }

  // round はメッセージを読みやすくします
  // round 無し："217.4453125 DPI" / round 有り："217 DPI"
  doc.images.each { |img|
    assert img.dpi >= 300,
      "image on page #{img.page_number}: #{round(img.dpi)} DPI"
  }

  print("document:", doc.filename)
  print("pages:", doc.page_count, "| fonts:", doc.fonts.length)
}
```

---

## 10.4 よく使うパターン

```pdfl
// 失敗した要素の数を数える
check "Problem count" {
  bad = doc.images.filter { |i| i.dpi < 300 }
  assert bad.length == 0,
    "#{bad.length} of #{doc.images.length} images below 300 DPI"
}

// 失敗した要素をメッセージに並べる
check "List in the message" {
  // 連結は同じ行に：ドットの前で改行しないでください
  problems = doc.pages.filter { |p| !p.has_trim_box }.map { |p| p.number }
  assert problems.length == 0,
    "pages without a TrimBox: #{problems.join(", ")}"
}

// 許容差付きの検証
function close_to(value, target, tolerance) {
  abs(value - target) < tolerance
}

check "With tolerance" {
  doc.pages.each { |page|
    assert close_to(page.width, 595.0, 2.0),
      "page #{page.number}: width #{page.width}pt (expected 595 ± 2)"
  }
}

// 空のドキュメントでエラーにしない
check "Defensive" {
  // 短絡評価により、空リストで first() を呼びません
  assert doc.pages.length == 0 || doc.pages.first().width > 0,
    "the first page has no width"
}
```

---

[← `data::`](09-data.md) · [目次](README.md) · [次：CLI コマンド →](11-cli.md)
