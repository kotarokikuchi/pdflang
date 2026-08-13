# 9. `data::` namespace — external data

[← `fix::`](08-fix.md) · [Index](README.md) · [Next: Standard library →](10-stdlib.md)

8 functions to cross-check the PDF content against your own lists and tables.
Everything is local: no data leaves the machine.

---

## 9.1 Where the files live

Glossaries and datasets take a **path relative to the working directory**:

```pdfl
data::load_glossary("terms/legal.txt")
data::load_dataset("data/batches.csv")
```

Lookup tables (`query_gtin`, `query_medicamento`, `query_postal_code`) have fixed
names and are searched for in this order:

1. `$PDFL_DATA_DIR` (environment variable)
2. `./dados/`
3. `./`
4. Profiles installed by `pdfl add` (`pdfl_profiles/*/dados/`)
5. Next to the analyzed PDF

```bash
# Pointing explicitly at the data folder
PDFL_DATA_DIR=/opt/databases pdfl run profile.pdfl document.pdf
```

If a table cannot be found, the error message says where to put it.

To ship tables together with profiles, use `pdfl pack` — see
[chapter 11](11-cli.md#pdfl-pack).

---

## 9.2 Glossaries

A glossary is a text file with one term per line. Blank lines and lines starting
with `#` are ignored.

`terms/required.txt`:

```
# Terms every policy must contain
waiting period
covered benefits
general conditions
```

### `data::load_glossary(file)`

Loads the glossary as a list of terms.

```pdfl
check "Glossary loaded" {
  terms = data::load_glossary("terms/required.txt")
  print("terms in the glossary:", terms.length)
  require terms.contains("general conditions")
}
```

### `data::validate_against_reference(file)`

The most direct route: returns the list of glossary terms that do **not** appear
in the document. An empty list means everything is there.

```pdfl
check "Mandatory clauses" {
  missing = data::validate_against_reference("terms/required.txt")
  assert missing.length == 0,
    "clauses missing from the policy: #{missing.join("; ")}"
}
```

Comparison ignores case and spacing — "GENERAL  CONDITIONS" satisfies
"general conditions".

---

## 9.3 Datasets (CSV and JSON)

### `data::load_dataset(file)`

Loads a CSV or JSON file as a list of rows; each row is a list of columns. In
CSV, quotes follow the standard (a quoted field may contain commas); JSON is
described below.

`data/batches.csv`:

```csv
batch,description,expiry
L2026-08,Approved batch August/2026,2028-08-01
L2026-09,Approved batch September/2026,2028-09-01
```

```pdfl
check "Walking the table" {
  rows = data::load_dataset("data/batches.csv")

  // The first row is the header
  print("columns:", rows.first().join(" | "))
  print("records:", rows.length - 1)

  // get(n) is 1-based: get(1) is the first column
  rows.each { |row|
    print(row.get(1), "->", row.get(2))
  }
}
```

### JSON datasets

A file ending in `.json` is read as JSON — by `load_dataset` and by
`lookup_value` alike. Two shapes are accepted, because those are the two a
dataset is actually written in.

An array of arrays is the rows as they stand:

```json
[["batch", "description"],
 ["L2026-08", "Approved batch August/2026"]]
```

An array of objects turns into a header row plus one row per object. The columns
are ordered as the **first** object writes them, not alphabetically, so the
first key stays the key `lookup_value` searches:

```json
[{"batch": "L2026-08", "description": "Approved batch August/2026"},
 {"batch": "L2026-09", "description": "Approved batch September/2026"}]
```

A key missing from a later object leaves an **empty cell**, never a shifted row:
a hole is visible in the report, a shift is not. Numbers keep the digits they
were written with, and `null` is an empty cell — what an empty CSV field means.

Mixing the two shapes in one file is an error that names the row.

### `data::lookup_value(file, key)`

Looks the key up in the first column and returns the **second** column's value,
in a CSV or a JSON file alike.
Returns `null` when not found — and since `null` is falsy, you can test it
directly.

```pdfl
check "Approved batch" {
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()

  description = data::lookup_value("data/batches.csv", batch)
  assert description,
    "batch #{batch} is not in the approved list"

  print("batch recognized:", description)
}
```

---

## 9.4 Lookup tables

These functions look for fixed-name files in the folders described in 9.1 and
return the **whole row** as a list (or `null`).

### `data::query_gtin(code)`

Queries `gtin.csv`. Punctuation in the code is ignored.

`dados/gtin.csv`:

```csv
gtin,description,manufacturer
7891234567895,Dipyrone 500mg 20 tablets,Example Labs
```

```pdfl
check "Approved product" {
  // Cross-checking against the code read from the packaging itself
  code = codes::decode_barcode(1)
  product = data::query_gtin(code)

  assert product,
    "GTIN #{code} is not in the product database"

  print("product:", product.get(2))
  print("manufacturer:", product.get(3))
}
```

### `data::query_medicamento(registration_or_name)`

Queries `medicamentos.csv`. Accepts the registration number (first column) or
part of the name (second column).

`dados/medicamentos.csv`:

```csv
registration,name,active_ingredient,band
1.0298.0123,Dipyrone,dipyrone monohydrate,otc
1.0298.0456,Amoxicillin,amoxicillin trihydrate,prescription
```

```pdfl
check "Correct band on the insert" {
  registration = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
  medicine = data::query_medicamento(registration)

  assert medicine,
    "registration #{registration} not found in the regulatory database"

  // If the band is prescription-only, the mandatory text must be in the artwork
  band = medicine.get(4)
  print("medicine:", medicine.get(2), "| band:", band)

  assert band != "prescription" || text::require_text("PRESCRIPTION ONLY"),
    "prescription medicine without the mandatory text"
}
```

### `data::query_postal_code(code)`

Queries `ceps.csv`. Accepts the Brazilian postal code with or without a hyphen;
requires 8 digits.

`dados/ceps.csv`:

```csv
cep,street,district,city,state
01310100,Avenida Paulista,Bela Vista,Sao Paulo,SP
```

```pdfl
check "Manufacturer address" {
  address = data::query_postal_code("01310-100")
  assert address, "postal code not found in the database"

  print("street:", address.get(2))
  print("city:", address.get(4), "-", address.get(5))
}
```

### `data::validate_address(postal_code, "fragment")`

Checks whether the given fragment appears in the address for that postal code.

```pdfl
check "Printed address matches the postal code" {
  // The address on the packaging must match the declared postal code
  assert data::validate_address("01310100", "Avenida Paulista"),
    "printed address does not match the given postal code"
}
```

---

## 9.5 Complete example

```pdfl
// insert_with_databases.pdfl — validation cross-checking the PDF against local data
// Usage: PDFL_DATA_DIR=./databases pdfl run insert_with_databases.pdfl insert.pdf
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

    // The registered name must appear in print
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

[← `fix::`](08-fix.md) · [Index](README.md) · [Next: Standard library →](10-stdlib.md)
