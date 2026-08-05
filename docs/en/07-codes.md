# 7. `codes::` namespace — barcodes and QR codes

[← `prepress::`](06-prepress.md) · [Index](README.md) · [Next: `fix::` →](08-fix.md)

13 functions to detect, decode and validate barcodes and QR codes printed in the
document.

> Scanning renders the pages at high resolution and runs **exactly once**, on the
> first use of any `codes::` function. Scripts that never touch this namespace do
> not pay that cost.

Recognized formats include EAN-8/13, UPC-A/E, Code 128, Code 39, ITF, QR Code,
Data Matrix, Aztec and PDF417.

---

## 7.1 Detection

### `codes::detect_barcodes()` and `codes::detect_qrcodes()`

```pdfl
check "Packaging carries a code" {
  assert codes::detect_barcodes(),
    "no barcode found in the artwork"

  // Traceability QR
  assert codes::detect_qrcodes(),
    "the traceability QR code is missing"
}
```

### `codes::count_barcodes()`

Total number of codes detected (barcodes + QR).

```pdfl
check "Number of codes" {
  total = codes::count_barcodes()
  print("codes detected:", total)

  assert total == 2,
    "expected 2 codes (EAN + QR), found #{total}"
}
```

### `codes::get_barcode_type(n)`

Format of the nth code (1-based): `"EAN_13"`, `"QR_CODE"`, `"CODE_128"`...

```pdfl
check "Main code type" {
  kind = codes::get_barcode_type(1)
  assert kind == "EAN_13",
    "the main code should be EAN-13, it is #{kind}"
}

check "Listing them all" {
  // Walk by index, from 1 up to the count
  print("first:", codes::get_barcode_type(1))
  print("second:", codes::get_barcode_type(2))
}
```

### `codes::get_barcode_location(n)`

Where the code sits: `[page, x, y]` in points (origin at the bottom-left corner).

```pdfl
check "Code position" {
  spot = codes::get_barcode_location(1)
  print("page:", spot.get(1), "x:", spot.get(2), "y:", spot.get(3))

  // The code must be on the first page
  assert spot.first() == 1,
    "barcode is not on the cover"
}
```

---

## 7.2 Decoding and validation

### `codes::decode_barcode(n)`

The decoded content of the nth code.

```pdfl
check "Code content" {
  code = codes::decode_barcode(1)
  print("code read:", code)
  assert code.starts_with("789"),
    "GTIN is not Brazilian (should start with 789): #{code}"
}
```

### `codes::validate_barcode_checksum(n)`

Validates the GTIN check digit of the nth detected code.

```pdfl
check "Check digit" {
  // A GTIN with a wrong check digit is rejected at the supermarket till
  assert codes::validate_barcode_checksum(1),
    "invalid check digit in code #{codes::decode_barcode(1)}"
}
```

### `codes::validate_gtin(text)` and `codes::validate_ean(text)`

Synonyms. They validate the check digit of a **string** (EAN-8/13, UPC-A,
GTIN-14) — useful for a number coming from another source.

```pdfl
check "GTIN stated in the text" {
  require codes::validate_gtin("7891234567895")
  require !codes::validate_gtin("7891234567890")   // wrong check digit

  // Checking the number printed under the bars
  printed = text::extract_from_region(1, region(400, 20, 180, 15)).trim()
  assert codes::validate_ean(printed),
    "the printed number is not a valid GTIN: #{printed}"
}
```

### `codes::validate_code128()`

True when at least one Code 128 decoded successfully (its checksum is validated
during decoding).

```pdfl
check "Logistics code" {
  assert codes::validate_code128(),
    "the Code 128 logistics label is missing"
}
```

---

## 7.3 Cross-checking

### `codes::compare_barcode_with_text()`

True when the content of **every** code appears in the document text.

This is the test that catches the industry's most expensive mistake: the barcode
pointing to one product while the printed text says another.

```pdfl
check "Code matches the printed text" {
  assert codes::compare_barcode_with_text(),
    "the barcode number does not appear in the text — artwork with swapped data?"
}
```

### `codes::validate_barcode_format(regex)`

True when the content of every code matches the regular expression.

```pdfl
check "Expected format" {
  // EAN-13 only: exactly 13 digits
  assert codes::validate_barcode_format("^\d{13}$"),
    "there is a code outside the EAN-13 pattern"
}

check "QR points to the official site" {
  assert codes::validate_barcode_format("^https://company\.com/.*"),
    "QR code points to an unauthorized address"
}
```

### `codes::validate_barcode_position(region)` or `(x0, y0, x1, y1)`

True when every code sits inside the area. Accepts a `region` or four numbers in
points.

```pdfl
check "Code in the reserved area" {
  // With a named region — more readable
  area = region(400, 20, 180, 80, "barcode area")
  assert codes::validate_barcode_position(area),
    "barcode outside the reserved area of the packaging"
}

check "With raw coordinates" {
  // x0, y0, x1, y1 in points
  assert codes::validate_barcode_position(400, 20, 580, 100),
    "code outside the specified position"
}
```

---

## 7.4 Complete example

```pdfl
// package_insert.pdfl — batch code validation on a medicine insert
// Usage: pdfl run package_insert.pdfl insert.pdf
profile "medicine-insert" {

  check "Codes present" tags: ["codes"] {
    assert codes::detect_barcodes(), "insert has no barcode"
    assert codes::count_barcodes() >= 1,
      "expected at least the product EAN"
  }

  check "Code integrity" tags: ["codes"] {
    code = codes::decode_barcode(1)
    kind = codes::get_barcode_type(1)
    print("code:", kind, "=", code)

    assert kind == "EAN_13", "main code is not EAN-13 (it is #{kind})"
    assert codes::validate_barcode_checksum(1),
      "invalid check digit: #{code}"
    assert code.starts_with("789"),
      "GTIN is not Brazilian: #{code}"
  }

  check "Cross-check with the text" tags: ["codes", "critical"] {
    // The most expensive mistake: one product's code, another's text
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Position in the artwork" tags: ["codes", "layout"] {
    reserved = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(reserved),
      "code outside the reserved area — risk of being trimmed off"
  }

  check "Cross-check with the product database" tags: ["data"] {
    // Integrates with data:: — see chapter 9
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product,
      "GTIN #{code} is not in the approved product database"
    print("product:", product.get(2))
  }
}
```

---

[← `prepress::`](06-prepress.md) · [Index](README.md) · [Next: `fix::` →](08-fix.md)
