# 10. Standard library

[← `data::`](09-data.md) · [Index](README.md) · [Next: CLI commands →](11-cli.md)

List and string methods, plus the global functions available anywhere in a
script.

---

## 10.1 List methods

### Iterating

#### `list.each { |item| ... }`

Runs the block for every item.

```pdfl
check "each" {
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }
}
```

#### `list.each_with_index { |item, i| ... }`

Like `each`, but the second parameter receives the position (starting at **0**).

```pdfl
check "each_with_index" {
  doc.fonts.each_with_index { |font, i|
    print("font", i + 1, "of", doc.fonts.length, ":", font.name)
  }
}
```

### Testing

#### `list.all { |item| ... }`

True when **every** item satisfies the condition. An empty list returns true.

```pdfl
check "all" {
  require doc.fonts.all { |f| f.is_embedded }
  require doc.pages.all { |p| p.has_trim_box }
}
```

#### `list.any { |item| ... }`

True when **at least one** item satisfies it. An empty list returns false.

```pdfl
check "any" {
  assert doc.pages.any { |p| p.extract_text() != "" },
    "the entire document has no text"
}
```

#### `list.contains(value)`

True when the value is in the list.

```pdfl
check "contains" {
  require [1, 2, 3].contains(2)
  require prepress::detect_spot_colors().contains("Varnish")
}
```

### Transforming

#### `list.filter { |item| ... }`

A new list holding only the items that satisfy the condition.

```pdfl
check "filter" {
  bad = doc.images.filter { |img| img.dpi < 300 }
  assert bad.length == 0,
    "#{bad.length} image(s) with low resolution"

  // Chaining: filter and then transform
  names = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  print("loose fonts:", names.join(", "))
}
```

#### `list.map { |item| ... }`

A new list holding the result of the block for each item.

```pdfl
check "map" {
  numbers = doc.pages.map { |p| p.number }
  widths = doc.pages.map { |p| p.width }
  print("pages:", numbers.join(", "))
}
```

### Accessing

#### `list.length`

Item count. Works as a property or a method: `list.length` and `list.length()`
are equivalent.

```pdfl
check "length" {
  require doc.pages.length == doc.page_count
  print("fonts:", doc.fonts.length)
}
```

#### `list.get(n)`

The nth item, **1-based**. Friendly error when the index does not exist.

```pdfl
check "get" {
  row = data::load_dataset("data/batches.csv").get(2)   // second row
  print("first column:", row.get(1))
}
```

#### `list.first()` and `list.last()`

First and last item. Both return `null` on an empty list (no error).

```pdfl
check "first and last" {
  first = doc.pages.first()
  last = doc.pages.last()
  print("from page", first.number, "to", last.number)

  // Safe on an empty list: null is falsy
  spots = prepress::detect_spot_colors()
  assert !spots.first() || spots.first() == "Varnish",
    "unexpected special ink: #{spots.first()}"
}
```

#### `list.join([separator])`

Joins items into text. Default separator: `", "`.

```pdfl
check "join" {
  print(doc.fonts.map { |f| f.name }.join(", "))
  print(prepress::get_page_boxes(1).join(" | "))
  print([1, 2, 3].join(" -> "))
}
```

---

## 10.2 String methods

| Method | What it does |
|---|---|
| `text.contains(sub)` | Does it contain the fragment? |
| `text.starts_with(sub)` | Does it start with it? |
| `text.ends_with(sub)` | Does it end with it? |
| `text.trim()` | Strips whitespace from both ends |
| `text.to_uppercase()` | All uppercase |
| `text.to_lowercase()` | All lowercase |
| `text.length` | Character count |

```pdfl
check "String methods" {
  title = doc.title

  require title.length > 0
  require title.trim() == title          // no stray whitespace
  assert !title.to_lowercase().contains("draft"),
    "title still marked as draft"

  code = codes::decode_barcode(1)
  assert code.starts_with("789"), "GTIN is not Brazilian"

  file = doc.filename
  assert file.ends_with(".pdf"), "unexpected extension"
}
```

The difference between `contains` on a string and on a list:

```pdfl
check "contains on each type" {
  // string: looks for a FRAGMENT inside the text
  require "final document".contains("final")

  // list: looks for a whole ITEM
  require ["a", "b"].contains("a")
  require !["ab"].contains("a")      // "a" is not an item of the list
}
```

---

## 10.3 Global functions

### `min(a, b)` and `max(a, b)`

```pdfl
check "min and max" {
  widths = doc.pages.map { |p| p.width }
  // Reducing a list with each
  smallest = 99999
  doc.pages.each { |p| smallest = min(smallest, p.width) }
  print("narrowest page:", smallest, "pt")
}
```

### `abs(x)`

Absolute value — essential for comparing dimensions with a tolerance.

```pdfl
check "abs" {
  const A4_WIDTH = 595.0
  const TOLERANCE = 5.0

  doc.pages.each { |page|
    // "the difference, either way, is smaller than the tolerance"
    assert abs(page.width - A4_WIDTH) < TOLERANCE,
      "page #{page.number} is outside A4: #{page.width}pt"
  }
}
```

### `round(x)`

Rounds to the nearest integer. Useful for keeping messages readable.

```pdfl
check "round" {
  doc.images.each { |img|
    // without round: "217.4453125 DPI" — with round: "217 DPI"
    assert img.dpi >= 300,
      "image on page #{img.page_number}: #{round(img.dpi)} DPI"
  }

  mb = struct::file_size() / 1024 / 1024
  print("size:", round(mb), "MB")
}
```

### `print(...)`

Prints values separated by spaces. It goes to **stderr**, so it never pollutes
the report on stdout — you can use `> report.json` without mixing the two.

```pdfl
check "print" {
  print("document:", doc.filename)
  print("pages:", doc.page_count, "| fonts:", doc.fonts.length)

  // Handy for exploring before writing the final validation
  doc.images.each { |img|
    print("image", img.width, "x", img.height, "@", round(img.dpi), "DPI")
  }
}
```

### `region(x, y, width, height [, name])`

Creates a region. Documented in [chapter 2](02-types.md#25-region--an-area-of-the-page).

---

## 10.4 Useful patterns

### Counting how many items fail

```pdfl
check "Problem count" {
  bad = doc.images.filter { |i| i.dpi < 300 }
  assert bad.length == 0,
    "#{bad.length} of #{doc.images.length} images below 300 DPI"
}
```

### Listing the failures in the message

```pdfl
check "List in the message" {
  // Chains stay on one line: the dot must follow the previous value
  // with no line break in between.
  problems = doc.pages.filter { |p| !p.has_trim_box }.map { |p| p.number }

  assert problems.length == 0,
    "pages without a TrimBox: #{problems.join(", ")}"
}
```

### Validating with a tolerance

```pdfl
function close_to(value, target, tolerance) {
  abs(value - target) < tolerance
}

check "With tolerance" {
  doc.pages.each { |page|
    assert close_to(page.width, 595.0, 2.0),
      "page #{page.number}: width #{page.width}pt (expected 595 ± 2)"
  }
}
```

### Avoiding errors on an empty document

```pdfl
check "Defensive" {
  // Short-circuiting avoids calling first() on an empty list
  assert doc.pages.length == 0 || doc.pages.first().width > 0,
    "the first page has no width"
}
```

---

[← `data::`](09-data.md) · [Index](README.md) · [Next: CLI commands →](11-cli.md)
