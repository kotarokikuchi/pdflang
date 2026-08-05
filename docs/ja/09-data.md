# 9. `data::` 名前空間 — 外部データ

[← `fix::`](08-fix.md) · [目次](README.md) · [次：標準ライブラリ →](10-stdlib.md)

PDF の内容を自前のリストや表と突き合わせる8つの関数。すべてローカルで処理され、
データが外部に出ることはありません。

---

## 9.1 ファイルの置き場所

用語集とデータセットは**実行ディレクトリからの相対パス**を受け取ります：

```pdfl
data::load_glossary("terms/legal.txt")
data::load_dataset("data/batches.csv")
```

参照テーブル（`query_gtin`、`query_medicamento`、`query_postal_code`）は
固定のファイル名で、次の順に探されます：

1. `$PDFL_DATA_DIR`（環境変数）
2. `./dados/`
3. `./`
4. `pdfl add` でインストールされたプロファイル（`pdfl_profiles/*/dados/`）
5. 解析対象の PDF と同じ場所

```bash
PDFL_DATA_DIR=/opt/databases pdfl run profile.pdfl document.pdf
```

見つからない場合、エラーメッセージが置き場所を案内します。プロファイルと
一緒に配布するには `pdfl pack` を使います（[第11章](11-cli.md)）。

---

## 9.2 用語集とデータセット

| 関数 | 動作 |
|---|---|
| `data::load_glossary(file)` | 用語のリスト（1行1語、`#` はコメント） |
| `data::validate_against_reference(file)` | 文書に**現れない**用語のリスト |
| `data::load_dataset(file)` | CSV を行のリストとして読み込む |
| `data::lookup_value(file, key)` | 1列目がキーの行の2列目（無ければ `null`） |

比較は大文字小文字と空白を無視します。

`terms/required.txt`:

```
# すべての約款に必要な用語
waiting period
covered benefits
general conditions
```

```pdfl
check "Glossary and dataset" {
  terms = data::load_glossary("terms/required.txt")
  print("terms in the glossary:", terms.length)

  // 最も直接的な使い方
  missing = data::validate_against_reference("terms/required.txt")
  assert missing.length == 0,
    "clauses missing from the policy: #{missing.join("; ")}"

  rows = data::load_dataset("data/batches.csv")
  print("columns:", rows.first().join(" | "))   // 1行目はヘッダー
  print("records:", rows.length - 1)

  // null は偽なので、そのまま検証できます
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  description = data::lookup_value("data/batches.csv", batch)
  assert description, "batch #{batch} is not in the approved list"
}
```

---

## 9.3 参照テーブル

固定名のファイルを 9.1 の順で探し、**行全体**をリストで返します（無ければ
`null`）。

| 関数 | 参照ファイル | 動作 |
|---|---|---|
| `data::query_gtin(code)` | `gtin.csv` | GTIN で検索（記号は無視） |
| `data::query_medicamento(reg_or_name)` | `medicamentos.csv` | 登録番号または名称の一部で検索 |
| `data::query_postal_code(code)` | `ceps.csv` | 郵便番号（8桁）で検索 |
| `data::validate_address(code, "fragment")` | `ceps.csv` | その郵便番号の住所に文字列が含まれるか |

`dados/gtin.csv`:

```csv
gtin,description,manufacturer
7891234567895,Dipyrone 500mg 20 tablets,Example Labs
```

```pdfl
check "Lookup tables" {
  // 包装から読み取ったコードと突き合わせます
  code = codes::decode_barcode(1)
  product = data::query_gtin(code)
  assert product, "GTIN #{code} is not in the product database"
  print("product:", product.get(2), "| manufacturer:", product.get(3))

  // 登録番号から医薬品情報を引く
  registration = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
  medicine = data::query_medicamento(registration)
  assert medicine, "registration #{registration} not found"

  // 処方箋医薬品なら法定文言が必要です
  band = medicine.get(4)
  assert band != "prescription" || text::require_text("PRESCRIPTION ONLY"),
    "prescription medicine without the mandatory text"

  // 印刷された住所と郵便番号の整合
  assert data::validate_address("01310100", "Avenida Paulista"),
    "printed address does not match the given postal code"
}
```

---

## 9.4 完全な例

```pdfl
// insert_with_databases.pdfl — ローカルデータとの突き合わせ
// 使い方: PDFL_DATA_DIR=./databases pdfl run insert_with_databases.pdfl insert.pdf
profile "insert-with-references" {

  check "Mandatory regulatory terms" tags: ["glossary"] {
    missing = data::validate_against_reference("databases/regulatory_terms.txt")
    assert missing.length == 0,
      "mandatory texts missing: #{missing.join("; ")}"
  }

  check "Product in the database" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} not approved"

    // 登録名が印刷されている必要があります
    name = product.get(2)
    assert text::require_text(name),
      "the name '#{name}' from the database does not appear on the insert"
    print("product verified:", name)
  }

  check "Registration and band" tags: ["regulatory"] {
    registration = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
    med = data::query_medicamento(registration)
    assert med, "registration #{registration} not found"
    assert med.get(4) != "prescription" || text::require_text("PRESCRIPTION ONLY"),
      "prescription band requires the prescription notice"
  }

  check "Manufacturer address" tags: ["data"] {
    assert data::validate_address("01310100", "Avenida Paulista"),
      "manufacturer address does not match the postal code"
  }
}
```

---

[← `fix::`](08-fix.md) · [目次](README.md) · [次：標準ライブラリ →](10-stdlib.md)
