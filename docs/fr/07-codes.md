# 7. Espace de noms `codes::` — codes-barres et QR codes

[← `prepress::`](06-prepress.md) · [Sommaire](README.md) · [Suivant : `fix::` →](08-fix.md)

13 fonctions pour détecter, décoder et vérifier les codes-barres et QR codes d'un
document.

> La lecture rend les pages en haute résolution et n'a lieu **qu'une fois**, au
> premier appel d'une fonction `codes::`. Un script qui n'utilise pas cet espace
> de noms ne paie pas ce coût.

Formats reconnus : EAN-8/13, UPC-A/E, Code 128, Code 39, ITF, QR Code, Data
Matrix, Aztec et PDF417.

---

## 7.1 Détection

| Fonction | Rôle |
|---|---|
| `codes::detect_barcodes()` | Vrai s'il y a un code-barres |
| `codes::detect_qrcodes()` | Vrai s'il y a un QR code |
| `codes::count_barcodes()` | Nombre total de codes lus |
| `codes::get_barcode_type(n)` | Format du n-ième code (`"EAN_13"`, `"QR_CODE"`…) |
| `codes::get_barcode_location(n)` | Position `[page, x, y]` en points, origine en bas à gauche |

```pdfl
check "Codes present" {
  assert codes::detect_barcodes(), "no barcode found in the artwork"
  assert codes::detect_qrcodes(), "the traceability QR code is missing"

  total = codes::count_barcodes()
  assert total == 2, "expected 2 codes (EAN + QR), found #{total}"

  type = codes::get_barcode_type(1)
  assert type == "EAN_13", "the main code should be EAN-13, it is #{type}"

  endroit = codes::get_barcode_location(1)
  assert endroit.first() == 1, "barcode is not on the cover"
}
```

---

## 7.2 Décodage et vérification

| Fonction | Rôle |
|---|---|
| `codes::decode_barcode(n)` | Contenu du n-ième code |
| `codes::validate_barcode_checksum(n)` | Clé GTIN du n-ième code |
| `codes::validate_gtin(text)` / `codes::validate_ean(text)` | Clé GTIN d'une chaîne |
| `codes::validate_code128()` | Vrai s'il existe un Code 128 décodé avec succès |

```pdfl
check "Code integrity" {
  code = codes::decode_barcode(1)
  print("code read:", code)

  // Un GTIN à clé fausse est rejeté en caisse
  assert codes::validate_barcode_checksum(1),
    "invalid check digit in code #{code}"
  assert code.starts_with("789"),
    "GTIN is not Brazilian (should start with 789): #{code}"

  // Recoupement avec le numéro imprimé sous le code
  imprime = text::extract_from_region(1, region(400, 20, 180, 15)).trim()
  assert codes::validate_ean(imprime),
    "the printed number is not a valid GTIN: #{imprime}"
}
```

---

## 7.3 Recoupements

| Fonction | Rôle |
|---|---|
| `codes::compare_barcode_with_text()` | Vrai si le contenu de chaque code apparaît dans le texte |
| `codes::validate_barcode_format(regex)` | Vrai si tous les contenus correspondent à l'expression |
| `codes::validate_barcode_position(region)` ou `(x0, y0, x1, y1)` | Vrai si tous les codes sont dans la zone |

`compare_barcode_with_text` attrape l'erreur la plus coûteuse du secteur : le
code pointe un produit, le texte imprimé en annonce un autre.

```pdfl
check "Cross-checks" {
  assert codes::compare_barcode_with_text(),
    "the barcode number does not appear in the text — artwork with swapped data?"

  // Seul l'EAN-13 est admis : exactement 13 chiffres
  assert codes::validate_barcode_format("^\d{13}$"),
    "there is a code outside the EAN-13 pattern"

  // Une région nommée se relit mieux
  zone = region(400, 20, 180, 80, "barcode area")
  assert codes::validate_barcode_position(zone),
    "barcode outside the reserved area of the packaging"
}
```

---

## 7.4 Exemple complet

```pdfl
// notice.pdfl — contrôle des codes d'une notice de médicament
// Usage : pdfl run notice.pdfl notice.pdf
profile "medicine-insert" {

  check "Codes present" tags: ["codes"] {
    assert codes::detect_barcodes(), "insert has no barcode"
    assert codes::count_barcodes() >= 1, "expected at least the product EAN"
  }

  check "Code integrity" tags: ["codes"] {
    code = codes::decode_barcode(1)
    type = codes::get_barcode_type(1)
    print("code:", type, "=", code)

    assert type == "EAN_13", "main code is not EAN-13 (it is #{type})"
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"
    assert code.starts_with("789"), "GTIN is not Brazilian: #{code}"
  }

  check "Cross-check with the text" tags: ["codes", "critical"] {
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Position in the artwork" tags: ["codes", "layout"] {
    reservee = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(reservee),
      "code outside the reserved area — risk of being trimmed off"
  }

  check "Cross-check with the product database" tags: ["data"] {
    // Se combine avec data:: — voir le chapitre 9
    code = codes::decode_barcode(1)
    produit = data::query_gtin(code)
    assert produit, "GTIN #{code} is not in the approved product database"
    print("product:", produit.get(2))
  }
}
```

---

[← `prepress::`](06-prepress.md) · [Sommaire](README.md) · [Suivant : `fix::` →](08-fix.md)
