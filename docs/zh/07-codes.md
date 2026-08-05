# 7. `codes::` 命名空间 — 条码与二维码

[← `prepress::`](06-prepress.md) · [目录](README.md) · [下一章：`fix::` →](08-fix.md)

用于检测、解码和校验文档中条码与二维码的 13 个函数。

> 扫描会以高分辨率渲染页面，并在首次调用任一 `codes::` 函数时**只执行一次**。
> 不使用该命名空间的脚本不承担这一开销。

支持的格式包括 EAN-8/13、UPC-A/E、Code 128、Code 39、ITF、二维码、
Data Matrix、Aztec 和 PDF417。

---

## 7.1 检测

| 函数 | 功能 |
|---|---|
| `codes::detect_barcodes()` | 存在条码则为真 |
| `codes::detect_qrcodes()` | 存在二维码则为真 |
| `codes::count_barcodes()` | 检测到的码总数 |
| `codes::get_barcode_type(n)` | 第 n 个码的格式（`"EAN_13"`、`"QR_CODE"`……） |
| `codes::get_barcode_location(n)` | 位置 `[页码, x, y]`（点，原点在左下） |

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

## 7.2 解码与校验

| 函数 | 功能 |
|---|---|
| `codes::decode_barcode(n)` | 第 n 个码的内容 |
| `codes::validate_barcode_checksum(n)` | 第 n 个码的 GTIN 校验位 |
| `codes::validate_gtin(text)` / `codes::validate_ean(text)` | 字符串的 GTIN 校验位 |
| `codes::validate_code128()` | 存在成功解码的 Code 128 则为真 |

```pdfl
check "Code integrity" {
  code = codes::decode_barcode(1)
  print("code read:", code)

  // 校验位错误的 GTIN 会在收银台被拒
  assert codes::validate_barcode_checksum(1),
    "invalid check digit in code #{code}"
  assert code.starts_with("789"),
    "GTIN is not Brazilian (should start with 789): #{code}"

  // 核对条码下方印刷的数字
  printed = text::extract_from_region(1, region(400, 20, 180, 15)).trim()
  assert codes::validate_ean(printed),
    "the printed number is not a valid GTIN: #{printed}"
}
```

---

## 7.3 交叉核对

| 函数 | 功能 |
|---|---|
| `codes::compare_barcode_with_text()` | 所有码的内容都出现在正文中则为真 |
| `codes::validate_barcode_format(regex)` | 所有码的内容都匹配正则则为真 |
| `codes::validate_barcode_position(region)` 或 `(x0, y0, x1, y1)` | 所有码都在区域内则为真 |

`compare_barcode_with_text` 能抓住业内代价最高的错误：条码指向某个产品，
而印刷的文字却是另一个产品。

```pdfl
check "Cross-checks" {
  assert codes::compare_barcode_with_text(),
    "the barcode number does not appear in the text — artwork with swapped data?"

  // 只允许 EAN-13：正好 13 位数字
  assert codes::validate_barcode_format("^\d{13}$"),
    "there is a code outside the EAN-13 pattern"

  // 使用具名区域更易读
  area = region(400, 20, 180, 80, "barcode area")
  assert codes::validate_barcode_position(area),
    "barcode outside the reserved area of the packaging"
}
```

---

## 7.4 完整示例

```pdfl
// package_insert.pdfl — 药品说明书的批号校验
// 用法: pdfl run package_insert.pdfl insert.pdf
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
    // 与 data:: 配合使用 — 见第 9 章
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} is not in the approved product database"
    print("product:", product.get(2))
  }
}
```

---

[← `prepress::`](06-prepress.md) · [目录](README.md) · [下一章：`fix::` →](08-fix.md)
