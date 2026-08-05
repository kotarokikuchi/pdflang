# 12. Recipes

[← CLI commands](11-cli.md) · [Index](README.md)

Complete cases, ready to adapt. Each one solves a real production problem.

---

## 12.1 Print shop: offset magazine preflight

**Problem:** the file arrives from the client and someone must check ink, fonts,
images and bleed before it goes to plate. A mistake found afterwards costs the
whole print run.

`profiles/offset.pdfl`:

```pdfl
profile "offset-magazine" {

  const TAC_LIMIT = 300%       // ink limit for coated stock
  const BLEED = 3mm            // imposition requirement
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress"] {
    // Exact TAC reads the colors declared in the file — the render-based
    // estimate underrates rich black and lets excess through
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMIT,
        "page #{page.number}: #{tac}% ink (limit #{TAC_LIMIT}%)"
    }
  }

  check "Colors" tags: ["prepress"] {
    assert prepress::detect_color_mode() != "RGB",
      "document is in RGB — convert to CMYK"

    spots = prepress::detect_spot_colors()
    assert spots.length == 0,
      "unquoted special ink: #{spots.join(", ")}"

    assert !prepress::detect_rich_black(),
      "rich black detected — use 0/0/0/100 for text"
  }

  check "Fonts" tags: ["fonts"] {
    loose = prepress::detect_text_substitution()
    assert loose.length == 0,
      "fonts not embedded (text will change at the RIP): #{loose.join(", ")}"

    assert prepress::validate_font_size(6),
      "there is text below 6 pt — illegible once printed"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25),
      "strokes below 0.25 pt disappear in print"
    assert !prepress::detect_hairlines_exact(),
      "there is a stroke with 0 width — set a real thickness"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI (minimum #{MIN_DPI})"
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number}"
    }
  }

  check "Geometry" tags: ["prepress"] {
    assert prepress::validate_trim_box(),
      "no TrimBox — imposition cannot know where to trim"
    assert prepress::validate_bleed_box(),
      "no BleedBox — no bleed is defined"
    assert prepress::check_page_geometry(BLEED),
      "bleed smaller than 3 mm on some page"
  }
}
```

**At the counter:**

```bash
# HTML report to hand back to the client
pdfl run profiles/offset.pdfl client.pdf --output html --output-file report.html
```

**As a watch folder:** the operator drops the file in and the report appears next
to it.

```bash
pdfl watch inbox/ --script profiles/offset.pdfl \
  --output-dir reports/ --report html
```

---

## 12.2 Legal publisher: contract before publishing

**Problem:** contracts and policies must carry mandatory clauses, must not
contain draft text or expose personal data, and the text must be searchable.

`profiles/legal.pdfl`:

```pdfl
profile "standard-contract" {

  check "Mandatory clauses" tags: ["legal"] {
    // Glossary maintained by the legal department
    missing = data::validate_against_reference("terms/clauses.txt")
    assert missing.length == 0,
      "missing clauses: #{missing.join("; ")}"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "document marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("X{3,}"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    // Tax IDs only make the list when the check digit is valid,
    // so a sample number raises no false alarm
    found = text::detect_personal_data()
    assert found.length == 0,
      "personal data in the document: #{found.join("; ")}"
  }

  check "Numbering and initials" tags: ["legal"] {
    doc.pages.each { |page|
      footer = region(0, 0, page.width, 60, "footer")
      content = text::extract_from_region(page.number, footer).trim()
      assert content != "",
        "page #{page.number} has no numbering/initials in the footer"
    }
  }

  check "Searchable text" tags: ["accessibility"] {
    assert !text::detect_rasterized_text(),
      "there are scanned pages — text cannot be searched or read by screen readers"
    assert text::detect_language() == "en",
      "document is not in English"
  }
}
```

**Usage:**

```bash
pdfl run profiles/legal.pdfl contract.pdf --output pdf --output-file review.pdf
```

---

## 12.3 Laboratory: package insert with batch code

**Problem:** the insert must carry the texts required by the regulator, and the
barcode must match the right product — swapping codes between products is the
sector's costliest mistake.

`profiles/insert.pdfl`:

```pdfl
profile "regulated-insert" {

  check "Mandatory texts" tags: ["regulatory"] {
    missing = data::validate_against_reference("databases/regulatory_texts.txt")
    assert missing.length == 0,
      "mandatory texts missing: #{missing.join("; ")}"
  }

  check "Legibility" tags: ["regulatory"] {
    // Regulators require a minimum body size on inserts
    assert prepress::validate_font_size(6),
      "there is text below 6 pt"
  }

  check "Barcode" tags: ["codes", "critical"] {
    assert codes::detect_barcodes(), "insert has no barcode"

    code = codes::decode_barcode(1)
    assert codes::validate_barcode_checksum(1),
      "invalid check digit: #{code}"

    // This check catches the costliest mistake: one product's code,
    // another product's text
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Approved product" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product,
      "GTIN #{code} is not in the product database"

    // The registered name must appear in print
    name = product.get(2)
    assert text::require_text(name),
      "the name '#{name}' does not appear on the insert"
    print("product verified:", name)
  }

  check "Code position" tags: ["layout"] {
    area = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(area),
      "code outside the reserved area — risk of being trimmed off"
  }
}
```

**Running it with the databases:**

```bash
PDFL_DATA_DIR=./databases pdfl run profiles/insert.pdfl insert_v3.pdf
```

---

## 12.4 Approval: comparing against the approved version

**Problem:** the client approved v1; v2 arrives claiming "I only changed one
word". Trusting that is expensive.

```bash
# What actually changed, as HTML for the client to see
pdfl compare approved/catalogue_v1.pdf received/catalogue_v2.pdf \
  --normalize --ignore-dates \
  --output html --output-file differences.html

echo "exit: $?"   # 0 identical · 1 metadata only · 2 content changed
```

To also check the **appearance** (not just the text), a script:

`profiles/fidelity.pdfl`:

```pdfl
profile "visual-fidelity" {

  const APPROVED = "approved/catalogue_v1.pdf"

  check "Pages visually identical" tags: ["approval"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APPROVED)
      assert ssim > 0.99,
        "page #{page.number} changed visually (SSIM #{ssim}, #{visual::pixel_diff(page.number, APPROVED)}% of pixels)"
    }
  }

  check "No image replaced" tags: ["approval"] {
    doc.pages.each { |page|
      assert !visual::detect_image_replacement(page.number, APPROVED),
        "page #{page.number}: image swapped compared to the approved version"
    }
  }
}
```

---

## 12.5 CI/CD: validating a whole batch

**Problem:** every file entering the repository must pass preflight, with nobody
running anything by hand.

`.github/workflows/preflight.yml`:

```yaml
name: PDF preflight

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install pdfl
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # automatic Actions token, no setup
        run: |
          gh release download --repo kotarokikuchi/pdflang \
            --pattern 'pdfl-*-linux-x64.tar.gz'
          mkdir pdfl && tar xzf pdfl-*-linux-x64.tar.gz --strip-components=1 -C pdfl
          echo "$PWD/pdfl" >> $GITHUB_PATH

      - name: Check the scripts themselves
        run: |
          for f in profiles/*.pdfl; do
            pdfl lint "$f"
            pdfl fmt "$f" --check
          done

      - name: Preflight every PDF
        run: |
          # --once processes what is in the folder and exits with the worst code
          pdfl watch files/ --script profiles/offset.pdfl \
            --output-dir reports/ --once

      - name: Publish the reports
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: reports
          path: reports/
```

In plain shell, with per-file control:

```bash
#!/usr/bin/env bash
# validate_batch.sh — validates a folder and prints a summary
set -uo pipefail

rejected=0
for file in inbox/*.pdf; do
  name=$(basename "$file" .pdf)
  if pdfl run profiles/offset.pdfl "$file" \
       --output json --output-file "reports/$name.json"; then
    echo "OK        $name"
  else
    echo "REJECTED  $name"
    rejected=$((rejected + 1))
  fi
done

echo "---"
echo "$rejected file(s) rejected"
exit $((rejected > 0))
```

---

## 12.6 Preparing a publisher's file for the print shop

**Problem:** the file arrives without production boxes, with review comments and
with layers that could be re-enabled by mistake.

`profiles/prepare.pdfl`:

```pdfl
// Validate before touching anything: a failed precondition shows in the report
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask for the open version"
}

// Production boxes the publisher never defined
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Cleanup
fix::remove_annotations()      // review comments
fix::remove_attachments()      // attachments that only add weight
fix::flatten_layers()          // layers must not be re-enabled
fix::remove_unused_resources() // file leftovers
```

```bash
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf --dry-run  # review
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf            # apply
pdfl run profiles/offset.pdfl print.pdf                                    # validate
```

---

## 12.7 Distributing profiles to the team

**Problem:** five machines must use exactly the same profiles and data files,
with a guarantee that nobody changed anything.

```bash
# On the machine that maintains the profiles
pdfl pack profiles/ --name print-profile --version 1.2.0
# produces print-profile.pdflpkg (scripts + data + SHA-256 manifest)

# On the production machines
pdfl add print-profile.pdflpkg
# installs into ./pdfl_profiles/print-profile@1.2.0/ verifying every hash

pdfl run pdfl_profiles/print-profile@1.2.0/offset.pdfl file.pdf
```

If the package was altered in transit, `add` **refuses to install it**.

---

## 12.8 Investigating a problematic file

A practical sequence when something is wrong and nobody knows what:

```bash
# 1. Overview in seconds
pdfl inspect suspect.pdf

# 2. An exploratory script, print() only
cat > investigate.pdfl <<'EOF'
check "X-ray" {
  print("exact TAC:", prepress::calculate_exact_tac(), "%")
  print("estimated TAC:", prepress::calculate_tac(), "%")
  print("spots:", prepress::detect_spot_colors().join(", "))
  print("rich black?", prepress::detect_rich_black())
  print("overprint ok?", prepress::validate_overprint_settings())
  print("loose fonts:", prepress::detect_text_substitution().join(", "))

  doc.images.each { |img|
    print("image page", img.page_number, ":", img.width, "x", img.height,
          "@", round(img.dpi), "DPI", img.color_space)
  }
}
EOF

pdfl run investigate.pdfl suspect.pdf > /dev/null
# print() goes to stderr, so the report is discarded
# and you only see the investigation
```

---

[← CLI commands](11-cli.md) · [Index](README.md)
