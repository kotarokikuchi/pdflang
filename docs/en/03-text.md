# 3. `text::` namespace — text

[← Types](02-types.md) · [Index](README.md) · [Next: `struct::` →](04-struct.md)

25 functions to extract, normalize, search and validate the text of a document.

> In functions marked with `[text]`, the argument is **optional**: without it,
> the function works on the whole document; with it, on the string you pass.

---

## 3.1 Extraction

### `text::extract_all()`

All text in the document (pages joined by newlines).

```pdfl
check "Document has content" {
  content = text::extract_all()
  assert content.trim() != "", "PDF has no extractable text"
  print("total characters:", content.length)
}
```

### `text::extract_from_page(page)`

Text from one page (1-based). Friendly error if the page does not exist.

```pdfl
check "Cover and back cover" {
  cover = text::extract_from_page(1)
  assert cover.contains("User Manual"), "cover lacks the expected title"

  last = text::extract_from_page(doc.page_count)
  assert last.contains("ISBN"), "last page has no ISBN"
}
```

### `text::extract_from_region(page, region)`

Text inside a specific area. Returns an empty string when the region has no text
(that is not an error).

```pdfl
check "Production footers must not survive" {
  // Production footers (InDesign file name, export date) sometimes
  // leak into the final file
  footer = region(0, 0, 467, 40, "footer")

  doc.pages.each { |page|
    content = text::extract_from_region(page.number, footer)
    assert !content.contains(".indd"),
      "page #{page.number} has a production mark in the footer: #{content.trim()}"
  }
}
```

### `text::extract_with_normalization()`

The document text already normalized (lowercase, collapsed whitespace).
Shorthand for `text::normalize(text::extract_all())`.

```pdfl
check "Search without worrying about case" {
  content = text::extract_with_normalization()
  require content.contains("general conditions")   // matches "GENERAL  CONDITIONS"
}
```

---

## 3.2 Normalization and splitting

### `text::normalize([text])`

Lowercase and collapsed whitespace (runs of spaces become one).

```pdfl
check "Normalization" {
  require text::normalize("  HELLO   World  ") == "hello world"

  // With no argument it normalizes the whole document
  print("normalized document has", text::normalize().length, "characters")
}
```

### `text::split_words([text])`

Splits into words, stripping punctuation from the edges.

```pdfl
check "Words" {
  words = text::split_words("Hello, world! (test)")
  require words.length == 3
  require words.first() == "Hello"
  require words.contains("test")
}
```

### `text::split_sentences([text])`

Splits into sentences (separated by `.`, `!` or `?` followed by a space).

```pdfl
check "Sentences that run too long" {
  // Package inserts and contracts have a practical readability limit
  text::split_sentences().each { |sentence|
    assert sentence.length < 400,
      "sentence with #{sentence.length} characters — hard to read"
  }
}
```

### `text::split_paragraphs([text])`

Splits into paragraphs (separated by a blank line).

```pdfl
check "Document structure" {
  paragraphs = text::split_paragraphs()
  print("paragraphs:", paragraphs.length)
  require paragraphs.length >= 3
}
```

### `text::count_words([text])` and `text::count_characters([text])`

```pdfl
check "Text volume" {
  require text::count_words() > 100
  require text::count_characters() > 500

  // They also work on any string
  summary = text::extract_from_page(1)
  assert text::count_words(summary) <= 250,
    "summary has #{text::count_words(summary)} words (max 250)"
}
```

### `text::detect_language([text])`

Returns `"pt"`, `"en"`, `"es"` or `"unknown"` (heuristic based on common words).

```pdfl
check "Document language" {
  language = text::detect_language()
  assert language == "en",
    "document should be in English, detected: #{language}"
}
```

---

## 3.3 Search and required content

### `text::require_text(term)` and `text::forbid_text(term)`

Return true/false. Comparison ignores case and spacing.

```pdfl
profile "contract" {
  check "Mandatory clauses" {
    assert text::require_text("governing law"),
      "contract has no governing-law clause"
    assert text::require_text("term of agreement"),
      "contract has no term clause"
  }

  check "Forbidden terms" {
    assert text::forbid_text("DRAFT"),
      "document still marked as draft"
    assert text::forbid_text("lorem ipsum"),
      "placeholder text was not replaced"
  }
}
```

### `text::require_match(regex)` and `text::forbid_match(regex)`

Same as above, but with a regular expression.

```pdfl
check "Patterns in the document" {
  // Must carry a contract number like 2026/0001
  assert text::require_match("\d{4}/\d{4}"),
    "contract number not found"

  // Must not carry US-style dates
  assert text::forbid_match("\d{2}-\d{2}-\d{4}"),
    "US-format date found"
}
```

### `text::fuzzy_match(a, b)`

Similarity between two strings, from `0.0` (unrelated) to `1.0` (identical).
Useful when typos or OCR noise are expected.

```pdfl
check "Product name with tolerance" {
  expected = "Paracetamol 750mg"
  found = text::extract_from_region(1, region(50, 700, 300, 40))

  similarity = text::fuzzy_match(expected, found)
  assert similarity > 0.9,
    "product name differs from expected (#{round(similarity * 100)}% similar)"
}
```

---

## 3.4 Personal data (privacy)

### `text::detect_personal_data([text])` and `text::detect_pii([text])`

Synonyms. They return the **list** of personal data found: CPF, CNPJ (Brazilian
tax IDs), e-mail and phone number.

> CPF and CNPJ only make the list when the **check digit is valid**. A number
> that merely looks like a CPF (e.g. `111.111.111-12`) raises no alarm.

```pdfl
check "Public document must carry no personal data" {
  found = text::detect_personal_data()
  assert found.length == 0,
    "personal data exposed: #{found.join("; ")}"
}

check "Report what was found" {
  // Each entry looks like "CPF: 529.982.247-25"
  text::detect_pii().each { |item|
    print("found:", item)
  }
}
```

---

## 3.5 Brazilian validations

### `text::validate_cpf(text)` and `text::validate_cnpj(text)`

Validate the check digit (mod 11) of Brazilian tax IDs. They accept punctuated or
plain input and reject repeated sequences (`111.111.111-11`).

```pdfl
check "Account holder's CPF" {
  cpf = text::extract_from_region(1, region(100, 600, 200, 20)).trim()
  assert text::validate_cpf(cpf),
    "invalid CPF in the record: #{cpf}"
}

check "Company CNPJ" {
  require text::validate_cnpj("11.222.333/0001-81")
  require !text::validate_cnpj("11.222.333/0001-82")   // wrong check digit
}
```

### `text::validate_date_format(text [, format])`

Checks whether the string is a **valid calendar date** (leap years and days per
month included). Accepted formats: `"dd/mm/aaaa"` and `"aaaa-mm-dd"`; without the
second argument, both are accepted.

```pdfl
check "Dates in the document" {
  require text::validate_date_format("29/02/2024")     // 2024 is a leap year
  require !text::validate_date_format("29/02/2023")    // 2023 is not
  require !text::validate_date_format("31/04/2026")    // April has 30 days

  // Requiring one specific format
  require text::validate_date_format("02/08/2026", "dd/mm/aaaa")
  require !text::validate_date_format("2026-08-02", "dd/mm/aaaa")
}
```

### `text::validate_phone_format(text)`

Brazilian phone numbers: `(DD) 9XXXX-XXXX` or `(DD) XXXX-XXXX`, punctuation
optional.

```pdfl
check "Contact phone" {
  require text::validate_phone_format("(11) 98765-4321")
  require text::validate_phone_format("1198765432")
  require !text::validate_phone_format("12345")
}
```

### `text::validate_format(text, regex)`

True when the **entire** string matches the regular expression.

```pdfl
check "Batch code in the factory pattern" {
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  assert text::validate_format(batch, "L\d{4}-\d{2}"),
    "batch code does not follow the L0000-00 pattern: #{batch}"
}
```

---

## 3.6 Comparison and diagnostics

### `text::diff(a, b)`

Lists lines that changed between two strings: `-` for lines removed, `+` for
lines added.

```pdfl
check "Comparing two pages" {
  before = text::extract_from_page(1)
  after = text::extract_from_page(2)

  changes = text::diff(before, after)
  print("changed lines:", changes.length)
  changes.each { |line| print(line) }
}
```

> To compare two **files**, use the `pdfl compare` command — it aligns pages
> automatically. See [chapter 11](11-cli.md).

### `text::detect_rasterized_text()`

True when some page has no extractable text but does have an image covering half
the area or more — a sign of text turned into an image.

```pdfl
check "Text must be text" {
  // A scanned or outlined page cannot be searched, made accessible
  // or spell-checked
  assert !text::detect_rasterized_text(),
    "there are pages with rasterized text (scanned or outlined)"
}
```

---

## 3.7 Complete example

```pdfl
// legal_document.pdfl — contract validation
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

[← Types](02-types.md) · [Index](README.md) · [Next: `struct::` →](04-struct.md)
