# 7. `codes::` 名前空間 — バーコードと QR コード

[← `prepress::`](06-prepress.md) · [目次](README.md) · [次：`fix::` →](08-fix.md)

ドキュメントに印刷されたバーコードと QR コードを検出・デコード・検証する13の
関数。

> スキャンはページを高解像度でレンダリングし、いずれかの `codes::` 関数を
> 最初に使ったときに**一度だけ**実行されます。この名前空間を使わない
> スクリプトはその負荷を負いません。

対応形式：EAN-8/13、UPC-A/E、Code 128、Code 39、ITF、QR コード、Data Matrix、
Aztec、PDF417 など。

---

## 7.1 検出

| 関数 | 動作 |
|---|---|
| `codes::detect_barcodes()` | バーコードがあれば真 |
| `codes::detect_qrcodes()` | QR コードがあれば真 |
| `codes::count_barcodes()` | 検出されたコードの総数 |
| `codes::get_barcode_type(n)` | n番目の形式（`"EAN_13"`、`"QR_CODE"`…） |
| `codes::get_barcode_location(n)` | 位置 `[ページ, x, y]`（ポイント、左下原点） |

```pdfl
check "Codes present" {
  assert codes::detect_barcodes(), "no barcode found in the artwork"
  assert codes::detect_qrcodes(), "the traceability QR code is missing"

  total = codes::count_barcodes()
  assert total == 2, "expected 2 codes (EAN + QR), found #{total}"

  kind = codes::get_barcode_type(1)
  assert kind == "EAN_13", "the main code should be EAN-13, it is #{kind}"

  spot = codes::get_barcode_location(1)
  assert spot.first() == 1, "barcode is not on the cover"
}
```

---

## 7.2 デコードと検証

| 関数 | 動作 |
|---|---|
| `codes::decode_barcode(n)` | n番目のコードの内容 |
| `codes::validate_barcode_checksum(n)` | n番目の GTIN チェックディジット |
| `codes::validate_gtin(text)` / `codes::validate_ean(text)` | 文字列の GTIN チェックディジット |
| `codes::validate_code128()` | Code 128 が正常にデコードできれば真 |

```pdfl
check "Code integrity" {
  code = codes::decode_barcode(1)
  print("code read:", code)

  // チェックディジットが誤った GTIN はレジで弾かれます
  assert codes::validate_barcode_checksum(1),
    "invalid check digit in code #{code}"
  assert code.starts_with("789"),
    "GTIN is not Brazilian (should start with 789): #{code}"

  // バーの下に印刷された数字の確認
  printed = text::extract_from_region(1, region(400, 20, 180, 15)).trim()
  assert codes::validate_ean(printed),
    "the printed number is not a valid GTIN: #{printed}"
}
```

---

## 7.3 相互確認

| 関数 | 動作 |
|---|---|
| `codes::compare_barcode_with_text()` | すべてのコードの内容が本文に現れれば真 |
| `codes::validate_barcode_format(regex)` | 全コードの内容が正規表現に一致すれば真 |
| `codes::validate_barcode_position(region)` または `(x0, y0, x1, y1)` | 全コードが領域内にあれば真 |

`compare_barcode_with_text` は業界で最も高くつくミスを捕まえます。バーコードが
ある製品を指しているのに、印刷された文字は別の製品を示している場合です。

```pdfl
check "Cross-checks" {
  assert codes::compare_barcode_with_text(),
    "the barcode number does not appear in the text — artwork with swapped data?"

  // EAN-13 のみ：ちょうど13桁
  assert codes::validate_barcode_format("^\d{13}$"),
    "there is a code outside the EAN-13 pattern"

  // 名前付き領域で指定すると読みやすくなります
  area = region(400, 20, 180, 80, "barcode area")
  assert codes::validate_barcode_position(area),
    "barcode outside the reserved area of the packaging"
}
```

---

## 7.4 完全な例

```pdfl
// package_insert.pdfl — 添付文書のロットコード検証
// 使い方: pdfl run package_insert.pdfl insert.pdf
profile "medicine-insert" {

  check "Codes present" tags: ["codes"] {
    assert codes::detect_barcodes(), "insert has no barcode"
    assert codes::count_barcodes() >= 1, "expected at least the product EAN"
  }

  check "Code integrity" tags: ["codes"] {
    code = codes::decode_barcode(1)
    kind = codes::get_barcode_type(1)
    print("code:", kind, "=", code)

    assert kind == "EAN_13", "main code is not EAN-13 (it is #{kind})"
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"
    assert code.starts_with("789"), "GTIN is not Brazilian: #{code}"
  }

  check "Cross-check with the text" tags: ["codes", "critical"] {
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Position in the artwork" tags: ["codes", "layout"] {
    reserved = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(reserved),
      "code outside the reserved area — risk of being trimmed off"
  }

  check "Cross-check with the product database" tags: ["data"] {
    // data:: と連携します — 第9章を参照
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} is not in the approved product database"
    print("product:", product.get(2))
  }
}
```

---

[← `prepress::`](06-prepress.md) · [目次](README.md) · [次：`fix::` →](08-fix.md)
