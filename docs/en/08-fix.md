# 8. `fix::` namespace — normalization

[← `codes::`](07-codes.md) · [Index](README.md) · [Next: `data::` →](09-data.md)

19 operations that **modify** the PDF and save a new file. The original is never
touched.

---

## 8.1 How to use it

`fix::` is the only namespace that writes, so it runs under its own command:

```bash
pdfl fix input.pdf script.pdfl --output fixed.pdf
```

Options:

| Option | What it does |
|---|---|
| `--output <file>` | Output PDF (required) |
| `--dry-run` | Lists the operations without saving anything |
| `--report json\|csv\|html\|pdf` | Report format |
| `--report-file <file>` | Writes the report to a file |

Under `pdfl run`, any `fix::` call raises an error pointing to the right command —
so nobody applies corrections while believing they are only validating.

### How operations work

```pdfl
// This script needs no checks: these are commands, executed in order.
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::add_page_numbers()
fix::add_watermark("DRAFT")
```

Every call is **validated at call time** (nonexistent page, invalid rotation,
missing file) and only then applied. The report carries a `fixes` field with what
was done:

```json
"fixes": [
  "TrimBox set to [8.5, 8.5, 586.5, 833.5]",
  "page numbering added",
  "watermark \"DRAFT\" added"
]
```

Nothing stops you from mixing validation and correction in the same script:

```pdfl
// Validate before fixing — a failed precondition shows up in the report
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(), "file is encrypted, cannot fix it"
}

fix::add_page_numbers()
```

---

## 8.2 Page boxes

### `fix::set_page_size(width, height)`

Sets the MediaBox on every page.

```pdfl
// A4 in points — or use units and let the language convert
fix::set_page_size(595, 842)
fix::set_page_size(210mm, 297mm)    // identical, and more readable
```

### `fix::set_crop_box(x0, y0, x1, y1)`, `set_trim_box`, `set_bleed_box`

Set the matching box on every page. Coordinates in points, from the bottom-left
to the top-right corner.

```pdfl
// The file arrived from the publisher without production boxes:
// TrimBox = final area; BleedBox = with 3 mm of bleed around it
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)
```

---

## 8.3 Pages

### `fix::rotate_page([page,] degrees)`

Rotates by 90, 180 or 270 degrees. Without a page number, rotates all of them.

```pdfl
fix::rotate_page(90)        // every page
fix::rotate_page(3, 180)    // page 3 only
```

### `fix::delete_page(n)` and `fix::duplicate_page(n)`

```pdfl
fix::delete_page(1)         // drop the draft cover
fix::duplicate_page(1)      // duplicate the cover (the copy goes right after)
```

Deleting the only page in a document is refused with a clear message.

### `fix::reorder_pages([new, order])`

New page order. The list must use every page exactly once.

```pdfl
// A 4-page document with the cover at the end: bring it to the front
fix::reorder_pages([4, 1, 2, 3])
```

### `fix::split_document(from, to, "output.pdf")`

Saves a page range to another file. The document being edited stays intact.

```pdfl
// Separate cover from body for different suppliers
fix::split_document(1, 2, "cover.pdf")
fix::split_document(3, 50, "body.pdf")
```

### `fix::merge_documents("other.pdf")`

Appends the pages of another PDF at the end.

```pdfl
fix::merge_documents("attachments/warranty.pdf")
fix::merge_documents("attachments/size_chart.pdf")
```

---

## 8.4 Content

### `fix::add_watermark("text")`

Diagonal grey watermark on every page.

```pdfl
fix::add_watermark("DRAFT — DO NOT PRINT")
```

### `fix::add_stamps("text")`

Red stamp in the top-right corner of each page.

```pdfl
fix::add_stamps("APPROVED 2026-08-02")
```

### `fix::add_page_numbers()`

`n / total` numbering in the footer of every page.

```pdfl
fix::add_page_numbers()
```

### `fix::remove_annotations()` and `fix::remove_attachments()`

Remove annotations (comments, review markup) and attached files.

```pdfl
// Before sending to the print shop: review comments must not show up,
// and attachments only inflate the file
fix::remove_annotations()
fix::remove_attachments()
```

### `fix::flatten_layers()`

Removes the optional content (OCG) structure, leaving all content permanently
visible.

```pdfl
// Layers with "English version" turned off can be re-enabled by mistake
// at the shop — flattening removes the risk
fix::flatten_layers()
```

---

## 8.5 Optimization

> The operations in this section **only write when the file gets smaller**. If
> rewriting produces a larger file, the original is kept.

### `fix::remove_unused_resources()`

Discards objects unreachable from the trailer.

```pdfl
fix::remove_unused_resources()
```

### `fix::downsample_images([dpi])`

Resamples images above the target DPI (default 300). DPI is computed from the
**actual printed size** of the image on the page.

```pdfl
// An e-mail approval copy does not need 300 DPI
fix::downsample_images(96)

// A digital-print version
fix::downsample_images(200)
```

> **CMYK images are preserved.** Resampling them would require converting to RGB,
> which would destroy the prepress separations. In print-shop files, the savings
> come from the RGB images.

### `fix::compress_images([quality])`

Re-encodes images as JPEG at the given quality (1 to 100, default 85).

```pdfl
fix::compress_images(70)
```

### Not available

`subset_fonts` and `linearize_document` do **not** exist as `fix::` operations and
raise an unknown-function error:

- **subset_fonts**: it was implemented and measured. Professional producers
  already embed only the glyphs in use, so the measured gain was 0.5% at best and
  nil elsewhere — not worth the risk of corrupting fonts. To *check* whether
  fonts are subset, use
  [`prepress::subset_fonts()`](06-prepress.md#prepresssubset_fonts).
- **linearize_document**: it requires generating hint tables (§7.14 of the PDF
  specification). No Rust library does this, and a partial implementation would
  not be recognized as "Fast Web View" by readers.

---

## 8.6 Complete examples

### Preparing a publisher's file for the print shop

```pdfl
// prepare_for_print.pdfl
// Usage: pdfl fix publisher.pdf prepare_for_print.pdfl --output print.pdf

check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask the publisher for the open version"
}

// Production boxes the publisher never defined
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Cleanup: review comments and attachments do not go to print
fix::remove_annotations()
fix::remove_attachments()
fix::flatten_layers()
fix::remove_unused_resources()
```

### Lightweight version for e-mail approval

```pdfl
// email_version.pdfl
// Usage: pdfl fix final.pdf email_version.pdfl --output approval.pdf

fix::downsample_images(96)
fix::compress_images(70)
fix::add_watermark("PROOF — NOT THE FINAL VERSION")
fix::add_page_numbers()
```

Checking the result with `pdfl` itself:

```bash
pdfl fix final.pdf email_version.pdfl --output approval.pdf
pdfl inspect approval.pdf          # size, DPI and warnings for the new file
```

### Splitting a book into cover and body

```pdfl
// split.pdfl
// Usage: pdfl fix book.pdf split.pdfl --output book_processed.pdf

check "Expected structure" {
  assert doc.page_count > 4,
    "book has only #{doc.page_count} pages — unexpected structure"
}

fix::split_document(1, 2, "output/cover.pdf")
fix::split_document(3, doc.page_count, "output/body.pdf")
```

---

[← `codes::`](07-codes.md) · [Index](README.md) · [Next: `data::` →](09-data.md)
