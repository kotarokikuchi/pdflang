# Documentation PDFLang — Français

Guide complet du langage `.pdfl` et de l'outil en ligne de commande `pdfl` —
version 0.10.1.

Chaque exemple de cette documentation est du code exécutable et commenté. Si
vous découvrez le langage, commencez par le manuel du chapitre 1 ; les autres
chapitres se consultent comme une référence.

> **La langue de l'outil.** Les messages de `pdfl` (diagnostics, erreurs, aide
> en ligne de commande, libellés des rapports) sont en **anglais**, comme le
> veut l'usage pour les outils en ligne de commande. Cette documentation est en
> français, mais un contrôle qui échoue affichera quelque chose comme
> `page 7: 324% ink (limit 300%)`. Les messages que vous écrivez **vous-même**
> dans vos scripts sortent tels quels, dans la langue que vous avez employée.

## Sommaire

| Chapitre | Contenu |
|---|---|
| [1. Le langage](01-language.md) | Manuel complet : checks, assertions, types, unités, blocs, fonctions, import, rule |
| [2. Types du document](02-types.md) | `doc`, `page`, `font`, `image`, `region` — toutes les propriétés et méthodes |
| [3. `text::`](03-text.md) | Texte : extraction, normalisation, recherche, validations brésiliennes, données personnelles |
| [4. `struct::`](04-struct.md) | Structure et métadonnées : objets, XMP, sécurité, empreintes |
| [5. `visual::`](05-visual.md) | Images : résolution, comparaison visuelle, pHash, SSIM, qualité |
| [6. `prepress::`](06-prepress.md) | Prépresse : encrage total, séparations, tons directs, polices, boîtes |
| [7. `codes::`](07-codes.md) | Codes-barres et QR codes : détection, décodage, vérification |
| [8. `fix::`](08-fix.md) | Normalisation : boîtes, pages, filigranes, fusion/découpe, optimisation |
| [9. `data::`](09-data.md) | Données externes : glossaires, jeux de données, tables de consultation |
| [10. Bibliothèque standard](10-stdlib.md) | Méthodes de listes et de chaînes, fonctions globales |
| [11. Ligne de commande](11-cli.md) | `run`, `compare`, `watch`, `fix`, `inspect`, `lint`, `fmt`, `doc`, `pack`, `add` |
| [12. Recettes](12-recipes.md) | Cas complets : imprimerie, édition juridique, laboratoire, CI/CD |

## Démarrer en 30 secondes

Créez `mon_profil.pdfl` :

```pdfl
// Chaque script est un ensemble de checks. Un check regroupe des
// validations qui vont ensemble et devient une section du rapport.
check "Basic structure" {
  // require : le message est produit automatiquement à partir de l'expression
  require doc.page_count > 0

  // assert : avec le message que vous écrivez vous-même
  assert doc.title != "", "PDF has no title in its metadata"
}
```

Exécutez :

```bash
pdfl run mon_profil.pdfl document.pdf
```

Le rapport sort en JSON sur la sortie standard. Le code de sortie dit le
résultat : `0` tout est passé, `1` seulement des avertissements, `2` erreurs de
validation, `3` erreur de syntaxe.

## Conventions de cette documentation

- Chaque fonction indique sa **signature**, ce qu'elle **fait**, ce qu'elle
  **retourne** et un **exemple commenté**.
- Les arguments entre crochets sont facultatifs : `calculate_tac([page])`.
- « à partir de 1 » signifie que la première page est `1`, pas `0` — le langage
  compte comme comptent les gens, pas comme comptent les programmeurs.
- Les dimensions sont toujours en **points** (1 pt = 1/72 pouce). Les littéraux
  d'unité (`3mm`, `1in`) font la conversion pour vous.

---

Autres langues : [English](../en/) · [Português (Brasil)](../pt-br/) ·
[日本語](../ja/) · [中文](../zh/) · [العربية](../ar/) · [Deutsch](../de/)
