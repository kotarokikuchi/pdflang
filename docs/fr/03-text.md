# 3. Espace de noms `text::` — le texte

[← Types](02-types.md) · [Sommaire](README.md) · [Suivant : `struct::` →](04-struct.md)

25 fonctions pour extraire, normaliser, chercher et valider le texte d'un
document.

> Dans les fonctions marquées `[text]`, l'argument est **facultatif** : sans
> lui, la fonction travaille sur tout le document ; avec lui, sur la chaîne que
> vous passez.

---

## 3.1 Extraction

| Fonction | Rôle |
|---|---|
| `text::extract_all()` | Tout le texte du document (pages jointes par des sauts de ligne) |
| `text::extract_from_page(page)` | Le texte d'une page (à partir de 1) |
| `text::extract_from_region(page, region)` | Le texte d'une zone (chaîne vide s'il n'y en a pas) |
| `text::extract_with_normalization()` | Le texte du document déjà normalisé |

```pdfl
check "Extraction" {
  contenu = text::extract_all()
  assert contenu.trim() != "", "PDF has no extractable text"

  couverture = text::extract_from_page(1)
  assert couverture.contains("User Manual"), "cover lacks the expected title"

  // Les pieds de page de fabrication (nom du fichier InDesign, date d'export)
  // survivent parfois jusqu'au fichier final
  pied = region(0, 0, 467, 40, "footer")
  doc.pages.each { |page|
    ligne = text::extract_from_region(page.number, pied)
    assert !ligne.contains(".indd"),
      "page #{page.number} has a production mark in the footer: #{ligne.trim()}"
  }
}
```

---

## 3.2 Normalisation et découpage

| Fonction | Rôle |
|---|---|
| `text::normalize([text])` | Minuscules + espaces compactés |
| `text::split_words([text])` | Découpe en mots (ponctuation des bords retirée) |
| `text::split_sentences([text])` | Découpe en phrases |
| `text::split_paragraphs([text])` | Découpe en paragraphes (ligne vide) |
| `text::count_words([text])` | Nombre de mots |
| `text::count_characters([text])` | Nombre de caractères |
| `text::detect_language([text])` | `"pt"`, `"en"`, `"es"` ou `"unknown"` |

```pdfl
check "Normalization and splitting" {
  require text::normalize("  HELLO   World  ") == "hello world"

  mots = text::split_words("Hello, world! (test)")
  require mots.length == 3
  require mots.first() == "Hello"

  // Notices et contrats ont une limite pratique de lisibilité
  text::split_sentences().each { |phrase|
    assert phrase.length < 400,
      "sentence with #{phrase.length} characters — hard to read"
  }

  require text::count_words() > 100
  assert text::detect_language() == "en",
    "document should be in English, detected: #{text::detect_language()}"
}
```

---

## 3.3 Recherche et contenu obligatoire

| Fonction | Rôle |
|---|---|
| `text::require_text(terme)` | Vrai si le terme est présent |
| `text::forbid_text(terme)` | Vrai si le terme est absent |
| `text::require_match(regex)` | Vrai si l'expression régulière trouve quelque chose |
| `text::forbid_match(regex)` | Vrai si elle ne trouve rien |
| `text::fuzzy_match(a, b)` | Similarité entre deux chaînes (0.0 à 1.0) |

La comparaison ignore la casse et les espaces.

```pdfl
profile "contract" {
  check "Mandatory clauses" {
    assert text::require_text("governing law"),
      "contract has no governing-law clause"
    assert text::require_match("\d{4}/\d{4}"), "contract number not found"
  }

  check "Forbidden terms" {
    assert text::forbid_text("DRAFT"), "document still marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text was not replaced"
    assert text::forbid_match("\d{2}-\d{2}-\d{4}"), "US-format date found"
  }

  check "Name with tolerance" {
    // Utile quand des fautes de frappe ou du bruit d'OCR sont attendus
    trouve = text::extract_from_region(1, region(50, 700, 300, 40))
    similarite = text::fuzzy_match("Paracetamol 750mg", trouve)
    assert similarite > 0.9,
      "product name differs from expected (#{round(similarite * 100)}% similar)"
  }
}
```

---

## 3.4 Données personnelles

`text::detect_personal_data()` et `text::detect_pii()` sont synonymes. Elles
retournent la **liste** des données personnelles trouvées : CPF, CNPJ
(identifiants fiscaux brésiliens), adresse e-mail et téléphone.

> CPF et CNPJ n'entrent dans la liste que si la **clé de contrôle est valide**.
> Un numéro qui ressemble seulement à un CPF (`111.111.111-12`) ne déclenche
> aucune alerte.

```pdfl
check "Public document must carry no personal data" {
  trouves = text::detect_personal_data()
  assert trouves.length == 0, "personal data exposed: #{trouves.join("; ")}"

  // Chaque entrée ressemble à "CPF: 529.982.247-25"
  text::detect_pii().each { |item| print("found:", item) }
}
```

---

## 3.5 Validations de format

| Fonction | Rôle |
|---|---|
| `text::validate_cpf(text)` | Clé de contrôle du CPF (mod 11) |
| `text::validate_cnpj(text)` | Clé de contrôle du CNPJ |
| `text::validate_date_format(text [, format])` | Date réellement valide au calendrier |
| `text::validate_phone_format(text)` | Format de téléphone brésilien |
| `text::validate_format(text, regex)` | La chaîne **entière** correspond-elle ? |

Formats de date acceptés : `"dd/mm/aaaa"` et `"aaaa-mm-dd"` ; sans deuxième
argument, les deux sont acceptés.

```pdfl
check "Format validation" {
  require text::validate_cpf("529.982.247-25")
  require !text::validate_cpf("111.111.111-11")    // chiffres tous identiques
  require text::validate_cnpj("11.222.333/0001-81")

  require text::validate_date_format("29/02/2024")   // 2024 est bissextile
  require !text::validate_date_format("29/02/2023")  // 2023 ne l'est pas
  require !text::validate_date_format("31/04/2026")  // avril a 30 jours

  require text::validate_phone_format("(11) 98765-4321")

  // Code de lot au format de l'usine
  lot = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  assert text::validate_format(lot, "L\d{4}-\d{2}"),
    "batch code does not follow the L0000-00 pattern: #{lot}"
}
```

---

## 3.6 Comparaison et diagnostic

`text::diff(a, b)` liste les lignes qui changent (`-` retirée, `+` ajoutée).
`text::detect_rasterized_text()` est vrai s'il existe du texte transformé en
image.

```pdfl
check "Comparison and diagnostics" {
  changements = text::diff(text::extract_from_page(1), text::extract_from_page(2))
  print("changed lines:", changements.length)

  // Une page scannée ou vectorisée n'est ni cherchable ni lisible
  // par un lecteur d'écran
  assert !text::detect_rasterized_text(),
    "there are pages with rasterized text (scanned or outlined)"
}
```

> Pour comparer deux **fichiers**, utilisez la commande `pdfl compare` : elle
> aligne les pages automatiquement. Voir le [chapitre 11](11-cli.md).

---

## 3.7 Exemple complet

```pdfl
// document_juridique.pdfl — validation d'un contrat
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
    trouves = text::detect_personal_data()
    assert trouves.length == 0,
      "personal data in a public document: #{trouves.join("; ")}"
  }

  check "Text quality" tags: ["text"] {
    assert text::detect_language() == "en", "document is not in English"
    assert !text::detect_rasterized_text(), "rasterized text blocks search"
    require text::count_words() > 200
  }
}
```

---

[← Types](02-types.md) · [Sommaire](README.md) · [Suivant : `struct::` →](04-struct.md)
