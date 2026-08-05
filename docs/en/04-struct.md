# 4. `struct::` namespace — structure and metadata

[← `text::`](03-text.md) · [Index](README.md) · [Next: `visual::` →](05-visual.md)

23 functions about the file itself: metadata, internal objects, security and
traceability.

> Functions from `list_objects` onwards read the internal structure of the file.
> That analysis runs **exactly once**, on first use, and is cached.

---

## 4.1 Metadata

### Direct reads

| Function | Returns |
|---|---|
| `struct::get_title()` | Title |
| `struct::get_author()` | Author |
| `struct::get_subject()` | Subject |
| `struct::get_keywords()` | Keywords |
| `struct::get_creator()` | Program that authored the original document |
| `struct::get_producer()` | Program that produced the PDF |

All return an empty string when the field is missing.

```pdfl
check "Required metadata" {
  assert struct::get_title() != "", "PDF has no title"
  assert struct::get_author() != "", "PDF has no author"

  // Producer reveals the originating tool — useful for tracing problems
  print("produced by:", struct::get_producer())
  print("authored in:", struct::get_creator())
}

check "Trusted origin" {
  // Some workflows only accept PDFs from approved tools
  producer = struct::get_producer()
  assert producer.contains("Adobe") || producer.contains("Ghostscript"),
    "PDF produced by an unapproved tool: #{producer}"
}
```

### `struct::get_creation_date()` and `struct::get_modification_date()`

Dates already converted from the PDF internal format
(`D:20260802173622-03'00'`) to `YYYY-MM-DD HH:MM:SS`.

```pdfl
check "File dates" {
  created = struct::get_creation_date()
  assert created != "", "PDF has no creation date"
  print("created:", created)
  print("modified:", struct::get_modification_date())

  // String comparison works because the format sorts correctly
  assert created > "2026-01-01", "file is too old for this campaign"
}
```

### `struct::list_metadata_entries()`

List of all non-empty entries, formatted as `"Key: value"`.

```pdfl
check "Metadata inventory" {
  entries = struct::list_metadata_entries()
  print("metadata:", entries.join(" | "))
  require entries.length >= 2
}
```

### `struct::extract_xmp()`

XMP (XML) metadata from the catalog. Empty string when absent.

```pdfl
check "XMP present" {
  xmp = struct::extract_xmp()
  assert xmp != "", "PDF has no XMP metadata"

  // XMP is XML — you can look for specific fields
  assert xmp.contains("pdfaid"), "no PDF/A identification in the XMP"
  print("XMP has", xmp.length, "characters")
}
```

---

## 4.2 File and traceability

### `struct::file_size()`

Size in bytes.

```pdfl
check "Size for e-mail delivery" {
  mb = struct::file_size() / 1024 / 1024
  assert mb < 10, "file is #{round(mb)} MB (10 MB e-mail limit)"
}
```

### `struct::calculate_sha256()`

SHA-256 hash of the file — the fingerprint for an audit trail.

```pdfl
check "Audit record" {
  // The hash goes into the report and proves exactly which file was approved
  hash = struct::calculate_sha256()
  print("SHA-256:", hash)
  require hash.length == 64
}
```

### `struct::detect_file_bloat([kb_per_page])`

True when the file is bloated — above the KB-per-page limit (default 1024).

```pdfl
check "Lean file" {
  assert !struct::detect_file_bloat(1024),
    "heavy file: #{struct::file_size() / 1024} KB for #{doc.page_count} pages"

  // Stricter limit for web publishing
  assert !struct::detect_file_bloat(200),
    "too heavy for web publishing"
}
```

---

## 4.3 Internal objects

### `struct::count_objects()`

Number of content objects (text, images, strokes) across the pages.

```pdfl
check "Document is not empty" {
  require struct::count_objects() > 0
  print("content objects:", struct::count_objects())
}
```

### `struct::list_objects()`

Lists every object in the file as `"number: type"`.

```pdfl
check "File inventory" {
  objects = struct::list_objects()
  print("total objects:", objects.length)

  // How many are fonts?
  fonts = objects.filter { |o| o.contains("Font") }
  print("font objects:", fonts.length)
}
```

### `struct::detect_unreferenced_objects()`

Objects unreachable from the trailer — dead weight in the file.

> Infrastructure objects (`ObjStm`, `XRef`) are excluded: by definition they are
> never referenced from the trailer, and reporting them would be a false alarm.

```pdfl
check "No junk in the file" {
  loose = struct::detect_unreferenced_objects()
  assert loose.length == 0,
    "#{loose.length} unreferenced object(s): #{loose.join(", ")}"
}
```

### `struct::detect_orphaned_resources()`

Same as above but limited to resources (fonts, images, XObjects) — the kind of
leftover that weighs the most.

```pdfl
check "Orphaned resources" {
  orphans = struct::detect_orphaned_resources()
  assert orphans.length == 0,
    "unused embedded resources: #{orphans.join(", ")} — run 'pdfl fix' with remove_unused_resources()"
}
```

### `struct::measure_object_size(number)`

Approximate size of a specific object, in bytes.

```pdfl
check "Heaviest object" {
  // Combine with list_objects to investigate what takes up space
  struct::list_objects().each { |entry| print(entry) }
  print("size of object 5:", struct::measure_object_size(5), "bytes")
}
```

---

## 4.4 Security

### `struct::detect_javascript()`

True when the PDF has embedded JavaScript.

```pdfl
check "No executable code" {
  // JavaScript in a PDF is a common attack vector and unnecessary
  // in production print documents
  assert !struct::detect_javascript(),
    "PDF contains embedded JavaScript"
}
```

### `struct::detect_suspicious_actions()`

Lists risky actions found: `JavaScript`, `Launch` (runs a program), `URI`,
`SubmitForm`, `ImportData`, `GoToR` — each with its originating object.

```pdfl
check "Document actions" {
  actions = struct::detect_suspicious_actions()
  assert actions.length == 0,
    "suspicious actions in the PDF: #{actions.join("; ")}"
}

check "Only links are acceptable" {
  // If external links are allowed, filter down to what actually worries you
  dangerous = struct::detect_suspicious_actions().filter { |a|
    a.contains("Launch") || a.contains("JavaScript")
  }
  assert dangerous.length == 0, "dangerous actions: #{dangerous.join("; ")}"
}
```

### `struct::check_encryption()`

True when the document is encrypted.

```pdfl
check "File open for production" {
  // An encrypted PDF can fail at the print shop's RIP
  assert !struct::check_encryption(),
    "PDF is encrypted — remove protection before sending it to print"
}
```

### `struct::validate_permissions()`

True when there are **no** permission restrictions (document free to process).

```pdfl
check "Permissions" {
  assert struct::validate_permissions(),
    "PDF has permission restrictions that may block processing"
}
```

### `struct::validate_signatures()`

True when the document has digital signature fields.

> This function detects the **presence** of those fields. Cryptographic
> validation of the certificate chain is not performed in this version.

```pdfl
check "Signed document" {
  assert struct::validate_signatures(),
    "document has no digital signature field"
}
```

---

## 4.5 Complete example

```pdfl
// audit.pdfl — compliance and security verification
profile "file-audit" {

  check "Identification" tags: ["metadata"] {
    assert struct::get_title() != "", "no title"
    assert struct::get_author() != "", "no author"
    assert struct::get_creation_date() != "", "no creation date"
    print("document:", struct::get_title())
    print("produced by:", struct::get_producer())
  }

  check "Traceability" tags: ["audit"] {
    // The hash proves exactly which file was validated
    print("SHA-256:", struct::calculate_sha256())
    print("size:", struct::file_size() / 1024, "KB")
  }

  check "Security" tags: ["security"] {
    assert !struct::detect_javascript(), "embedded JavaScript"
    assert !struct::check_encryption(), "encrypted file"
    actions = struct::detect_suspicious_actions()
    assert actions.length == 0, "suspicious actions: #{actions.join("; ")}"
  }

  check "File hygiene" tags: ["optimization"] {
    orphans = struct::detect_orphaned_resources()
    assert orphans.length == 0, "unused resources: #{orphans.join(", ")}"
    assert !struct::detect_file_bloat(1024), "bloated file"
  }
}
```

---

[← `text::`](03-text.md) · [Index](README.md) · [Next: `visual::` →](05-visual.md)
