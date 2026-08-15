# 11. Ligne de commande

[← Bibliothèque standard](10-stdlib.md) · [Sommaire](README.md) · [Suivant : recettes →](12-recipes.md)

Treize commandes : six pour les PDF, quatre pour les scripts, deux pour la
distribution et une pour le shell.

| Commande | Rôle |
|---|---|
| [`run`](#pdfl-run) | Valide un PDF avec un script |
| [`compare`](#pdfl-compare) | Compare deux versions |
| [`pixelcompare`](#pdfl-pixelcompare) | Compare deux PDF pixel par pixel, avec une visionneuse pour voir le changement |
| [`watch`](#pdfl-watch) | Surveille un dossier et valide ce qui arrive |
| [`fix`](#pdfl-fix) | Applique des modifications et enregistre un nouveau PDF |
| [`inspect`](#pdfl-inspect) | Vue d'ensemble rapide d'un PDF |
| [`lint`](#pdfl-lint) | Analyse un script sans l'exécuter |
| [`fmt`](#pdfl-fmt) | Met en forme un script |
| [`test`](#pdfl-test) | Exécute un script sur un dossier de PDF et compare chaque rapport |
| [`doc`](#pdfl-doc) | Génère la documentation d'un script |
| [`pack`](#pdfl-pack) | Empaquette profils et données |
| [`add`](#pdfl-add) | Installe un paquet |
| [`completions`](#pdfl-completions) | Imprime un script de complétion pour votre shell |

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

## Options globales

| Option | Rôle |
|---|---|
| `--quiet` | Fait taire la progression et les confirmations sur stderr |

`--quiet` fonctionne avant comme après la sous-commande, et sur chacune d'elles.
Il retire les lignes qu'une personne veut et qu'une chaîne d'intégration ne veut
pas — `report saved to …`, `watching …`, le résultat par fichier de `watch`. Il
ne retire **pas** les erreurs : une exécution silencieuse qui échoue dit toujours
pourquoi.

Il ne fait pas taire `print()` non plus. C'est la sortie du script lui-même, et
l'avaler changerait ce que le script fait. Redirigez stderr si vous voulez vous
en débarrasser.

`--quiet` l'emporte sur `--verbose`.

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

## `pdfl pixelcompare`

Compare deux PDF sur ce à quoi ils *ressemblent*, page par page.

```bash
pdfl pixelcompare <original.pdf> <nouveau.pdf> [options]
```

| Option | Défaut | Rôle |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format du rapport |
| `--output-file <fichier>` | — | Écrit le rapport dans un fichier |
| `--viewer <dossier>` | — | Écrit une visionneuse autonome : les pages, les différences et un `index.html` pour les regarder |
| `--dpi <n>` | `150` | Résolution de rendu. Plus haute voit plus et coûte plus |
| `--threshold <0.0-1.0>` | `0.05` | Distance de couleur à partir de laquelle deux pixels diffèrent |
| `--max-diff <pourcent>` | `0.0` | Ce qu'une page peut changer avant d'être signalée |
| `--pages <plage>` | toutes | `1-10` ou `1,3,7-12` |
| `--no-align` | — | Ne compense pas un décalage global entre les pages |
| `--blur <rayon>` | `0` | Flou avant comparaison, pour absorber l'anticrénelage |
| `--jobs <n>` | un par CPU | Pages comparées en même temps |

`pdfl compare` répond à « le texte ou la structure ont-ils changé ». Ceci répond
à une autre question — « est-ce que ça se ressemble toujours » — et les deux se
contredisent plus souvent qu'on ne croit. Un logo décalé de 2mm, un filet
disparu, un ton direct remplacé par sa composition CMJN : dans les trois cas le
texte est identique.

```bash
# Tout le document, en JSON
pdfl pixelcompare approuve.pdf retirage.pdf

# Avec un endroit où regarder vraiment les différences
pdfl pixelcompare approuve.pdf retirage.pdf --viewer diff/

# Tolérer un peu de bruit, puis regarder de plus près ce qui reste
pdfl pixelcompare approuve.pdf retirage.pdf --max-diff 0.1 --dpi 300
```

Un constat par page modifiée, avec la part de pixels et le nombre de zones
distinctes :

```
page 7: 0.51% of the pixels differ, in 29 area(s)
```

Une page présente dans un fichier et absente de l'autre est un constat à part :
il n'y a rien à quoi la comparer. Le `similarity` du rapport est la moyenne sur
les pages comparées, une page refaite sur deux cents ne fait donc pas un autre
document ; les chiffres par page sont dans les diagnostics.

### L'alignement, et pourquoi il est actif

Un fichier réexporté depuis la même source tombe souvent à un ou deux pixels
près. Sans compensation, chaque bord de glyphe de la page devient « différent »
et le seul changement qui compte s'y noie. `pixelcompare` cherche un décalage
global unique — d'abord grossièrement sur une copie réduite, puis affiné — et
le signale quand il en trouve un :

```
page 3: 2.10% of the pixels differ, in 44 area(s) (aligned by 2, -1 px)
```

Désactivez-le avec `--no-align` quand c'est justement la position qui est
vérifiée.

### La visionneuse

`--viewer diff/` écrit un dossier contenant trois PNG par page et un
`index.html`. Sans dépendance d'aucune sorte — pas de CDN, pas de bundler, pas
de serveur. Ouvrez le fichier, ou zippez le dossier et envoyez-le à qui doit
approuver le retirage.

Trois volets côte à côte, toujours sur la même page :

| Volet | Ce qu'il montre |
|---|---|
| **Original** | la page du premier fichier, telle quelle |
| **New** | la page du second fichier, telle quelle |
| **Difference** | les deux, avec ce qui a changé peint dessus — glissez pour balayer |

Les trois volets portent la même paire de barres — une verticale, une
horizontale — au même endroit, et elles bougent dans les trois à la fois. La
verticale se glisse ; l'horizontale suit le pointeur, pressé ou non. Leur
croisement est le coin de ce qui est révélé, et la pastille se place sur la
verticale à cette hauteur : elle marque donc l'endroit que tient le pointeur.

Dans le volet **Difference**, les barres tranchent : le nouveau fichier apparaît
à droite de la verticale et sous l'horizontale, l'original partout ailleurs.
Intacte, l'horizontale reste en haut, ce qui fait de la verticale un balayage
pleine hauteur ordinaire — descendez-la quand le changement cherché tient dans
une bande plutôt que dans une colonne. Dans les deux autres volets, ce sont des
règles sur la même colonne et la même ligne de la page.

Les deux positions sont des pourcentages de la page et non d'un volet : elles
survivent au changement de page et au redimensionnement de la fenêtre.

La molette zoome, jusqu'à 8×, et les trois volets zooment ensemble autour du
point sous le pointeur : ce que vous regardiez reste où il était. Le
dézoomage s'arrête à la page ajustée — en dessous il n'y a rien d'utile, le
volet contient déjà la page entière. Les barres gardent leur épaisseur à tout
niveau de zoom. **Reset view** remet le zoom sur la page entière et les barres
à leur place de départ ; le bouton reste désactivé tant qu'il n'y a rien à
défaire.

Les différences sont peintes sur place, et la couleur dit laquelle :

| Couleur | Signification |
|---|---|
| Rouge | Encre disparue du nouveau fichier |
| Vert | Encre nouvelle dans celui-ci |
| Bleu | Même graisse, autre couleur |

Les trois volets sont dimensionnés d'après la fenêtre : toute la comparaison
tient à l'écran sans défilement, et ils gardent les proportions de la page quel
que soit le format de la fenêtre. Là où les deux fichiers ne s'accordent pas
sur la taille d'une page — l'une passée en paysage —, chacune est montrée
entière dans le cadre commun plutôt qu'étirée pour le remplir.

**Elle s'ouvre sur les pages qui diffèrent.** Sur un document de deux cents
pages dont trois ont bougé, ces trois-là sont la raison de l'ouvrir ; **All**
remet les autres. Les flèches et `←` `→` suivent le filtre et enjambent ce que
la bande masque. Quand rien ne diffère, le bouton le dit et reste désactivé au
lieu de réduire la bande à rien.

### Progression

Rastériser un long document à 300 dpi prend des minutes : chaque étape trace donc
une barre sur stderr — une par fichier rastérisé, une pour la comparaison, une
pour l'écriture de la visionneuse.

```
rasterising approuve.pdf  [############------------]  98/207
```

Elle n'est tracée que si stderr est un terminal. La barre revient en début de
ligne et la réécrit ; un fichier de log n'a pas de curseur à déplacer, une
exécution redirigée accumulerait donc des milliers de fragments. Redirigée, elle
reste muette et les messages ordinaires passent toujours. `--quiet` la fait taire
partout.

### Vitesse

La comparaison utilise tous les CPU par défaut. Sur 41 pages à 150 dpi :

| `--jobs` | Temps |
|---|---|
| `1` | 3,6s |
| `4` | 1,7s |
| `8` | 1,2s |
| `20` | 1,3s |

Cela cesse de progresser vers huit, car cette étape est limitée par la bande
passante mémoire et non par le calcul — elle fait défiler des pages entières
dans le CPU. Au-delà, les threads font la queue devant la même mémoire. En
demander plus n'est pas nuisible, seulement inutile.

Notez ce qui n'est **pas** parallèle : la rastérisation. pdfium sérialise
chaque appel derrière un unique verrou global ; un second thread devant lui ne
fait qu'attendre. Cela pose un plancher — environ 0,8s des chiffres ci-dessus —
et c'est pourquoi `--jobs 8` va trois fois plus vite, et non huit.

Ici la valeur par défaut est un par CPU, alors que `pdfl test` et `pdfl watch`
utilisent `--jobs 1`. La différence est réelle : là-bas une tâche est un
processus enfant qui tient son propre document, donc un document de plus en
mémoire à chaque fois. Ici les pages sont déjà en mémoire et les threads se les
partagent : une tâche coûte l'espace de travail d'une page. Baissez-la si vous
partagez la machine.

Codes de sortie : `0` aucune page n'a changé de plus que `--max-diff`, `2` au
moins une, `10` un fichier illisible ou une visionneuse non écrite.

Le rapport ne dépend pas de `--jobs`. Les pages sont réassemblées dans l'ordre
des pages : les diagnostics, leur ordre et leurs empreintes sortent identiques
quelle que soit la valeur — un test le garantit — et les fichiers de la
visionneuse sortent octet pour octet identiques.

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
| `--events` | — | Se réveille sur les notifications du système plutôt que sur une minuterie — pas sur un partage réseau |
| `--journal <fichier>` | — | Journal en ajout seul de ce qui a été validé ; relancer saute ce qu'il couvre |
| `--timeout <s>` | — | Tue l'analyse d'un fichier après ce nombre de secondes et le signale comme refusé |
| `--var NOM=VALEUR` | — | Valeur que chaque fichier lit comme `vars.NOM` ; répétable |
| `--jobs <n>` | `1` | Fichiers validés en même temps ; `0` = un par CPU |
| `--once` | — | Traite l'existant puis quitte |

```bash
# Dossier de réception d'une imprimerie, en continu
pdfl watch inbox/ --script preflight.pdfl --output-dir rapports/ --report html

# Traitement par lot pour la CI : sort avec le pire code rencontré
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

`--jobs` s'applique à tout ce qu'une passe doit traiter, en lot comme lors d'une
rafale d'arrivées. Chaque fichier est validé par son propre processus `pdfl` — la
même raison que pour `pdfl test` — et c'est ce processus-ci qui rend les
rapports : le fichier écrit est donc identique quel que soit `--jobs`. Sur huit
fichiers de 41 pages : 9,5s avec `--jobs 1`, 1,2s avec `--jobs 0`.

Avec `--fail-fast`, aucun nouveau fichier n'est lancé dès qu'un échec est
constaté ; ceux déjà en cours vont au bout, car les tuer laisserait des rapports
à moitié écrits. Les rapports sont écrits dans l'ordre où les fichiers ont été
trouvés : un lot imprime les mêmes lignes quel qu'ait été le parallélisme.

L'attente se termine exactement quand le fichier le plus récent a fini
d'arriver : un fichier qui arrive pendant une attente n'est donc pas retenu un
intervalle complet de plus.

Par défaut, le dossier est listé sur une minuterie ; avec `--events`, watch
attend les notifications du système. Le défaut est la minuterie, et c'est
mesuré : lister 10 000 fichiers toutes les 200ms ne coûte pas de CPU mesurable,
et le temps de stabilisation domine la latence de toute façon — sur un dossier
local, les deux modes terminent à un centième de seconde près.

N'utilisez pas `--events` sur un partage réseau. Sur un montage NFS ou SMB,
inotify ne signale que ce qu'écrit la machine locale : les fichiers venus
d'ailleurs ne seraient jamais vus, et watch n'en dirait rien. Là où l'option
paie, c'est sur une machine qui surveille beaucoup de dossiers, ou dont le
listage est coûteux. Si le surveillant ne peut pas être créé, watch le dit et
revient à la minuterie plutôt que de se taire.

Le **debounce** existe parce qu'un gros fichier arrive par morceaux : on ne
traite qu'un fichier qui a cessé de changer, donc jamais un PDF à moitié écrit.

### Le journal : finir un lot interrompu

Cinq mille fichiers, et la machine redémarre au quatre millième. Sans trace, la
prochaine exécution repart du premier.

```bash
pdfl watch entree/ --script offset.pdfl --once --journal lot.jsonl
```

Un objet JSON par fichier, ajouté au fur et à mesure :

```json
{"input":"entree/couverture.pdf","sha256":"9f2b…","status":"FAIL","errors":2,"warnings":0,"exit":2}
```

Relancez avec le même journal : les fichiers qu'il couvre sont sautés. Pas leurs
verdicts — un lot repris qui saute un fichier refusé sort toujours en `2`, car le
journal est la trace du lot et le code de sortie en est le verdict. Un lot
annonçant « propre » parce qu'il avait déjà vu l'échec serait le pire bug que cet
outil puisse avoir.

Un fichier est reconnu **à ses octets**, ni à son nom ni à sa date. Remplacez
`couverture.pdf` par un autre `couverture.pdf` et il est revalidé : son empreinte
n'est pas celle enregistrée.

Rien n'est écrit sans `--journal`. L'outil ne garde aucun état ; ceci est un
fichier que vous avez demandé par son nom, exactement comme un rapport. Et il n'y
a pas d'horodatage dans une ligne : le journal dit *si* un fichier a été validé
et ce qu'il en est sorti, le rapport à côté dit *quoi*, et le système de fichiers
dit *quand* — ce qui garde une réexécution identique octet pour octet à la
première, comme tout le reste ici.

Les lignes sont écrites une à une : ce qu'un plantage laisse est donc vrai
jusque-là. Un journal illisible arrête l'exécution en nommant la ligne — sauter
des fichiers sur un enregistrement mal lu serait pire que de tout reprendre.

### `--timeout` : un mauvais fichier ne doit pas bloquer le lot

```bash
pdfl watch entree/ --script offset.pdfl --once --timeout 60
```

Un fichier dont l'analyse dépasse `60` secondes est tué et signalé de la même
façon qu'un PDF illisible — un rapport avec un constat, `check_name: "timeout"` —
il s'imprime donc, s'écrit sur disque et entre dans le journal exactement comme
n'importe quel autre verdict. Rien n'est sauté en silence, et le lot passe au
fichier suivant plutôt que de rester bloqué sur celui-là.

```json
{"input":"entree/adversarial.pdf","sha256":"7a1c…","status":"FAIL","errors":1,"warnings":0,"exit":2}
```

Rien dans le langage `.pdfl` ne permet à un script de bloquer l'interpréteur
exprès — la récursion est limitée en profondeur — `--timeout` existe donc pour ce
qu'un script ne peut pas causer : pdfium qui boucle ou se bloque sur un PDF
malformé ou hostile. Sans le drapeau, l'analyse d'un fichier attend aussi
longtemps que nécessaire, le seul comportement avant l'existence de cette option.

`--var` atteint chaque fichier sans changer — une valeur pour toute l'exécution,
utile pour quelque chose de constant sur un dossier (un nom de client) plutôt
que variable par fichier (un numéro de commande). Sans lui, un script lisant
`vars.*` ne pourrait jamais être surveillé : chaque fichier échouerait avec
« was not provided ».

Les rapports s'écrivent en `<nom>.report.json` (ou `.csv`, `.html`, `.pdf`).

---

## `pdfl fix`

Applique les opérations `fix::` et enregistre un nouveau PDF. Détails au
[chapitre 8](08-fix.md).

```bash
pdfl fix <entree.pdf> <script.pdfl> --output <sortie.pdf> [options]
```

| Option | Défaut | Rôle |
|---|---|---|
| `--output <fichier>` | — | PDF de sortie (obligatoire) |
| `--dry-run` | — | Liste les opérations sans enregistrer |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | Format du rapport |
| `--report-file <fichier>` | — | Écrit le rapport dans un fichier |

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

Il collecte récursivement les `.pdfl`, `.csv`, `.txt` et `.json` du dossier et
ajoute un `manifest.json` qui note le SHA-256 de chaque fichier. L'empaquetage
est déterministe : le même dossier produit les mêmes octets.

Un tableur (`.xlsx`, `.xls`, `.ods`) n'est **pas** empaqueté, et `pack` dit quel
fichier il a laissé. Aucune fonction `data::` ne sait en ouvrir un : l'inclure
livrerait un paquet qui s'installe proprement et échoue à la première
consultation.

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

## `pdfl test`

Exécute un script sur chaque PDF d'un dossier et compare chaque rapport à celui
enregistré à côté. Un profil qui se met à trouver autre chose fait échouer un
test au lieu de surprendre quelqu'un plus loin.

```bash
pdfl test <script.pdfl> [--dir <dossier>] [--update]
```

| Option | Défaut | Rôle |
|---|---|---|
| `--dir <dossier>` | `tests/` à côté du script | Où se trouvent les PDF des cas |
| `--update` | — | Enregistre les rapports attendus au lieu de comparer |
| `--jobs <n>` | `1` | Cas exécutés en même temps ; `0` = un par CPU |
| `--var NOM=VALEUR` | — | Valeur que chaque cas lit comme `vars.NOM` ; répétable |

Un cas, c'est un PDF et le rapport qu'on en attend, côte à côte :

```
profils/imprimerie/
  prepresse.pdfl
  tests/
    approuve.pdf
    approuve.expected.json
    encre_lourde.pdf
    encre_lourde.expected.json
```

```bash
# La première fois : enregistrer ce que le script trouve aujourd'hui
pdfl test prepresse.pdfl --update

# Ensuite
pdfl test prepresse.pdfl
```

```
ok   approuve.pdf
FAIL encre_lourde.pdf
     error_count: expected 1, got 0
     missing:    PDFL-093751a2 [error] Taux d'encrage (line 12) : page 7 : 324% d'encre (limite 300%)
1 passed, 1 failed
```

L'échec nomme ce qui a changé — les compteurs, le verdict, et quels constats
sont apparus ou ont disparu — plutôt que d'imprimer deux fichiers JSON côte à
côte.

Enregistrer est toujours un geste délibéré : une exécution qui rafraîchirait sa
propre référence ne pourrait jamais échouer. Lisez d'abord la différence, puis
réenregistrez avec `--update` quand le changement est celui que vous vouliez.

Le rapport attendu est celui de `pdfl run`, avec `input_file` réduit au nom du
fichier : une référence qui changerait selon le répertoire d'appel n'en serait
pas une. Un PDF illisible fait échouer son propre cas et laisse les autres
s'exécuter.

Codes de sortie : `0` tous passés, `2` au moins un échec, `10` dossier illisible
ou sans PDF.

### Exécuter les cas en même temps

Chaque cas s'exécute dans son propre processus `pdfl` : `--jobs` transforme donc
une suite en vrai travail parallèle. Sur huit fichiers de 41 pages, `--jobs 1` a
pris 8,9s et `--jobs 8` 1,1s. Des threads dans un seul processus n'y seraient pas
parvenus — pdfium sérialise chaque appel derrière un unique mutex, et la version
threadée s'est mesurée *plus lente* que la séquentielle.

La valeur par défaut est `1`, car chaque tâche est un processus qui tient un
document en mémoire, et cet outil existe pour des fichiers qui peuvent être
énormes. Augmentez-la quand les cas sont ordinaires : `--jobs 0` en donne un par
CPU.

L'ordre de la sortie ne change jamais avec `--jobs` : les cas sont jugés dans
l'ordre où ils ont été trouvés, quel que soit l'enfant terminé le premier.

Un cas dont le PDF est illisible est jugé comme les autres — son rapport porte la
raison sous forme de constat, donc « ce fichier doit être refusé comme illisible »
peut être un test à part entière. Ce rapport nomme le fichier tel qu'il a été
passé : enregistrez les références avec un `--dir` **relatif** si elles doivent
être versionnées.

`--var` atteint chaque cas sans changer — une valeur pour toute l'exécution, pas
une par fichier. Sans lui, un script lisant `vars.*` ne pourrait jamais être
testé : chaque cas échouerait avec « was not provided », quel que soit le PDF.

---

## `pdfl completions`

Imprime sur stdout un script de complétion pour votre shell.

```bash
pdfl completions <bash|zsh|fish|elvish|powershell>
```

```bash
# bash, pour l'utilisateur courant
pdfl completions bash > ~/.local/share/bash-completion/completions/pdfl

# zsh — n'importe où dans votre $fpath
pdfl completions zsh > ~/.zfunc/_pdfl

# fish
pdfl completions fish > ~/.config/fish/completions/pdfl.fish
```

Rien d'autre ne part sur stdout : la sortie peut donc être redirigée directement
dans le répertoire de complétion. Régénérez-la après une mise à jour — le script
est construit à partir des commandes et des options du binaire qui l'a imprimé.

---

[← Bibliothèque standard](10-stdlib.md) · [Sommaire](README.md) · [Suivant : recettes →](12-recipes.md)
