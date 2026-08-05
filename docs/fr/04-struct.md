# 4. Espace de noms `struct::` — structure et métadonnées

[← `text::`](03-text.md) · [Sommaire](README.md) · [Suivant : `visual::` →](05-visual.md)

23 fonctions à propos du fichier lui-même : métadonnées, objets internes,
sécurité et traçabilité.

> Les fonctions à partir de `list_objects` lisent la structure interne du
> fichier. Cette analyse ne tourne **qu'une seule fois**, au premier usage, puis
> est mise en cache.

---

## 4.1 Métadonnées

| Fonction | Retourne |
|---|---|
| `struct::get_title()` | Le titre |
| `struct::get_author()` | L'auteur |
| `struct::get_subject()` | Le sujet |
| `struct::get_keywords()` | Les mots-clés |
| `struct::get_creator()` | Le programme d'origine du document |
| `struct::get_producer()` | Le programme qui a produit le PDF |
| `struct::get_creation_date()` | Date de création (`AAAA-MM-JJ HH:MM:SS`) |
| `struct::get_modification_date()` | Date de modification (même format) |
| `struct::list_metadata_entries()` | Liste des entrées non vides (`"Clé : valeur"`) |
| `struct::extract_xmp()` | Les métadonnées XMP du catalogue |

Toutes retournent une chaîne vide si le champ manque.

```pdfl
check "Required metadata" {
  assert struct::get_title() != "", "PDF has no title"
  assert struct::get_author() != "", "PDF has no author"

  // Producer révèle l'outil d'origine — utile pour remonter à un problème
  print("produced by:", struct::get_producer())

  creation = struct::get_creation_date()
  assert creation != "", "PDF has no creation date"
  // La comparaison de chaînes marche parce que le format se trie correctement
  assert creation > "2026-01-01", "file is too old for this campaign"

  xmp = struct::extract_xmp()
  assert xmp.contains("pdfaid"), "no PDF/A identification in the XMP"
}
```

---

## 4.2 Fichier et traçabilité

| Fonction | Rôle |
|---|---|
| `struct::file_size()` | Taille en octets |
| `struct::calculate_sha256()` | Empreinte SHA-256 du fichier |
| `struct::detect_file_bloat([ko_par_page])` | Vrai au-delà de la limite par page (1024 Ko par défaut) |

```pdfl
check "File size and traceability" {
  mo = struct::file_size() / 1024 / 1024
  assert mo < 10, "file is #{round(mo)} MB (10 MB e-mail limit)"

  // L'empreinte prouve exactement quel fichier a été approuvé
  print("SHA-256:", struct::calculate_sha256())

  assert !struct::detect_file_bloat(1024),
    "heavy file: #{struct::file_size() / 1024} KB for #{doc.page_count} pages"
}
```

---

## 4.3 Objets internes

| Fonction | Rôle |
|---|---|
| `struct::count_objects()` | Nombre d'objets de contenu dans les pages |
| `struct::list_objects()` | Tous les objets (`"numéro : type"`) |
| `struct::detect_unreferenced_objects()` | Objets inatteignables depuis le trailer |
| `struct::detect_orphaned_resources()` | Ressources inatteignables (polices, images) |
| `struct::measure_object_size(numéro)` | Taille approximative d'un objet, en octets |

> Les objets d'infrastructure (`ObjStm`, `XRef`) sont exclus : par définition ils
> ne sont jamais référencés depuis le trailer, et les signaler serait une fausse
> alerte.

```pdfl
check "File hygiene" {
  require struct::count_objects() > 0

  perdus = struct::detect_unreferenced_objects()
  assert perdus.length == 0,
    "#{perdus.length} unreferenced object(s): #{perdus.join(", ")}"

  orphelins = struct::detect_orphaned_resources()
  assert orphelins.length == 0,
    "unused embedded resources: #{orphelins.join(", ")} — run 'pdfl fix' with remove_unused_resources()"
}
```

---

## 4.4 Sécurité

| Fonction | Rôle |
|---|---|
| `struct::detect_javascript()` | Vrai s'il y a du JavaScript incorporé |
| `struct::detect_suspicious_actions()` | Liste des actions à risque |
| `struct::check_encryption()` | Vrai si le document est chiffré |
| `struct::validate_permissions()` | Vrai s'il n'y a aucune restriction |
| `struct::validate_signatures()` | Vrai s'il existe des champs de signature |

`detect_suspicious_actions` repère `JavaScript`, `Launch` (lance un programme),
`URI`, `SubmitForm`, `ImportData` et `GoToR`.

> `validate_signatures` vérifie la **présence** de ces champs. La validation
> cryptographique de la chaîne de certificats n'est pas faite dans cette version.

```pdfl
check "Security" {
  // Le JavaScript dans un PDF est un vecteur d'attaque courant
  // et inutile dans un document destiné à l'impression
  assert !struct::detect_javascript(), "PDF contains embedded JavaScript"

  actions = struct::detect_suspicious_actions()
  assert actions.length == 0,
    "suspicious actions in the PDF: #{actions.join("; ")}"

  // Un PDF chiffré peut échouer sur le RIP de l'imprimeur
  assert !struct::check_encryption(),
    "PDF is encrypted — remove protection before sending it to print"
  assert struct::validate_permissions(),
    "PDF has permission restrictions that may block processing"
}
```

---

## 4.5 Exemple complet

```pdfl
// audit.pdfl — vérification de conformité et de sécurité
profile "file-audit" {

  check "Identification" tags: ["metadata"] {
    assert struct::get_title() != "", "no title"
    assert struct::get_author() != "", "no author"
    assert struct::get_creation_date() != "", "no creation date"
    print("produced by:", struct::get_producer())
  }

  check "Traceability" tags: ["audit"] {
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
    orphelins = struct::detect_orphaned_resources()
    assert orphelins.length == 0, "unused resources: #{orphelins.join(", ")}"
    assert !struct::detect_file_bloat(1024), "bloated file"
  }
}
```

---

[← `text::`](03-text.md) · [Sommaire](README.md) · [Suivant : `visual::` →](05-visual.md)
