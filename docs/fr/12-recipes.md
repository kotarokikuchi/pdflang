# 12. Recettes

[← Ligne de commande](11-cli.md) · [Sommaire](README.md) · [Suivant : changements →](13-changelog.md)

Des cas complets, réutilisables tels quels. Chacun résout un problème réel de
terrain.

---

## 12.1 Imprimerie : contrôle prépresse d'un magazine offset

**Le problème :** le client livre son fichier ; avant de graver les plaques il
faut vérifier encres, polices, images et fond perdu. Une erreur découverte plus
tard, c'est tout le tirage perdu.

`profils/offset.pdfl` :

```pdfl
profile "offset-magazine" {

  const LIMITE_TAC = 300%      // limite d'encre sur papier couché
  const FOND_PERDU = 3mm       // exigence de l'imposition
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress"] {
    // Le TAC exact lit les couleurs déclarées dans le fichier ; l'estimation
    // par rendu sous-évalue les noirs riches et laisse passer les dépassements
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= LIMITE_TAC,
        "page #{page.number}: #{tac}% ink (limit #{LIMITE_TAC}%)"
    }
  }

  check "Colors" tags: ["prepress"] {
    assert prepress::detect_color_mode() != "RGB",
      "document is in RGB — convert to CMYK"

    tons = prepress::detect_spot_colors()
    assert tons.length == 0, "unquoted special ink: #{tons.join(", ")}"

    assert !prepress::detect_rich_black(),
      "rich black detected — use 0/0/0/100 for text"
  }

  check "Fonts" tags: ["fonts"] {
    libres = prepress::detect_text_substitution()
    assert libres.length == 0,
      "fonts not embedded (text will change at the RIP): #{libres.join(", ")}"
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
    assert prepress::validate_bleed_box(), "no BleedBox — no bleed is defined"
    assert prepress::check_page_geometry(FOND_PERDU),
      "bleed smaller than 3 mm on some page"
  }
}
```

**Au comptoir :**

```bash
# Rapport HTML à renvoyer au client
pdfl run profils/offset.pdfl client.pdf --output html --output-file rapport.html
```

**En dossier surveillé :** l'opérateur dépose le fichier, le rapport apparaît à
côté.

```bash
pdfl watch inbox/ --script profils/offset.pdfl \
  --output-dir rapports/ --report html
```

---

## 12.2 Édition juridique : contrôle d'un contrat avant publication

**Le problème :** contrats et polices doivent porter les clauses obligatoires,
ne garder aucun texte de brouillon, n'exposer aucune donnée personnelle, et
rester cherchables.

`profils/juridique.pdfl` :

```pdfl
profile "standard-contract" {

  check "Mandatory clauses" tags: ["legal"] {
    // Glossaire tenu par le service juridique
    manquantes = data::validate_against_reference("termes/clauses.txt")
    assert manquantes.length == 0, "missing clauses: #{manquantes.join("; ")}"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "document marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("X{3,}"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    // Les identifiants fiscaux ne sont retenus que si leur clé est bonne :
    // les numéros d'exemple ne déclenchent pas de fausse alerte
    trouves = text::detect_personal_data()
    assert trouves.length == 0, "personal data in the document: #{trouves.join("; ")}"
  }

  check "Numbering and initials" tags: ["legal"] {
    doc.pages.each { |page|
      pied = region(0, 0, page.width, 60, "footer")
      contenu = text::extract_from_region(page.number, pied).trim()
      assert contenu != "",
        "page #{page.number} has no numbering/initials in the footer"
    }
  }

  check "Searchable text" tags: ["accessibility"] {
    assert !text::detect_rasterized_text(),
      "there are scanned pages — text cannot be searched or read by screen readers"
  }
}
```

---

## 12.3 Laboratoire pharmaceutique : notice avec code de lot

**Le problème :** la notice doit porter les mentions réglementaires, et le
code-barres doit désigner le bon produit. Intervertir deux codes entre produits
est l'erreur la plus coûteuse du secteur.

`profils/notice.pdfl` :

```pdfl
profile "regulated-insert" {

  check "Mandatory texts" tags: ["regulatory"] {
    manquants = data::validate_against_reference("bases/textes_reglementaires.txt")
    assert manquants.length == 0, "mandatory texts missing: #{manquants.join("; ")}"
  }

  check "Legibility" tags: ["regulatory"] {
    assert prepress::validate_font_size(6), "there is text below 6 pt"
  }

  check "Barcode" tags: ["codes", "critical"] {
    assert codes::detect_barcodes(), "insert has no barcode"

    code = codes::decode_barcode(1)
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"

    // Ce contrôle attrape l'erreur la plus coûteuse :
    // le code d'un produit avec le texte d'un autre
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Approved product" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    produit = data::query_gtin(code)
    assert produit, "GTIN #{code} is not in the product database"

    nom = produit.get(2)
    assert text::require_text(nom),
      "the name '#{nom}' does not appear on the insert"
    print("product verified:", nom)
  }

  check "Code position" tags: ["layout"] {
    zone = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(zone),
      "code outside the reserved area — risk of being trimmed off"
  }
}
```

```bash
PDFL_DATA_DIR=./bases pdfl run profils/notice.pdfl notice_v3.pdf
```

---

## 12.4 Approbation : comparer à la version approuvée

**Le problème :** le client a approuvé la v1. La v2 arrive avec « on n'a changé
qu'un mot ». Le croire coûte cher.

```bash
# HTML montrant ce qui a réellement changé
pdfl compare approuve/catalogue_v1.pdf recu/catalogue_v2.pdf \
  --normalize --ignore-dates \
  --output html --output-file differences.html

echo "exit: $?"   # 0 identiques · 1 métadonnées seulement · 2 contenu modifié
```

Pour vérifier aussi l'**aspect**, et pas seulement le texte :

```pdfl
// profils/fidelite.pdfl
profile "visual-fidelity" {

  const APPROUVE = "approuve/catalogue_v1.pdf"

  check "Pages visually identical" tags: ["approval"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APPROUVE)
      assert ssim > 0.99,
        "page #{page.number} changed visually (SSIM #{ssim}, #{visual::pixel_diff(page.number, APPROUVE)}% of pixels)"
    }
  }

  check "No image replaced" tags: ["approval"] {
    doc.pages.each { |page|
      assert !visual::detect_image_replacement(page.number, APPROUVE),
        "page #{page.number}: image swapped compared to the approved version"
    }
  }
}
```

Et un dossier pour le regarder, destiné à qui doit approuver le retirage :

```bash
pdfl pixelcompare approuve/catalogue_v1.pdf recu/catalogue_v2.pdf \
  --max-diff 0.05 --viewer epreuve/ --output-file pixels.json

zip -r epreuve.zip epreuve/    # un index.html et trois PNG par page, rien d'autre
```

Le dossier n'a besoin ni de serveur ni de réseau : qui ouvre `index.html` voit
l'original, le nouveau fichier et les deux ensemble avec les différences
peintes dessus, et il s'ouvre sur les pages qui diffèrent — sur un catalogue où
deux pages ont bougé sur quatre-vingt-dix, ce sont ces deux-là qui comptent.
Bouger la souris fait passer le nouveau fichier sur l'ancien ; la molette zoome
les trois volets ensemble et le glissement les déplace, de sorte qu'un filet se
tranche à 8× sans quitter la page.

---

## 12.5 CI/CD : validation par lot

**Le problème :** tout fichier entrant dans le dépôt doit passer le contrôle
prépresse, sans que personne n'ait à le lancer à la main.

`.github/workflows/preflight.yml` :

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
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # jeton automatique d'Actions, aucune configuration
        run: |
          gh release download --repo kotarokikuchi/pdflang \
            --pattern 'pdfl_*_amd64.deb'
          sudo dpkg -i pdfl_*_amd64.deb

      - name: Check the scripts themselves
        run: |
          for f in profils/*.pdfl; do
            pdfl lint "$f"
            pdfl fmt "$f" --check
          done

      - name: Preflight every PDF
        run: |
          pdfl watch fichiers/ --script profils/offset.pdfl \
            --output-dir rapports/ --once

      - name: Publish the reports
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: rapports
          path: rapports/

      # Un artefact, il faut aller l'ouvrir. Une annotation sur la pull request,
      # non.
      - name: Constats sur la pull request
        run: |
          pdfl run profils/offset.pdfl fichiers/couverture.pdf \
            --output sarif --output-file pdfl.sarif
        continue-on-error: true          # le code 2 signale un fichier refusé ; l'envoi doit avoir lieu
      - uses: github/codeql-action/upload-sarif@v3
        if: always()
        with:
          sarif_file: pdfl.sarif
```

---

## 12.6 Préparer un fichier d'éditeur pour l'imprimerie

```pdfl
// profils/preparer.pdfl
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask for the open version"
}

// Boîtes de fabrication que l'éditeur n'a pas définies
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// Nettoyage
fix::remove_annotations()      // commentaires de relecture
fix::remove_attachments()      // pièces jointes qui ne font qu'alourdir
fix::flatten_layers()          // évite qu'un calque soit rallumé par erreur
fix::remove_unused_resources()
```

```bash
pdfl fix editeur.pdf profils/preparer.pdfl --output impression.pdf --dry-run  # vérifier
pdfl fix editeur.pdf profils/preparer.pdfl --output impression.pdf            # appliquer
pdfl run profils/offset.pdfl impression.pdf                                   # valider
```

---

## 12.7 Distribuer un profil à l'équipe

**Le problème :** cinq postes doivent utiliser exactement le même profil et les
mêmes données, sans que personne n'y touche.

```bash
# Sur le poste qui maintient le profil
pdfl pack profils/ --name profil-impression --version 1.2.0

# Sur les postes de production
pdfl add profil-impression.pdflpkg
# installe dans ./pdfl_profiles/profil-impression@1.2.0/, chaque empreinte vérifiée

pdfl run pdfl_profiles/profil-impression@1.2.0/offset.pdfl fichier.pdf
```

Si le paquet a été modifié en route, `add` **refuse l'installation**.

---

## 12.8 Enquêter sur un fichier problématique

Marche à suivre quand on ne sait pas d'où vient le problème :

```bash
# 1. Vue d'ensemble en quelques secondes
pdfl inspect suspect.pdf

# 2. Script d'enquête, uniquement des print()
cat > enquete.pdfl <<'EOF'
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

pdfl run enquete.pdfl suspect.pdf > /dev/null
# print() écrit sur la sortie d'erreur : on jette le rapport
# et on ne garde que les résultats de l'enquête
```

## 12.9 Tester un profil avant qu'il ne coûte un tirage

**Problème :** un profil est du code, et quelqu'un le modifie. Un seuil bouge,
un check est renommé, et personne ne s'en aperçoit avant qu'un fichier qui
aurait dû être refusé passe à la plaque.

Gardez les fichiers qui vous ont appris la règle, et figez ce que le profil en
dit :

```
profils/imprimerie/
  prepresse.pdfl
  tests/
    approuve.pdf              # passait, et doit continuer à passer
    approuve.expected.json
    encre_324.pdf             # le fichier qui a coûté un retirage en mars
    encre_324.expected.json
    polices_non_incorporees.pdf
    polices_non_incorporees.expected.json
```

```bash
# Une fois, quand les cas sont ceux que vous voulez
pdfl test profils/imprimerie/prepresse.pdfl --update

# Ensuite — en CI, et avant chaque commit sur le profil
pdfl test profils/imprimerie/prepresse.pdfl --jobs 0
```

Un fichier refusé fait un cas aussi bon qu'un fichier approuvé : ce qui est
enregistré, c'est le rapport entier. Le test échoue donc aussi fort si le profil
cesse de signaler les 324% d'encre que s'il se met à signaler un fichier
irréprochable.

```yaml
      - name: Les profils trouvent toujours ce qu'ils trouvaient
        run: pdfl test profils/imprimerie/prepresse.pdfl --jobs 0
```

Lisez l'échec avant de réenregistrer. `--update` est le moment où vous décidez
que le nouveau comportement est le bon — il n'y en a pas d'autre.

---

---

[← Ligne de commande](11-cli.md) · [Sommaire](README.md) · [Suivant : changements →](13-changelog.md)
