# 9. Espace de noms `data::` — données externes

[← `fix::`](08-fix.md) · [Sommaire](README.md) · [Suivant : bibliothèque standard →](10-stdlib.md)

8 fonctions pour recouper le contenu du PDF avec vos propres listes et tableaux.
Tout se passe en local : aucune donnée ne sort.

---

## 9.1 Où placer les fichiers

Glossaires et jeux de données acceptent un chemin **relatif au répertoire
d'exécution** :

```pdfl
data::load_glossary("termes/juridique.txt")
data::load_dataset("donnees/lots.csv")
```

Les tables de consultation (`query_gtin`, `query_medicamento`,
`query_postal_code`) utilisent des noms de fichiers fixes, cherchés dans cet
ordre :

1. `$PDFL_DATA_DIR` (variable d'environnement)
2. `./dados/`
3. `./`
4. Profils installés par `pdfl add` (`pdfl_profiles/*/dados/`)
5. Le dossier du PDF analysé

```bash
PDFL_DATA_DIR=/opt/bases pdfl run profil.pdfl document.pdf
```

Si rien n'est trouvé, le message d'erreur indique où placer le fichier. Pour
distribuer les données avec le profil, utilisez `pdfl pack`
([chapitre 11](11-cli.md)).

---

## 9.2 Glossaires et jeux de données

| Fonction | Rôle |
|---|---|
| `data::load_glossary(fichier)` | Liste de termes (un par ligne, `#` = commentaire) |
| `data::validate_against_reference(fichier)` | Liste des termes **absents** du document |
| `data::load_dataset(fichier)` | Lit un CSV comme une liste de lignes |
| `data::lookup_value(fichier, clé)` | 2e colonne de la ligne dont la 1re vaut la clé (`null` sinon) |

La comparaison ignore la casse et les espaces.

`termes/obligatoires.txt` :

```
# Termes que toute police d'assurance doit contenir
waiting period
covered benefits
general conditions
```

```pdfl
check "Glossary and dataset" {
  termes = data::load_glossary("termes/obligatoires.txt")
  print("terms in the glossary:", termes.length)

  // L'usage le plus direct
  manquants = data::validate_against_reference("termes/obligatoires.txt")
  assert manquants.length == 0,
    "clauses missing from the policy: #{manquants.join("; ")}"

  lignes = data::load_dataset("donnees/lots.csv")
  print("columns:", lignes.first().join(" | "))   // la 1re ligne est l'en-tête
  print("records:", lignes.length - 1)

  // null est faux, la validation s'écrit donc directement
  lot = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  description = data::lookup_value("donnees/lots.csv", lot)
  assert description, "batch #{lot} is not in the approved list"
}
```

---

## 9.3 Tables de consultation

Fichiers à noms fixes, cherchés dans l'ordre de la section 9.1. Elles retournent
la **ligne entière** sous forme de liste, ou `null` si rien n'est trouvé.

| Fonction | Fichier de référence | Rôle |
|---|---|---|
| `data::query_gtin(code)` | `gtin.csv` | Consultation par GTIN (ponctuation ignorée) |
| `data::query_medicamento(enreg_ou_nom)` | `medicamentos.csv` | Par numéro d'enregistrement ou fragment de nom |
| `data::query_postal_code(code)` | `ceps.csv` | Par code postal (8 chiffres) |
| `data::validate_address(code, "fragment")` | `ceps.csv` | L'adresse de ce code contient-elle le fragment ? |

`dados/gtin.csv` :

```csv
gtin,description,manufacturer
7891234567895,Dipyrone 500mg 20 tablets,Example Labs
```

```pdfl
check "Lookup tables" {
  // Recoupement avec le code-barres lu sur l'emballage
  code = codes::decode_barcode(1)
  produit = data::query_gtin(code)
  assert produit, "GTIN #{code} is not in the product database"
  print("product:", produit.get(2), "| manufacturer:", produit.get(3))

  // Informations du médicament par numéro d'enregistrement
  enregistrement = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
  medicament = data::query_medicamento(enregistrement)
  assert medicament, "registration #{enregistrement} not found"

  // Un médicament sur ordonnance exige la mention légale
  bandeau = medicament.get(4)
  assert bandeau != "prescription" || text::require_text("PRESCRIPTION ONLY"),
    "prescription medicine without the mandatory text"

  // L'adresse imprimée correspond-elle au code postal ?
  assert data::validate_address("01310100", "Avenida Paulista"),
    "printed address does not match the given postal code"
}
```

---

## 9.4 Exemple complet

```pdfl
// notice_avec_bases.pdfl — recoupement avec des données locales
// Usage : PDFL_DATA_DIR=./bases pdfl run notice_avec_bases.pdfl notice.pdf
profile "insert-with-references" {

  check "Mandatory regulatory terms" tags: ["glossary"] {
    manquants = data::validate_against_reference("bases/termes_reglementaires.txt")
    assert manquants.length == 0, "mandatory texts missing: #{manquants.join("; ")}"
  }

  check "Product in the database" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    produit = data::query_gtin(code)
    assert produit, "GTIN #{code} not approved"

    // Le nom enregistré doit apparaître sur l'imprimé
    nom = produit.get(2)
    assert text::require_text(nom),
      "the name '#{nom}' from the database does not appear on the insert"
    print("product verified:", nom)
  }

  check "Registration and band" tags: ["regulatory"] {
    enregistrement = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
    med = data::query_medicamento(enregistrement)
    assert med, "registration #{enregistrement} not found"
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

[← `fix::`](08-fix.md) · [Sommaire](README.md) · [Suivant : bibliothèque standard →](10-stdlib.md)
