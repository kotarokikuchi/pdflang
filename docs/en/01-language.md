# 1. The PDFLang language

[← Index](README.md) · [Next: Document types →](02-types.md)

PDFLang is designed to be read by people who do not program. There are no
classes, no inheritance, no type declarations and no semicolons. A script is a
list of checks written almost in plain language.

---

## 1.1 The shape of a script

```pdfl
// Comments start with two slashes and run to the end of the line.

profile "profile-name" {         // profile is optional: it groups and names
                                 // the set; the name shows up in the report.

  const LIMIT = 300%             // constants: uppercase by convention

  check "Check Name" {           // each check becomes a report section
    require doc.page_count > 0   // one validation
  }

  check "Another Check" {        // as many checks as you need
    require doc.title != ""
  }
}
```

The `profile` wrapper is optional — a script can be just a list of checks:

```pdfl
check "Simple" {
  require doc.page_count > 0
}
```

### Tags on checks

Tags organize and visually filter checks in the report:

```pdfl
check "Ink within limit" tags: ["prepress", "colors"] {
  require prepress::validate_tac_limits(300)
}
```

---

## 1.2 Two ways to validate

Every validation uses `require` or `assert`. The only difference is the message
that appears in the report when the validation fails.

```pdfl
check "Comparing both forms" {

  // require: the message is generated from the expression itself.
  // On failure the report shows:
  //   "requirement not met: doc.page_count > 0"
  require doc.page_count > 0

  // assert: you write the message the end user will read.
  // On failure the report shows exactly:
  //   "PDF has no title in its metadata"
  assert doc.title != "", "PDF has no title in its metadata"
}
```

**Rule of thumb:** use `require` for obvious checks (where the expression speaks
for itself) and `assert` when whoever reads the report needs to understand the
problem without knowing the script.

### One failure does not stop the others

```pdfl
check "Three independent validations" {
  assert doc.page_count > 100, "too few pages"    // fails
  assert doc.title != "", "no title"              // still runs
  assert doc.author != "", "no author"            // this one too
}
```

The report lists **every** problem at once. That is deliberate: whoever gets the
file back wants the complete list of fixes, not one at a time.

The same holds between checks — if a check hits a runtime error (an undefined
variable, say), it becomes a diagnostic and the remaining checks keep running.

---

## 1.3 Values and types

### Numbers and units

```pdfl
check "Numbers" {
  x = 42          // integer
  y = 2.5         // decimal number

  // Measurement units become POINTS automatically (1 pt = 1/72 in):
  a = 3mm         // 8.5039... pt
  b = 2.5cm       // 70.866... pt
  c = 1in         // 72 pt
  d = 10pt        // 10 pt

  // Percentages keep their numeric value:
  limit = 300%    // 300

  require a < b            // compare directly, everything is points
  require c == 72.0
  require limit == 300
}
```

Writing `3mm` instead of `8.504` is the whole point: the script reads naturally
for someone who thinks in millimetres, and the conversion cannot go wrong.

### Text

```pdfl
check "Strings" {
  simple = "plain text"

  // Interpolation: #{...} inserts the value of any expression
  name = "document.pdf"
  message = "Analyzing #{name} with #{doc.page_count} pages"

  // Escapes: \n (newline), \t (tab), \" (quote), \\ (backslash)
  quoted = "he said \"hello\""

  // Unknown backslashes pass through untouched — that lets you write
  // regular expressions without double escaping:
  pattern = "\d{3}\.\d{3}\.\d{3}-\d{2}"    // Brazilian CPF

  require message.contains("pages")
}
```

### Booleans and what counts as true

```pdfl
check "True and false" {
  yes = true
  no = false

  // Only false and null are falsy. Everything else is truthy —
  // including 0, the empty string and the empty list.
  require 0        // passes (zero is truthy)
  require ""       // passes (empty string is truthy)

  // So to test for content, compare explicitly:
  require doc.title != ""              // correct
  require doc.pages.length > 0         // correct
}
```

This matters for functions that return `null` when they find nothing:

```pdfl
check "Taking advantage of null" {
  description = data::lookup_value("batches.csv", "L2026-08")
  // null is falsy, so this works directly:
  assert description, "batch not found in the table"
}
```

### Lists

```pdfl
check "Lists" {
  numbers = [1, 2, 3]
  words = ["a", "b", "c"]
  mixed = [1, "two", true]

  require numbers.length == 3
  require numbers.contains(2)
  require words.join(", ") == "a, b, c"

  // Access is 1-based: the first item is item 1
  require numbers.get(1) == 1
  require numbers.first() == 1
  require numbers.last() == 3
}
```

---

## 1.4 Operators

```pdfl
check "Operators" {
  // Comparison
  require 10 > 5
  require 10 >= 10
  require 3 < 4
  require 3 <= 3
  require "a" == "a"
  require "a" != "b"

  // Arithmetic
  require 2 + 3 == 5
  require 10 - 4 == 6
  require 3 * 4 == 12
  require 10 / 4 == 2.5        // inexact division yields a decimal
  require 10 / 5 == 2          // exact division stays an integer

  // Logic (short-circuiting: the right side is only evaluated if needed)
  require true && true
  require false || true
  require !false

  // Short-circuit in practice: with no pages, the second half is never
  // evaluated — no error on an empty document.
  require doc.page_count == 0 || doc.pages.first().width > 0
}
```

---

## 1.5 Blocks: repeating for each item

Blocks are pieces of code in braces that take a parameter between vertical bars.
They read like a sentence: "for each page, do...".

```pdfl
check "Walking through pages" {

  // each: runs the block for every item
  doc.pages.each { |page|
    assert page.width > 0, "page #{page.number} has no width"
  }

  // each_with_index: also receives the position (0, 1, 2...)
  doc.fonts.each_with_index { |font, i|
    print("font", i, ":", font.name)
  }

  // all: true when EVERY item satisfies the condition
  require doc.fonts.all { |f| f.is_embedded }

  // any: true when AT LEAST ONE item satisfies it
  require doc.pages.any { |p| p.extract_text() != "" }

  // filter: keeps only the items that satisfy the condition
  blank = doc.pages.filter { |p| p.extract_text() == "" }
  assert blank.length == 0,
    "#{blank.length} blank page(s)"

  // map: transforms each item into a new list
  names = doc.fonts.map { |f| f.name }
  print("fonts in use:", names.join(", "))
}
```

Blocks can be chained — **on a single line**, with no break before the dot:

```pdfl
check "Chaining" {
  // non-embedded fonts, names only, joined by commas
  problems = doc.fonts.filter { |f| !f.is_embedded }.map { |f| f.name }
  assert problems.length == 0,
    "fonts not embedded: #{problems.join(", ")}"
}
```

If the line gets too long, break it into named steps instead of breaking the
chain — it reads better anyway:

```pdfl
check "Named steps" {
  loose = doc.fonts.filter { |f| !f.is_embedded }
  names = loose.map { |f| f.name }
  assert names.length == 0, "fonts not embedded: #{names.join(", ")}"
}
```

---

## 1.6 Functions: naming your rules

When the same verification shows up in several places, give it a name:

```pdfl
// A function's value is that of its LAST expression — there is no "return".
function is_a4(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}

function exceeds_ink(page, limit) {
  page.tac > limit
}

check "Format and ink" {
  // now the check reads almost like a sentence
  require doc.pages.all { |p| is_a4(p) }

  doc.pages.each { |page|
    assert !exceeds_ink(page, 300), "page #{page.number} has too much ink"
  }
}
```

Rules for functions:

- Parameters exist only inside the function.
- Functions may call other functions.
- Recursion is allowed but capped at 200 calls (so a runaway script cannot hang
  the process).

---

## 1.7 Imports: sharing between profiles

Put common rules in one file and import it wherever you need.

`library.pdfl`:

```pdfl
// Constants and functions shared across the team
const OFFSET_TAC = 300%
const DEFAULT_BLEED = 3mm

function a4_page(page) {
  abs(page.width - 595.0) < 5.0 && abs(page.height - 842.0) < 5.0
}
```

`magazine.pdfl`:

```pdfl
// The path is relative to THIS file
import "library.pdfl"

check "Format" {
  // OFFSET_TAC and a4_page came from the import
  require doc.pages.all { |p| a4_page(p) }
  require prepress::validate_tac_limits(OFFSET_TAC)
}
```

Each file is loaded **exactly once**, even if several scripts import it — so
circular imports do not hang.

---

## 1.8 Rules (`rule`): validating page by page

A `rule` is a check that runs once per page, with the page already bound to the
`page` variable:

```pdfl
// Without "on": runs on every page
rule "Every page has text" {
  assert page.extract_text().trim() != "",
    "page #{page.number} is blank"
}
```

With `on`, you choose which pages the rule applies to:

```pdfl
rule "Body pages numbered" on doc.pages.filter { |p| p.number > 2 } {
  footer = region(0, 0, page.width, 60)
  assert text::extract_from_region(page.number, footer) != "",
    "page #{page.number} has no page number in the footer"
}
```

> **Syntax note:** if the `on` selection ends in a property (e.g. `on doc.pages`),
> wrap it in parentheses — without them, the `{` of the body would be read as a
> block belonging to that call:
>
> ```pdfl
> rule "Example" on (doc.pages) {     // parentheses required
>   require page.width > 0
> }
> ```

---

## 1.9 Variables and scope

```pdfl
const GLOBAL = 100          // visible throughout the file

check "Scope" {
  local = 42                // visible only inside this check

  doc.pages.each { |page|
    inner = page.width      // visible only inside the block
    require inner > 0
  }

  require local == 42       // still visible
  require GLOBAL == 100     // still visible
}
```

Convention: constants in UPPERCASE, variables in lowercase. The language does not
enforce it, but the examples and shipped profiles follow it.

---

## 1.10 Messages that help whoever gets the file

The quality of the report depends on the messages you write. Compare:

```pdfl
check "Poor messages" {
  require doc.pages.all { |p| p.tac <= 300 }
  // report: "requirement not met: doc.pages.all() { ... }"
  // — the recipient has no idea which page or by how much
}

check "Good messages" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "Page #{page.number}: ink coverage #{page.tac}% (max 300%)"
  }
  // report: "Page 7: ink coverage 324% (max 300%)"
  // — the operator knows exactly what to fix
}
```

Use `print()` for context that is not an error. It goes to stderr, so it never
pollutes the report:

```pdfl
check "Context" {
  print("Analyzing", doc.page_count, "pages")
  print("Fonts:", prepress::list_fonts().join(", "))
  require doc.page_count > 0
}
```

---

## 1.11 Common errors

| Message | Cause | Fix |
|---|---|---|
| `expected end of line after statement` | two statements on one line | one statement per line |
| `unknown variable: x` | used before assignment, or out of scope | declare it first, at the same level |
| `unknown function: text::xyz` | wrong name or nonexistent function | check the namespace chapter |
| `fix:: is only available in the 'pdfl fix' command` | `fix::` used under `pdfl run` | use `pdfl fix input.pdf script.pdfl --output out.pdf` |
| `unknown unit: 'kg'` | invalid unit suffix | use `pt`, `mm`, `cm`, `in` or `%` |
| `expected '{' with the rule body` | `on` selection ends in a property | wrap the selection in parentheses |
| `unexpected expression: Dot` | chain split across lines | keep `.method` on the same line, or use intermediate variables |

Before running, it is always worth doing:

```bash
pdfl lint my_profile.pdfl    # unused variables, duplicate checks...
pdfl fmt my_profile.pdfl     # standardize formatting
```

---

[← Index](README.md) · [Next: Document types →](02-types.md)
