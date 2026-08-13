# 11. Ligne de commande

[← Bibliothèque standard](10-stdlib.md) · [Sommaire](README.md) · [Suivant : recettes →](12-recipes.md)

Dix commandes : quatre pour les PDF, quatre pour les scripts et deux pour la
distribution.

| Commande | Rôle |
|---|---|
| [`run`](#pdfl-run) | Valide un PDF avec un script |
| [`compare`](#pdfl-compare) | Compare deux versions |
| [`watch`](#pdfl-watch) | Surveille un dossier et valide ce qui arrive |
| [`fix`](#pdfl-fix) | Applique des modifications et enregistre un nouveau PDF |
| [`inspect`](#pdfl-inspect) | Vue d'ensemble rapide d'un PDF |
| [`lint`](#pdfl-lint) | Analyse un script sans l'exécuter |
| [`fmt`](#pdfl-fmt) | Met en forme un script |
| [`doc`](#pdfl-doc) | Génère la documentation d'un script |
| [`pack`](#pdfl-pack) | Empaquette profils et données |
| [`add`](#pdfl-add) | Installe un paquet |

---

## Codes de sortie

Communs à toutes les commandes qui valident.

| Code | Signification |
|---|---|
| `0` | Tout est passé |
| `1` | Avertissements seulement |
| `2` | Erreurs de validation |
| `3` | Erreur de syntaxe dans le script |
| `10` | Document illisible, ou fichier non écrit — aucun verdict n'a été rendu |

```bash
pdfl run profil.pdfl fichier.pdf > rapport.json
case $? in
  0) echo "approved" ;;
  1) echo "approved with warnings" ;;
  2) echo "rejected — see rapport.json" ;;
  3) echo "error in the validation script" ;;
esac
```

---

## `pdfl run`

Valide un PDF avec un script.

```bash
pdfl run <script.pdfl> <entree.pdf> [options]
```

| Option | Défaut | Rôle |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format du rapport |
| `--output-file <fichier>` | — | Écrit dans un fichier au lieu de la sortie standard |
| `--fail-on error\|warning` | `error` | Avec `warning`, un avertissement donne aussi le code 2 |
| `--verbose` | — | Informations supplémentaires sur la sortie d'erreur |
| `--var NOM=VALEUR` | — | Valeur que le script lit comme `vars.NOM` ; répétable |
| `--tags TAG` | — | N'exécute que les checks portant ce tag ; répétable. Un tag qu'aucun check ne porte est une erreur, pas une réussite vide |

```bash
pdfl run prepresse.pdfl magazine.pdf                                     # JSON au terminal
pdfl run prepresse.pdfl magazine.pdf --output html --output-file rapport.html
pdfl run prepresse.pdfl magazine.pdf --output pdf --output-file rapport.pdf
pdfl run prepresse.pdfl magazine.pdf --output csv --output-file constats.csv
pdfl run prepresse.pdfl magazine.pdf --fail-on warning                   # mode strict
```

### Le rapport JSON

```json
{
  "schema_version": 1,
  "script_name": "prepress.pdfl",
  "input_file": "magazine.pdf",
  "profile": "offset-magazine",
  "status": "FAIL",
  "total_pages_analyzed": 120,
  "error_count": 2,
  "warning_count": 0,
  "info_count": 0,
  "diagnostics": [
    {
      "id": "PDFL-093751a2",
      "severity": "error",
      "check_name": "Ink coverage",
      "message": "page 7: 324% ink (limit 300%)",
      "line": 12
    }
  ],
  "checks_run": ["Ink coverage", "Fonts", "Bleed"]
}
```

Le même PDF avec le même script produit toujours un **rapport identique octet
pour octet** : on peut le versionner et comparer les différences en CI.

`schema_version` est la première clé, pour qu'un consommateur puisse trancher
avant d'analyser le reste. Elle n'augmente que si un lecteur de la sortie
précédente cassait ; ajouter un champ ne l'augmente pas.

### SARIF et JUnit

Deux formats de plus, pour que le résultat apparaisse là où l'équipe regarde
déjà, et non dans un log que personne n'ouvre.

```bash
# GitHub code scanning : les constats deviennent des annotations sur la pull request
pdfl run prepresse.pdfl magazine.pdf --output sarif --output-file pdfl.sarif

# Panneau de tests de n'importe quelle CI : un test par check, réussis compris
pdfl run prepresse.pdfl magazine.pdf --output junit --output-file pdfl.xml
```

En SARIF, un constat est ancré sur le **script**, pas sur le PDF : la ligne que
l'on connaît est celle du check, et le PDF est le plus souvent un artefact de
passage dans la CI plutôt qu'un fichier du dépôt — pointer là annoterait un
chemin qui n'existe pas. Le fichier validé voyage dans `properties.inputFile`,
et l'identifiant du diagnostic dans `partialFingerprints` : c'est ce qui permet
à GitHub de reconnaître un constat déjà vu au lieu de le rouvrir à chaque
exécution.

En JUnit, chaque check exécuté est un cas de test, y compris ceux qui n'ont rien
trouvé. Un format ne listant que les échecs annoncerait une exécution propre
comme zéro test, et une CI lit cela comme une exécution qui n'a jamais eu lieu.
Un constat `info` ne fait pas échouer son cas ; il part dans `<system-out>`.

```yaml
- name: Prépresse
  run: pdfl run prepresse.pdfl magazine.pdf --output sarif --output-file pdfl.sarif
  # le code 2 signale un fichier refusé, et l'envoi doit quand même avoir lieu
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: pdfl.sarif
```

---

## `pdfl compare`

Compare deux versions : texte, structure et métadonnées.

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| Option | Défaut | Rôle |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format |
| `--output-file <fichier>` | — | Écrit dans un fichier |
| `--normalize` | — | Ignore casse et espaces |
| `--ignore-dates` | — | Masque les dates avant de comparer |
| `--similarity-threshold <0-100>` | `100` | Similarité minimale acceptable |

```bash
pdfl compare approuve_v1.pdf recu_v2.pdf --normalize --ignore-dates

# Tolère jusqu'à 1 % d'écart ; en dessous, c'est une erreur
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file differences.html
```

### Comment ça marche

- Les pages sont mises en correspondance **par leur contenu**, pas par leur
  numéro : une page insérée au milieu ne fait pas signaler tout ce qui suit.
  Fonctionne sur des documents de plus de mille pages.
- Chaque paire reçoit un score de similarité et un échantillon des lignes qui
  changent (`-` retirée, `+` ajoutée).
- Un changement de métadonnées est un **avertissement** ; un changement de texte
  sous le seuil est une **erreur**, au-dessus un **avertissement**.
- Le score global figure dans le champ `similarity` du rapport.

```
page 4 → 4: similarity 97.8% | -original title | +revised title
```

---

## `pdfl watch`

Surveille un dossier et valide chaque PDF qui arrive ou change.

```bash
pdfl watch <dossier> --script <script.pdfl> [options]
```

| Option | Défaut | Rôle |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | Quels fichiers traiter |
| `--exclude <glob>` | — | Quels fichiers ignorer |
| `--output-dir <dossier>` | à côté du PDF | Où écrire les rapports |
| `--depth <n>` | `1` | Profondeur des sous-dossiers |
| `--debounce <ms>` | `1000` | Attente que le fichier se stabilise |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format des rapports |
| `--fail-fast` | — | S'arrête à la première erreur |
| `--once` | — | Traite l'existant puis quitte |

```bash
# Dossier de réception d'une imprimerie, en continu
pdfl watch inbox/ --script preflight.pdfl --output-dir rapports/ --report html

# Traitement par lot pour la CI : sort avec le pire code rencontré
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

Le **debounce** existe parce qu'un gros fichier arrive par morceaux : on ne
traite qu'un fichier qui a cessé de changer, donc jamais un PDF à moitié écrit.

Les rapports s'écrivent en `<nom>.report.json` (ou `.csv`, `.html`, `.pdf`).

---

## `pdfl fix`

Applique les opérations `fix::` et enregistre un nouveau PDF. Détails au
[chapitre 8](08-fix.md).

```bash
pdfl fix original.pdf normaliser.pdfl --output out.pdf --dry-run  # voir seulement
pdfl fix original.pdf normaliser.pdfl --output corrige.pdf        # appliquer
```

---

## `pdfl inspect`

Vue d'ensemble d'un PDF, sans script.

```bash
pdfl inspect <fichier.pdf>
```

`--json` renvoie le même résumé sous forme de données.

```
File:     magazine.pdf
Size:     26 KB (27284713 bytes)
SHA-256:  af1029842e5bfeae338ead82fb449ef851be742b1d63117c12596e3ea123a616

Pages:    120
Page size: 496 x 709 pt
Boxes:    MediaBox, TrimBox, BleedBox

Metadata:
  Title: Example Magazine
  Creator: Adobe InDesign 19.3

Fonts:    26
  ABCDEF+Helvetica — embedded
  Arial — NOT embedded
Images:   81 (minimum DPI 136, spaces: DeviceCMYK, Indexed)
Max. estimated TAC: 300% (RGB render approximation)

Warnings:
  ! there are non-embedded fonts
  ! 3 image(s) below 300 DPI
```

La première commande à lancer quand un fichier arrive : en quelques secondes on
sait s'il vaut la peine d'être ouvert.

---

## `pdfl lint`

Analyse un script sans l'exécuter et signale les problèmes de qualité.

```bash
pdfl lint <script.pdfl>
```

`--json` renvoie les mêmes avertissements sous forme de données.

Ce qu'il détecte :

- Variables, paramètres de bloc et fonctions déclarés et **jamais utilisés**
  (préfixez par `_` pour taire l'avertissement : `_page`)
- Checks **en double** ou **vides**
- Espaces de noms inconnus (`text::`, `struct::`, `visual::`, `prepress::`,
  `codes::`, `fix::`, `data::`)
- `assert` / `require` hors d'un check
- Usage de `fix::` (qui ne tourne que sous `pdfl fix`)

```bash
$ pdfl lint profil.pdfl
profil.pdfl: warning: variable 'LIMIT' declared and never used
profil.pdfl: warning: check "Fonts" declared 2 times
```

En présence d'avertissements, le code de sortie est `1` — utilisable en CI.

---

## `pdfl fmt`

Met en forme un script : indentation de deux espaces, espacement cohérent,
lignes vides compactées. Commentaires et unités (`3mm` reste `3mm`) sont
conservés.

```bash
pdfl fmt <script.pdfl>            # met en forme sur place
pdfl fmt <script.pdfl> --check    # ne modifie rien ; code 1 si non formaté
```

```bash
# Imposer la norme de l'équipe en CI
for f in profils/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

Génère la documentation à partir du script lui-même.

```bash
pdfl doc <script.pdfl> [--output markdown|html|json]
```

Il produit : le profil, un tableau des constantes, les fonctions, les imports,
et pour chaque check ses étiquettes et ce qu'il valide (les messages des
`assert` deviennent les descriptions).

```bash
pdfl doc prepresse.pdfl > docs/profil-prepresse.md
pdfl doc prepresse.pdfl --output html > profil.html
```

C'est le livrable qui explique ce que valide un profil à un responsable de
fabrication qui ne lit pas le code.

---

## `pdfl pack`

Empaquette scripts et données dans un `.pdflpkg` distribuable.

```bash
pdfl pack <dossier> [--name <nom>] [--version <version>] [--output <fichier>]
```

Il collecte récursivement les `.pdfl`, `.csv`, `.txt`, `.json` et `.xlsx` du
dossier et ajoute un `manifest.json` qui note le SHA-256 de chaque fichier.
L'empaquetage est déterministe : le même dossier produit les mêmes octets.

```bash
pdfl pack profils/imprimerie --name profil-impression --version 1.0.0
```

---

## `pdfl add`

Installe un paquet local en vérifiant les empreintes du manifeste.

```bash
pdfl add profil-impression.pdflpkg
# installe dans ./pdfl_profiles/profil-impression@1.0.0/

pdfl run pdfl_profiles/profil-impression@1.0.0/prepresse.pdfl fichier.pdf
```

Si l'empreinte d'un fichier ne correspond pas, l'installation est **refusée** —
un paquet corrompu ou altéré n'entre pas.

> Dépôts distants et signatures numériques ne font pas partie de cette version :
> `add` installe depuis un fichier local.

---

[← Bibliothèque standard](10-stdlib.md) · [Sommaire](README.md) · [Suivant : recettes →](12-recipes.md)
