# 8. Espace de noms `fix::` — normalisation

[← `codes::`](07-codes.md) · [Sommaire](README.md) · [Suivant : `data::` →](09-data.md)

19 opérations qui **modifient** le PDF et l'enregistrent sous un nouveau nom. Le
fichier d'origine n'est jamais touché.

---

## 8.1 Comment on s'en sert

`fix::` est le seul espace de noms qui écrit ; il a donc sa propre commande :

```bash
pdfl fix entree.pdf script.pdfl --output corrige.pdf
```

| Option | Rôle |
|---|---|
| `--output <fichier>` | PDF de sortie (obligatoire) |
| `--dry-run` | Liste les opérations sans rien enregistrer |
| `--report json\|csv\|html\|pdf` | Format du rapport |
| `--report-file <fichier>` | Écrit le rapport dans un fichier |

Appeler `fix::` depuis `pdfl run` produit une erreur qui indique la bonne
commande — pour que personne ne modifie un fichier en croyant seulement le
valider.

### Comment s'exécutent les opérations

```pdfl
// Ce script n'a pas besoin de check : ce sont des commandes
// exécutées dans l'ordre.
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::add_page_numbers()
fix::add_watermark("DRAFT")
```

Chaque appel est **validé sur place** (page inexistante, rotation invalide,
fichier absent) avant d'être appliqué. Le rapport garde trace de ce qui a été
fait dans le champ `fixes` :

```json
"fixes": [
  "TrimBox set to [8.5, 8.5, 586.5, 833.5]",
  "page numbering added",
  "watermark \"DRAFT\" added"
]
```

Mélanger validations et modifications dans un même script fonctionne très bien :

```pdfl
// Valider avant de modifier — si la condition n'est pas tenue, ça se voit
// dans le rapport
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(), "file is encrypted, cannot fix it"
}

fix::add_page_numbers()
```

---

## 8.2 Boîtes de page

| Opération | Rôle |
|---|---|
| `fix::set_page_size(largeur, hauteur)` | Définit la MediaBox de toutes les pages |
| `fix::set_crop_box(x0, y0, x1, y1)` | Définit la CropBox de toutes les pages |
| `fix::set_trim_box(x0, y0, x1, y1)` | Définit la TrimBox de toutes les pages |
| `fix::set_bleed_box(x0, y0, x1, y1)` | Définit la BleedBox de toutes les pages |

Coordonnées en points, du coin inférieur gauche au coin supérieur droit.

```pdfl
// Écrivez avec des unités, la conversion se fait toute seule
fix::set_page_size(210mm, 297mm)

// Le fichier reçu de l'éditeur n'a aucune boîte de fabrication :
// TrimBox = format fini, BleedBox = avec 3 mm de fond perdu
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)
```

---

## 8.3 Pages

| Opération | Rôle |
|---|---|
| `fix::rotate_page([page,] degrés)` | Rotation de 90/180/270° (toutes les pages sans numéro) |
| `fix::delete_page(n)` | Supprime une page |
| `fix::duplicate_page(n)` | Duplique une page (la copie se place juste après) |
| `fix::reorder_pages([nouvel, ordre])` | Réordonne (chaque page exactement une fois) |
| `fix::split_document(de, à, "sortie.pdf")` | Enregistre un intervalle de pages dans un fichier |
| `fix::merge_documents("autre.pdf")` | Ajoute les pages d'un autre PDF à la fin |

Supprimer l'unique page du document est refusé explicitement.

```pdfl
fix::rotate_page(90)        // toutes les pages
fix::rotate_page(3, 180)    // seulement la page 3
fix::delete_page(1)         // enlève la couverture provisoire
fix::reorder_pages([4, 1, 2, 3])

// Couverture et intérieur partent chez deux fournisseurs différents
fix::split_document(1, 2, "couverture.pdf")
fix::split_document(3, 50, "interieur.pdf")

fix::merge_documents("annexes/garantie.pdf")
```

---

## 8.4 Contenu

| Opération | Rôle |
|---|---|
| `fix::add_watermark("texte")` | Filigrane gris en diagonale sur toutes les pages |
| `fix::add_stamps("texte")` | Cachet rouge en haut à droite de chaque page |
| `fix::add_page_numbers()` | Ajoute `n / total` en pied de page |
| `fix::remove_annotations()` | Supprime toutes les annotations |
| `fix::remove_attachments()` | Supprime toutes les pièces jointes |
| `fix::flatten_layers()` | Défait la structure de contenu optionnel (OCG) |

```pdfl
fix::add_watermark("DRAFT — DO NOT PRINT")
fix::add_stamps("APPROVED 2026-08-02")
fix::add_page_numbers()

// Avant l'impression : les commentaires de relecture ne doivent pas passer,
// et les pièces jointes ne font qu'alourdir le fichier
fix::remove_annotations()
fix::remove_attachments()

// Évite qu'un calque « version anglaise », désactivé, soit rallumé chez l'imprimeur
fix::flatten_layers()
```

---

## 8.5 Optimisation

> Les opérations de cette section **n'écrivent que si le fichier rétrécit**. Si
> la réécriture donne plus gros, l'original est conservé.

| Opération | Rôle |
|---|---|
| `fix::remove_unused_resources()` | Jette les objets inatteignables depuis le trailer |
| `fix::downsample_images([dpi])` | Rééchantillonne les images au-dessus du DPI visé (300 par défaut) |
| `fix::compress_images([qualité])` | Réencode en JPEG (1 à 100, 85 par défaut) |

Le DPI est calculé d'après la **taille réellement imprimée** sur la page.

> **Les images CMJN sont conservées.** Les rééchantillonner exigerait de passer
> par le RVB, ce qui casserait la séparation prépresse. Dans un fichier
> d'imprimerie, le gain vient des images RVB.

```pdfl
// Une version d'approbation par e-mail n'a pas besoin de 300 DPI
fix::downsample_images(96)
fix::compress_images(70)
fix::remove_unused_resources()
```

### Ce qui n'existe pas ici

`subset_fonts` et `linearize_document` **ne sont pas** des opérations `fix::` ;
les appeler donne une erreur de fonction inconnue.

- **subset_fonts** : implémenté puis mesuré. Les outils professionnels
  n'incorporent déjà que les glyphes utilisés ; le gain mesuré était de 0,5 %
  au mieux et nul ailleurs — pas au prix du risque d'abîmer une police. Pour
  *vérifier* si les polices sont bien des sous-ensembles, utilisez
  [`prepress::subset_fonts()`](06-prepress.md).
- **linearize_document** : demande de générer les tables d'indices (§ 7.14 de la
  spécification PDF). Aucune bibliothèque Rust ne le fait, et une implémentation
  partielle n'est pas reconnue comme « Fast Web View » par les lecteurs.

---

## 8.6 Exemples complets

```pdfl
// preparer_impression.pdfl — met en forme un fichier d'éditeur pour l'imprimerie
// Usage : pdfl fix editeur.pdf preparer_impression.pdfl --output impression.pdf

check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask the publisher for the open version"
}

// Boîtes de fabrication que l'éditeur n'a pas définies
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Nettoyage : ni annotations de relecture ni pièces jointes à l'impression
fix::remove_annotations()
fix::remove_attachments()
fix::flatten_layers()
fix::remove_unused_resources()
```

```pdfl
// version_email.pdfl — version légère pour approbation par e-mail
// Usage : pdfl fix final.pdf version_email.pdfl --output approbation.pdf

fix::downsample_images(96)
fix::compress_images(70)
fix::add_watermark("PROOF — NOT THE FINAL VERSION")
fix::add_page_numbers()
```

Vérifiez le résultat avec `pdfl` lui-même :

```bash
pdfl fix final.pdf version_email.pdfl --output approbation.pdf
pdfl inspect approbation.pdf       # taille, DPI et avertissements du nouveau fichier
```

---

[← `codes::`](07-codes.md) · [Sommaire](README.md) · [Suivant : `data::` →](09-data.md)
