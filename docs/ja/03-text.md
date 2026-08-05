# 3. `text::` 名前空間 — テキスト

[← 型](02-types.md) · [目次](README.md) · [次：`struct::` →](04-struct.md)

ドキュメントのテキストを抽出・正規化・検索・検証する25の関数。

> `[text]` と書かれた引数は**省略可能**です。省略するとドキュメント全体、
> 指定すると渡した文字列に対して動作します。

---

## 3.1 抽出

| 関数 | 動作 |
|---|---|
| `text::extract_all()` | ドキュメント全体のテキスト（ページは改行区切り） |
| `text::extract_from_page(page)` | 指定ページのテキスト（1 起点） |
| `text::extract_from_region(page, region)` | 指定領域内のテキスト（無ければ空文字列） |
| `text::extract_with_normalization()` | 正規化済みのドキュメントテキスト |

```pdfl
check "Extraction" {
  content = text::extract_all()
  assert content.trim() != "", "PDF has no extractable text"

  cover = text::extract_from_page(1)
  assert cover.contains("User Manual"), "cover lacks the expected title"

  // 制作用フッター（InDesign のファイル名や書き出し日時）が
  // 最終版に残ってしまうことがあります
  footer = region(0, 0, 467, 40, "footer")
  doc.pages.each { |page|
    line = text::extract_from_region(page.number, footer)
    assert !line.contains(".indd"),
      "page #{page.number} has a production mark in the footer: #{line.trim()}"
  }
}
```

---

## 3.2 正規化と分割

| 関数 | 動作 |
|---|---|
| `text::normalize([text])` | 小文字化＋空白の圧縮 |
| `text::split_words([text])` | 単語に分割（両端の記号を除去） |
| `text::split_sentences([text])` | 文に分割（`.`、`!`、`?` ＋空白） |
| `text::split_paragraphs([text])` | 段落に分割（空行区切り） |
| `text::count_words([text])` | 単語数 |
| `text::count_characters([text])` | 文字数 |
| `text::detect_language([text])` | `"pt"`、`"en"`、`"es"`、`"unknown"` |

```pdfl
check "Normalization and splitting" {
  require text::normalize("  HELLO   World  ") == "hello world"

  words = text::split_words("Hello, world! (test)")
  require words.length == 3
  require words.first() == "Hello"

  // 添付文書や契約書には読みやすさの実務的な上限があります
  text::split_sentences().each { |sentence|
    assert sentence.length < 400,
      "sentence with #{sentence.length} characters — hard to read"
  }

  require text::count_words() > 100
  assert text::detect_language() == "en",
    "document should be in English, detected: #{text::detect_language()}"
}
```

---

## 3.3 検索と必須内容

| 関数 | 動作 |
|---|---|
| `text::require_text(term)` | その語句が含まれていれば真 |
| `text::forbid_text(term)` | その語句が含まれていなければ真 |
| `text::require_match(regex)` | 正規表現に一致すれば真 |
| `text::forbid_match(regex)` | 正規表現に一致しなければ真 |
| `text::fuzzy_match(a, b)` | 2つの文字列の類似度（0.0〜1.0） |

比較は大文字小文字と空白を無視します。

```pdfl
profile "contract" {
  check "Mandatory clauses" {
    assert text::require_text("governing law"),
      "contract has no governing-law clause"
    assert text::require_text("term of agreement"),
      "contract has no term clause"
    assert text::require_match("\d{4}/\d{4}"),
      "contract number not found"
  }

  check "Forbidden terms" {
    assert text::forbid_text("DRAFT"), "document still marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text was not replaced"
    assert text::forbid_match("\d{2}-\d{2}-\d{4}"), "US-format date found"
  }

  check "Name with tolerance" {
    // 誤字や OCR の揺れが想定される場合
    found = text::extract_from_region(1, region(50, 700, 300, 40))
    similarity = text::fuzzy_match("Paracetamol 750mg", found)
    assert similarity > 0.9,
      "product name differs from expected (#{round(similarity * 100)}% similar)"
  }
}
```

---

## 3.4 個人情報

| 関数 | 動作 |
|---|---|
| `text::detect_personal_data([text])` | 見つかった個人情報のリスト |
| `text::detect_pii([text])` | 同上（別名） |

CPF・CNPJ（ブラジルの納税者番号）、メールアドレス、電話番号を検出します。

> CPF と CNPJ は**チェックディジットが正しい場合のみ**リストに入ります。
> CPF に似ているだけの番号（例：`111.111.111-12`）で誤検出しません。

```pdfl
check "Public document must carry no personal data" {
  found = text::detect_personal_data()
  assert found.length == 0,
    "personal data exposed: #{found.join("; ")}"

  // 各項目は "CPF: 529.982.247-25" の形式です
  text::detect_pii().each { |item| print("found:", item) }
}
```

---

## 3.5 形式の検証

| 関数 | 動作 |
|---|---|
| `text::validate_cpf(text)` | CPF のチェックディジット（mod 11） |
| `text::validate_cnpj(text)` | CNPJ のチェックディジット |
| `text::validate_date_format(text [, format])` | 暦として妥当な日付か |
| `text::validate_phone_format(text)` | ブラジルの電話番号形式 |
| `text::validate_format(text, regex)` | 文字列**全体**が正規表現に一致するか |

日付の形式は `"dd/mm/aaaa"` と `"aaaa-mm-dd"`。第2引数を省略すると両方を
受け付けます。

```pdfl
check "Format validation" {
  require text::validate_cpf("529.982.247-25")
  require !text::validate_cpf("111.111.111-11")    // 同じ数字の連続
  require text::validate_cnpj("11.222.333/0001-81")

  require text::validate_date_format("29/02/2024")   // 2024 はうるう年
  require !text::validate_date_format("29/02/2023")  // 2023 は違う
  require !text::validate_date_format("31/04/2026")  // 4月は30日まで

  require text::validate_phone_format("(11) 98765-4321")

  // 工場のロット番号形式
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  assert text::validate_format(batch, "L\d{4}-\d{2}"),
    "batch code does not follow the L0000-00 pattern: #{batch}"
}
```

---

## 3.6 比較と診断

| 関数 | 動作 |
|---|---|
| `text::diff(a, b)` | 変化した行のリスト（`-` 削除、`+` 追加） |
| `text::detect_rasterized_text()` | 画像化されたテキストがあれば真 |

```pdfl
check "Comparison and diagnostics" {
  changes = text::diff(text::extract_from_page(1), text::extract_from_page(2))
  print("changed lines:", changes.length)

  // スキャンやアウトライン化されたページは検索も
  // スクリーンリーダーも効きません
  assert !text::detect_rasterized_text(),
    "there are pages with rasterized text (scanned or outlined)"
}
```

> 2つの**ファイル**を比較するには `pdfl compare` を使います。ページを自動で
> 対応付けます。[第11章](11-cli.md)を参照してください。

---

## 3.7 完全な例

```pdfl
// legal_document.pdfl — 契約書の検証
profile "standard-contract" {

  check "Required content" tags: ["legal"] {
    assert text::require_text("governing law"), "no governing-law clause"
    assert text::require_text("term of agreement"), "no term clause"
    assert text::require_match("\d{4}/\d{4}"), "no contract number"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("XXX+"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    found = text::detect_personal_data()
    assert found.length == 0,
      "personal data in a public document: #{found.join("; ")}"
  }

  check "Text quality" tags: ["text"] {
    assert text::detect_language() == "en", "document is not in English"
    assert !text::detect_rasterized_text(), "rasterized text blocks search"
    require text::count_words() > 200
  }
}
```

---

[← 型](02-types.md) · [目次](README.md) · [次：`struct::` →](04-struct.md)
