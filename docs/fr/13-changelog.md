# 13. Changements

[← Recettes](12-recipes.md) · [Sommaire](README.md)

Ce qui a changé à chaque version, et ce que cela peut casser chez vous.

La version est encore en `0.x` : un incrément mineur a le droit de casser
quelque chose. Quand cela arrive, l'entrée dit exactement quoi et comment s'y
adapter. Rien ne change ici en silence.

---

## Non publié

### Casse

- **`pdfl pack` n'empaquette plus les tableurs** (`.xlsx`, `.xls`, `.ods`), et
  nomme le fichier laissé de côté. Aucune fonction `data::` ne sait en ouvrir
  un : un paquet qui en transportait s'installait proprement puis échouait à la
  première consultation. Si vous empaquetiez un tableur, exportez-le d'abord en
  `.csv` ou `.json`.

### Ajouté

- `--tags TAG` sur `run` filtre les checks à exécuter. Répétable ; un check
  s'exécute s'il porte l'un des tags donnés.
- `--json` sur `inspect` et `lint`, `--output json` sur `doc`. Chaque
  sous-commande est désormais lisible par un programme.
- `--output sarif` et `--output junit`, partout où l'on choisit un format de
  rapport — `run`, `compare`, `watch` et `fix`. SARIF est ce que lit GitHub code
  scanning ; JUnit, ce que lit le panneau de tests de n'importe quelle CI.
- `pdfl completions <shell>` imprime un script de complétion pour bash, zsh,
  fish, elvish ou powershell.
- `--quiet` sur chaque commande fait taire la progression et les confirmations
  sur stderr. Les erreurs restent, et `print()` n'est pas touché — c'est la
  sortie du script lui-même, et l'avaler changerait ce qu'il fait.
- `data::load_dataset` et `data::lookup_value` lisent `.json` en plus de `.csv` :
  un tableau de tableaux, ou un tableau d'objets dont le premier nomme les
  colonnes dans l'ordre du fichier.
- `pdfl test <script>` exécute un script sur un dossier de PDF et compare chaque
  rapport à celui enregistré à côté : un profil qui se met à trouver autre chose
  fait échouer un test au lieu de surprendre quelqu'un plus loin. `--update`
  enregistre les rapports attendus.
- `--jobs <n>` sur `pdfl test` exécute autant de cas en même temps, chacun dans
  son processus. Huit fichiers de 41 pages : 8,9s avec `--jobs 1`, 1,1s avec
  `--jobs 8`. La valeur par défaut reste `1`, chaque tâche tenant un document en
  mémoire ; `--jobs 0` en donne un par CPU.
- `--jobs <n>` sur `pdfl watch` aussi : les fichiers sont validés par des
  processus enfants, une passe en lot passe donc à l'échelle de la même façon
  (9,5s à 1,2s sur huit fichiers de 41 pages). Le rapport écrit est identique
  quel que soit `--jobs`.
- `--events` sur `pdfl watch` attend les notifications du système de fichiers au
  lieu d'une minuterie. Sur demande, pas par défaut : sur un montage NFS ou SMB,
  inotify ne signale que ce qu'écrit la machine locale, un dossier réseau
  deviendrait donc muet. Si le surveillant ne peut pas être créé, watch le dit
  et revient à la minuterie.
- `--journal <fichier>` sur `pdfl watch` : un journal en ajout seul de ce qui a
  été validé, un objet JSON par fichier. Relancer avec le même journal saute les
  fichiers qu'il couvre — un lot interrompu à quatre mille sur cinq mille finit
  les mille restants — tout en rapportant leurs verdicts : un lot repris ne
  prétend jamais qu'un dossier est propre.
- `--timeout <s>` sur `pdfl watch` tue l'analyse d'un fichier au-delà de ce
  nombre de secondes et le signale comme refusé — un constat,
  `check_name: "timeout"` — plutôt que de laisser le lot bloqué. La récursion
  dans un script `.pdfl` est déjà limitée en profondeur : ce drapeau est donc
  pour ce qu'un script ne peut pas causer — pdfium qui boucle ou se bloque sur
  un PDF malformé ou hostile.

### Bon à savoir

- Un tag qu'aucun check ne porte est une **erreur**, pas une réussite vide.
  Sinon une chaîne d'intégration avec un tag mal orthographié ne validerait rien
  et annoncerait un fichier propre.
- Une `rule` ne porte pas de tags, donc `--tags` la saute — la même réponse
  qu'un check sans tag.
- Le rapport JSON gagne `checks_run`, les checks et rules qui se sont exécutés.
  Cela n'augmente pas `schema_version` : un lecteur qui ignore les champs
  inconnus y survit. JUnit en a besoin — les diagnostics ne nomment que les
  checks ayant trouvé quelque chose, et une exécution propre annoncée comme zéro
  test est, pour une CI, une exécution qui n'a jamais eu lieu.

### Corrigé

- `pdfl watch` se réveille désormais quand le fichier le plus récent a fini
  d'arriver, et non jusqu'à un intervalle complet plus tard. Avec
  `--debounce 3000`, un fichier qui arrive est signalé après environ 3s au lieu
  de jusqu'à 6s.

---

## 0.12.0

### Ajouté

- Les scripts reçoivent des valeurs depuis la ligne de commande :
  `--var nom=valeur`, lues comme `vars.nom`. Une valeur absente nomme l'option
  qui la fournirait, au lieu de se résoudre en rien.
- Quatre exemples complets de comparaison entre deux documents avec `visual::` :
  `proof.pdfl`, `reprint.pdfl`, `scope.pdfl` et `intake.pdfl`.

### Casse

Rien. Un script qui ne mentionne jamais `vars` se comporte exactement comme
avant.

---

## 0.11.0

### Casse

**Les identifiants de diagnostic ont changé de forme.** C'était `PDFL-001`, un
compteur interne à l'exécution ; ils dérivent maintenant du constat lui-même,
comme `PDFL-093751a2`.

> Tout ce qui correspondait à `PDFL-\d+` ne correspond plus. En échange, un
> identifiant survit à l'insertion d'un check au-dessus de lui — c'est ce qui
> rend tenable une ligne de base approuvée.

**Une entrée illisible sort avec `10` au lieu de `2`.** Un fichier corrompu et
un fichier refusé étaient indiscernables pour une chaîne d'intégration.

> Si votre CI traite `2` comme « ce fichier a été refusé », elle verra `10`
> quand le fichier n'a jamais été jugé. Les constats gardent `0`, `1` et `2` ;
> une erreur de syntaxe garde `3`.

### Ajouté

- Un check peut déclarer la gravité de ses constats :
  `check "..." severity: warning { ... }` — `error` (par défaut), `warning` ou
  `info`. C'est ce qui donne enfin prise à `--fail-on warning`.
- Le rapport JSON commence par `schema_version`, pour qu'un consommateur sache
  quelle forme il lit. Il n'augmente que si un lecteur de la sortie précédente
  cassait ; ajouter un champ ne l'augmente pas.

---

## 0.10.1

### Corrigé

- Le rapport PDF était partiellement en portugais : l'en-tête de section
  affichait `Diagnósticos` et chaque diagnostic portait `(linha N)`. Les deux
  sont en anglais désormais, ce que la documentation promettait depuis toujours.

---

## 0.10.0

### Casse

**Les cibles de publication passent de `x64` à `amd64`**, donc tous les noms
d'assets ont changé.

**Les archives portables ne sont plus publiées**, sauf une pour Linux amd64.

> Tout ce qui télécharge `pdfl-<version>-linux-x64.tar.gz`, ou n'importe quelle
> archive portable autre que Linux amd64, doit changer. En CI, installez depuis
> le `.deb` — les recettes de cette documentation ont été mises à jour en ce
> sens — ou prenez l'archive Linux amd64 là où installer n'est pas envisageable.

### Corrigé

- Deux lacunes trouvées en confrontant la documentation au code source :
  `text::detect_personal_data` et `text::detect_pii` acceptent une chaîne
  optionnelle qui n'était documentée nulle part, et `fix::reorder_pages` était
  écrit de deux façons différentes selon les langues.

---

## 0.9.0

### Ajouté

- Des installateurs pour chaque plateforme : `.deb` pour Linux, `.dmg` pour
  macOS, `-setup.exe` et `.msi` pour Windows.
- Les builds macOS Intel, compilés en croisé depuis le runner Apple Silicon.

### Corrigé

- L'installateur Windows était construit avec des chemins résolus depuis le
  mauvais répertoire, et ne produisait donc jamais de fichier.
- Les paquets de publication embarquaient les en-têtes C et les fichiers de
  build de pdfium, qui ne concernent que ceux qui compilent contre pdfium.
  Environ 550 Ko par paquet.

---

## 0.8.0

### Ajouté

- Windows x64 rejoint les plateformes publiées.

> La `pdfium.dll` fournie se trouve dans `pdfium\bin`, pas `pdfium\lib`. Si vous
> empaquetez `pdfl` vous-même, gardez la disposition livrée : le binaire cherche
> la bibliothèque à côté de lui.

---

## 0.7.0

### Casse

**Les assets de publication portent la version dans leur nom**, sous la forme
`pdfl-<version>-<cible>.tar.gz`, et le répertoire à l'intérieur aussi.

> `.../releases/latest/download/<nom>` ne résout plus, car ce point d'accès
> exige le nom exact du fichier. Téléchargez par motif :
> `gh release download --pattern 'pdfl-*-linux-amd64.*'`.

### Ajouté

- Le code, le README et les exemples sont en anglais. La documentation reste en
  sept langues.

---

## v0.6.1

Première version publique. Le langage, l'interpréteur et dix commandes CLI, avec
une documentation en sept langues.

---

[← Recettes](12-recipes.md) · [Sommaire](README.md)
