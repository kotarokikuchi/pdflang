# 11. CLI コマンド

[← 標準ライブラリ](10-stdlib.md) · [目次](README.md) · [次：レシピ →](12-recipes.md)

13のコマンド：PDF を扱うもの6つ、スクリプトを扱うもの4つ、配布用が2つ、シェル用
が1つ。

| コマンド | 動作 |
|---|---|
| [`run`](#pdfl-run) | スクリプトで PDF を検証 |
| [`compare`](#pdfl-compare) | 2つのバージョンを比較 |
| [`pixelcompare`](#pdfl-pixelcompare) | 2つの PDF をピクセル単位で比較。変化を見るビューアつき |
| [`watch`](#pdfl-watch) | フォルダを監視して到着したファイルを検証 |
| [`fix`](#pdfl-fix) | 修正を適用して新しい PDF を保存 |
| [`inspect`](#pdfl-inspect) | PDF の概要を素早く表示 |
| [`lint`](#pdfl-lint) | スクリプトを実行せずに解析 |
| [`fmt`](#pdfl-fmt) | スクリプトを整形 |
| [`test`](#pdfl-test) | PDF のフォルダに対してスクリプトを実行し、各レポートを比較 |
| [`doc`](#pdfl-doc) | スクリプトからドキュメントを生成 |
| [`pack`](#pdfl-pack) | プロファイルとデータを1つにまとめる |
| [`add`](#pdfl-add) | パッケージをインストール |
| [`completions`](#pdfl-completions) | シェル用の補完スクリプトを出力 |

---

## 終了コード

検証を行うすべてのコマンドで共通です。

| コード | 意味 |
|---|---|
| `0` | すべて合格 |
| `1` | 警告のみ |
| `2` | 検証エラー |
| `3` | スクリプトの構文エラー |
| `10` | 文書を読めなかった、またはファイルを書けなかった — 判定に至っていません |

```bash
pdfl run profile.pdfl file.pdf > report.json
case $? in
  0) echo "approved" ;;
  1) echo "approved with warnings" ;;
  2) echo "rejected — see report.json" ;;
  3) echo "error in the validation script" ;;
esac
```

---

## 全体オプション

| オプション | 動作 |
|---|---|
| `--quiet` | stderr への進捗と確認メッセージを止める |

`--quiet` はサブコマンドの前でも後ろでも効き、すべてのサブコマンドで使えます。
人には要るがパイプラインには要らない行——`report saved to …`、`watching …`、
`watch` のファイルごとの結果——を消します。エラーは**消しません**。静かな実行が
失敗したときも、理由は表示されます。

`print()` も止めません。あれはスクリプト自身の出力であり、握りつぶすとスクリプト
の振る舞いが変わってしまいます。要らない場合は stderr をリダイレクトしてくださ
い。

`--quiet` は `--verbose` より優先されます。

---

## `pdfl run`

スクリプトで PDF を検証します。

```bash
pdfl run <script.pdfl> <input.pdf> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | レポート形式 |
| `--output-file <file>` | — | 標準出力ではなくファイルへ書き出す |
| `--fail-on error\|warning` | `error` | `warning` にすると警告でも終了コード2 |
| `--verbose` | — | 標準エラー出力に追加情報 |
| `--var 名前=値` | — | スクリプトが `vars.名前` として読む値。繰り返し指定可 |
| `--tags TAG` | — | このタグを持つ check だけを実行。繰り返し可。どの check も持たないタグはエラーで、空の合格にはなりません |

```bash
# 端末に JSON レポート
pdfl run prepress.pdfl magazine.pdf

# 顧客に渡す HTML
pdfl run prepress.pdfl magazine.pdf --output html --output-file report.html

# 監査用 PDF（pdf 形式は常にファイルに出力されます）
pdfl run prepress.pdfl magazine.pdf --output pdf --output-file report.pdf

# 表計算用の CSV
pdfl run prepress.pdfl magazine.pdf --output csv --output-file findings.csv

# 厳格モード：警告も不合格にする
pdfl run prepress.pdfl magazine.pdf --fail-on warning
```

### JSON レポート

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

同じ PDF に同じスクリプトを適用すれば、常に**バイト単位で同一のレポート**が
得られます。バージョン管理や CI での差分比較に使えます。

`schema_version` を先頭のキーに置いてあるので、消費側は残りを解析する前に分岐
できます。以前の出力を読んでいた側が壊れる場合にのみ上がり、フィールドの追加で
は上がりません。

### SARIF と JUnit

結果を、誰も開かないログではなくチームがすでに見ている場所へ出すための2つの形式
です。

```bash
# GitHub code scanning：所見がプルリクエストの注釈になる
pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif

# 任意の CI のテストパネル：check ごとに1テスト。合格したものも含む
pdfl run prepress.pdfl magazine.pdf --output junit --output-file pdfl.xml
```

SARIF では所見を **スクリプト** に紐づけます。PDF ではありません。分かっている
行番号は check の行であり、PDF はたいてい CI を通り抜ける成果物であってリポジ
トリ内のファイルではないため、そちらを指すと存在しないパスに注釈を付けることに
なります。検証対象のファイルは `properties.inputFile` に、診断の識別子は
`partialFingerprints` に入ります。後者があるおかげで、GitHub は既に見た所見を
それと認識し、実行のたびに開き直すことをしません。

JUnit では、実行された check がすべてテストケースになります。何も見つけなかった
ものも含みます。失敗だけを並べる形式では、きれいな実行がテスト0件として報告され、
CI はそれを「実行されなかった」と読みます。`info` の所見はケースを失敗させず、
`<system-out>` に書き出されます。

```yaml
- name: Preflight
  run: pdfl run prepress.pdfl magazine.pdf --output sarif --output-file pdfl.sarif
  # 終了コード 2 は不合格のファイル。それでもアップロードは必要
  continue-on-error: true
- uses: github/codeql-action/upload-sarif@v3
  with:
    sarif_file: pdfl.sarif
```

---

## `pdfl compare`

2つのバージョンを比較します：テキスト、構造、メタデータ。

```bash
pdfl compare <v1.pdf> <v2.pdf> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | 形式 |
| `--output-file <file>` | — | ファイルへ書き出す |
| `--normalize` | — | 大文字小文字と空白を無視 |
| `--ignore-dates` | — | 日付を伏せてから比較 |
| `--similarity-threshold <0-100>` | `100` | 許容する最小類似度 |

```bash
pdfl compare approved_v1.pdf new_v2.pdf --normalize --ignore-dates

# 1% までの差を許容し、それを下回るとエラー
pdfl compare v1.pdf v2.pdf --similarity-threshold 99 \
  --output html --output-file diff.html
```

### 動作の仕組み

- ページは番号ではなく**内容で対応付け**られます。途中にページが挿入されても、
  それ以降すべてを差分として報告することはありません。1000ページ超の文書でも
  動きます。
- 対応付いた各ページに類似度スコアと、変化した行のサンプル（`-` 削除、
  `+` 追加）が付きます。
- メタデータの変更は**警告**、テキストの変更はしきい値未満なら**エラー**、
  しきい値以上なら**警告**になります。
- レポートには全体スコアが `similarity` として入ります。

```
page 4 → 4: similarity 97.8% | -original title | +revised title
```

---

## `pdfl pixelcompare`

2つの PDF を、*見た目*でページごとに比較します。

```bash
pdfl pixelcompare <original.pdf> <new.pdf> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--output json\|csv\|html\|pdf\|sarif\|junit` | `json` | レポート形式 |
| `--output-file <ファイル>` | — | レポートをファイルに書き出す |
| `--viewer <フォルダ>` | — | 自己完結したビューアを書き出す：各ページ、差分、そして見るための `index.html` |
| `--dpi <n>` | `150` | 描画解像度。上げるほどよく見えて、その分かかる |
| `--threshold <0.0-1.0>` | `0.05` | 2つのピクセルを別物とみなす色距離 |
| `--max-diff <percent>` | `0.0` | 報告されるまでにページが変わってよい割合 |
| `--pages <範囲>` | 全部 | `1-10` または `1,3,7-12` |
| `--no-align` | — | ページ全体のずれを補正しない |
| `--blur <半径>` | `0` | 比較前のぼかし。アンチエイリアスを吸収する |
| `--jobs <n>` | CPU 1つにつき1件 | 同時に比較するページ数 |

`pdfl compare` が答えるのは「テキストや構造が変わったか」です。こちらが答えるの
は別の問い——「見た目は同じままか」——で、この2つは思っているより頻繁に食い違い
ます。2mm ずれたロゴ、消えたヘアライン、特色を CMYK の掛け合わせに置き換えたもの。
どれもテキストは同一です。

```bash
# 文書全体を JSON で
pdfl pixelcompare approved.pdf reprint.pdf

# 実際に差分を見られる場所つきで
pdfl pixelcompare approved.pdf reprint.pdf --viewer diff/

# 少しのノイズは許容し、残ったものをよく見る
pdfl pixelcompare approved.pdf reprint.pdf --max-diff 0.1 --dpi 300
```

変化したページごとに1つの所見。ピクセルの割合と、それがいくつの離れた領域に
分かれているかを示します。

```
page 7: 0.51% of the pixels differ, in 29 area(s)
```

片方にしかないページはそれ自体が所見です——比較する相手がありません。レポートの
`similarity` は比較したページの平均なので、200ページ中1ページを作り直しても別の
文書には見えません。ページごとの数値は診断にあります。

### 位置合わせと、それが既定で有効な理由

同じ元データから書き出し直したファイルは、1〜2ピクセルずれることがよくあります。
補正しないと、ページ上のすべてのグリフの縁が「違う」ことになり、本当に大事な1つの
変化が埋もれてしまいます。`pixelcompare` は全体のずれを1つだけ探し——まず縮小した
コピーで粗く、次に精密に——見つかれば報告します。

```
page 3: 2.10% of the pixels differ, in 44 area(s) (aligned by 2, -1 px)
```

位置そのものを検査したい場合は `--no-align` で切ってください。

### ビューア

`--viewer diff/` は、1ページにつき3つの PNG と1つの `index.html` を収めた
フォルダを書き出します。依存は一切ありません——CDN もバンドラもサーバも不要。
ファイルを開くか、フォルダを固めて再印刷を承認する人に送ってください。

3つのペインが横に並び、常に同じページを映します：

| ペイン | 何を映すか |
|---|---|
| **Original** | 1つ目のファイルのページ、そのまま |
| **New** | 2つ目のファイルのページ、そのまま |
| **Difference** | 両方に、変わったところを重ねて着色——ドラッグでワイプ |

3つのペインは同じ位置に同じバーを持ち、どれをドラッグしても3つとも動きます。
**Difference** のペインではバーが切れ目になり、左が元、右が新しい方です。
残る2つではページの同じ列を通る定規になり、ワイプが切っている場所を、どちらの
原本の上でも目分量なしに見つけられます。位置はペインではなくページに対する
割合なので、ページを切り替えてもウィンドウの大きさを変えても保たれます。

差分はその場に着色され、色が種類を表します：

| 色 | 意味 |
|---|---|
| 赤 | 新しいファイルから消えたインキ |
| 緑 | そこで新しく現れたインキ |
| 青 | 同じ太さで色が違うもの |

3つのペインはウィンドウを基準に大きさが決まるので、比較の全体がスクロール
なしで画面に収まり、どんなウィンドウの形でもページの縦横比を保ちます。2つの
ファイルでページの大きさが食い違う場合——片方が横向きになった、など——は、
共通の枠いっぱいに引き伸ばすのではなく、それぞれを丸ごと収めて表示します。

**開いた時点で、違いのあるページに合わせます。** 200ページのうち3ページが
動いた文書なら、その3ページこそ開いた理由です。**All** で残りが戻ります。
矢印と `←` `→` は絞り込みに従い、隠れているページを飛び越えます。どこにも
違いがなければ、絞り込みのボタンはそう告げて無効のままになり、一覧を空に
することはありません。

### 進捗

長い文書を300 dpi でラスタライズすると数分かかるため、各段階が stderr にバーを
描きます。ラスタライズするファイルごとに1本、比較に1本、ビューアの書き出しに1本。

```
rasterising approved.pdf  [############------------]  98/207
```

描くのは stderr が端末のときだけです。バーは行頭に戻って上書きすることで動くので、
カーソルを動かせないログファイルにリダイレクトすると、代わりに数千の断片が
たまってしまいます。リダイレクト時は黙り、通常のメッセージはそのまま出ます。
`--quiet` はどの場合でも黙らせます。

### 速度

比較は既定ですべての CPU を使います。150 dpi の41ページで：

| `--jobs` | 時間 |
|---|---|
| `1` | 3.6秒 |
| `4` | 1.7秒 |
| `8` | 1.2秒 |
| `20` | 1.3秒 |

8あたりで頭打ちになります。この段階を縛っているのは計算量ではなくメモリ帯域だから
です——ページ全体を CPU に流し込む処理なので、それ以上のスレッドは同じメモリの前で
順番待ちをするだけになります。多く指定しても害はありませんが、意味もありません。

**並列にならない**ものにも注意してください：ラスタライズです。pdfium はすべての
呼び出しを1つのグローバルロックの背後で直列化するので、その前に立つ2本目のスレッド
は待つだけです。これが実行時間の下限になり——上の数字のうち約0.8秒——`--jobs 8` が
8倍ではなく3倍にとどまる理由でもあります。

ここでは既定が CPU 1つにつき1件で、`pdfl test` と `pdfl watch` は `--jobs 1` です。
この違いには理由があります。あちらでは1ジョブが自分の文書を抱えた子プロセスなので、
1つ増えるごとに文書がもう1つメモリに乗ります。こちらではページがすでにメモリにあり、
スレッドはそれを共有するので、1ジョブの費用は1ページ分の作業領域です。マシンを共有
しているなら下げてください。

終了コード：`0` どのページも `--max-diff` を超えて変わらなかった、`2` 1ページ
以上が変わった、`10` ファイルが読めないかビューアを書けなかった。

レポートは `--jobs` に左右されません。ページはページ順に畳み直されるので、診断も、
その順序も、指紋も、どの値でも同一に出ます——テストがそれを保証しており、ビューアの
ファイルもバイト単位で同一です。

---

## `pdfl watch`

フォルダを監視し、到着または変更された PDF を検証します。

```bash
pdfl watch <folder> --script <script.pdfl> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--pattern <glob>` | `*.pdf` | 処理対象のファイル |
| `--exclude <glob>` | — | 除外するファイル |
| `--output-dir <folder>` | PDF と同じ場所 | レポートの出力先 |
| `--depth <n>` | `1` | サブフォルダの深さ |
| `--debounce <ms>` | `1000` | ファイルが安定するまでの待ち時間 |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | レポート形式 |
| `--fail-fast` | — | 最初のエラーで停止 |
| `--events` | — | タイマーではなく OS の通知で起きる（ネットワーク共有では不可） |
| `--journal <ファイル>` | — | 検証済みのものを追記のみで記録。再実行時はそこにあるものを飛ばす |
| `--timeout <秒>` | — | この秒数を超えたファイルの解析を強制終了し、不合格として報告する |
| `--var 名前=値` | — | すべてのファイルが `vars.名前` として読む値。繰り返し指定可 |
| `--jobs <n>` | `1` | 同時に検証するファイル数。`0` は CPU 1つにつき1件 |
| `--once` | — | 既にあるファイルを処理して終了 |

```bash
# 印刷所の受付フォルダを常時監視
pdfl watch inbox/ --script preflight.pdfl --output-dir reports/ --report html

# CI 向けのバッチ実行：処理後、最悪の終了コードで終了
pdfl watch inbox/ --script preflight.pdfl --once
echo "result: $?"
```

`--jobs` は、そのパスで処理すべきものすべてに効きます。バッチでも、まとめて到着
したときでも同じです。各ファイルはそれぞれの `pdfl` プロセスが検証し（`pdfl test`
と同じ理由です）、レポートを書き出すのはこのプロセスなので、書かれるファイルは
`--jobs` の値によらず同一です。41ページのファイル8つで、`--jobs 1` が9.5秒、
`--jobs 0` が1.2秒。

`--fail-fast` を付けた場合、1件でも失敗したらそれ以降の新しいファイルは始めません。
すでに走っているものは最後まで走ります。途中で殺すと書きかけのレポートが残るから
です。レポートはファイルが見つかった順に書かれるので、同時に何件走ったかにかかわ
らずバッチは同じ行を出力します。

待機は、最も新しいファイルが書き終わったちょうどその時点で終わります。待機中に
到着したファイルが、さらに1周期ぶん待たされることはありません。

既定ではフォルダをタイマーで一覧します。`--events` を付けると、代わりに OS の
通知を待ちます。既定がタイマーなのは実測に基づきます。1万個のファイルを200ms
ごとに一覧しても計測できるほどの CPU は使わず、どちらにせよレイテンシは settle
時間が支配するため、ローカルのフォルダでは両者の差は100分の1秒に収まります。

ネットワーク共有で `--events` を使わないでください。NFS や SMB のマウントでは
inotify はローカルの書き込みしか報告しないため、他のマシンから届いたファイルは
永久に気づかれず、しかも watch は何も言いません。効くのは、多数のフォルダを
監視するマシンや、ディレクトリの一覧が高価な場合です。監視を開始できなかった
ときは、黙り込まずにその旨を告げてタイマーに戻ります。

**debounce** があるのは、大きなファイルが少しずつ届くためです。ファイルの
変化が止まってから処理するので、途中まで書かれた PDF を読むことがありません。

### journal：中断されたバッチを最後まで終わらせる

5000個のファイル、4000個目でマシンが再起動。記録がなければ、次の実行は1個目から
やり直しになります。

```bash
pdfl watch inbox/ --script offset.pdfl --once --journal batch.jsonl
```

1ファイルにつき1つの JSON オブジェクトを、検証のたびに追記します。

```json
{"input":"inbox/cover.pdf","sha256":"9f2b…","status":"FAIL","errors":2,"warnings":0,"exit":2}
```

同じ journal を指定して再実行すると、そこに載っているファイルは飛ばされます。
ただし判定は飛ばされません。再開したバッチが不合格のファイルを飛ばしても、終了
コードは `2` のままです。journal はバッチの記録であり、終了コードはその判定だから
です。失敗をすでに見たからという理由で「きれい」と報告するバッチは、このツールが
持ちうる最悪のバグです。

ファイルは**バイト列**で照合します。名前でもタイムスタンプでもありません。
`cover.pdf` を別の `cover.pdf` に差し替えれば、また検証されます。ハッシュが記録と
違うからです。

`--journal` を付けなければ何も書きません。このツールは自前の状態を持ちません。
これはあなたが名前を指定して要求したファイルであり、レポートと同じ扱いです。
そして行にタイムスタンプはありません。journal はファイルが検証された*かどうか*と
その結果を、隣のレポートは*何を*、ファイルシステムは*いつ*を答えます。これにより
再実行は最初とバイト単位で同一になります——ここの他のすべてと同じように。

行は1つずつ書き出されるので、クラッシュが残したものはその範囲で正しい記録です。
読めない journal は、何行目かを告げて実行を止めます。読み損ねた記録を根拠に
ファイルを飛ばすほうが、やり直すより悪いからです。

### `--timeout`：1つの悪いファイルでバッチを止めない

```bash
pdfl watch inbox/ --script offset.pdfl --once --timeout 60
```

解析が `60` 秒を超えたファイルは強制終了され、読めない PDF と同じ扱いで報告され
ます——1つの所見を持つレポート、`check_name: "timeout"`——なので出力され、ディスク
に書かれ、他の判定とまったく同じように journal にも入ります。何も黙って飛ばされ
ることはなく、バッチはそのファイルで止まらず次に進みます。

```json
{"input":"inbox/adversarial.pdf","sha256":"7a1c…","status":"FAIL","errors":1,"warnings":0,"exit":2}
```

`.pdfl` 言語には、スクリプトがわざとインタプリタを止められるような仕組みはあり
ません——再帰には深さの上限があります。`--timeout` はスクリプトでは起こせないもの
——不正な、あるいは悪意ある PDF に対して pdfium がループしたり止まったりすること
——のために存在します。フラグを付けなければ、ファイルの解析は必要なだけ待ちます。
このフラグができる前はそれが唯一の挙動でした。

`--var` は各ファイルに変わらず届きます——実行全体で1つの値であり、ファイルごとに
変わるもの（注文番号）ではなく、フォルダ全体で一定のもの（クライアント名）に向いて
います。これがなければ、`vars.*` を読むスクリプトは決して監視できません。どの
ファイルも「was not provided」で失敗します。

レポートは `<name>.report.json`（または `.csv`、`.html`、`.pdf`）として

レポートは `<name>.report.json`（または `.csv`、`.html`、`.pdf`）として
書き出されます。

---

## `pdfl fix`

`fix::` の操作を適用し、新しい PDF を保存します。詳細は[第8章](08-fix.md)。

```bash
pdfl fix <input.pdf> <script.pdfl> --output <output.pdf> [options]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--output <ファイル>` | — | 出力する PDF（必須） |
| `--dry-run` | — | 保存せずに操作を一覧表示する |
| `--report json\|csv\|html\|pdf\|sarif\|junit` | `json` | レポート形式 |
| `--report-file <ファイル>` | — | レポートをファイルに書き出す |

```bash
# 何が行われるかだけ確認（保存しない）
pdfl fix original.pdf normalize.pdfl --output out.pdf --dry-run

# 実際に適用
pdfl fix original.pdf normalize.pdfl --output fixed.pdf
```

---

## `pdfl inspect`

スクリプト無しで PDF の概要を表示します。

```bash
pdfl inspect <file.pdf>
```

`--json` は同じ要約をデータとして返します。

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

新しいファイルが届いたら最初に実行するコマンドです。数秒で、開く価値があるか
判断できます。

---

## `pdfl lint`

スクリプトを実行せずに解析し、品質上の問題を報告します。

```bash
pdfl lint <script.pdfl>
```

`--json` は同じ警告をデータとして返します。

検出する内容：

- 宣言されて**一度も使われない**変数・ブロック引数・関数（`_` を前置すると
  抑制されます：`_page`）
- **重複**または**空**の check
- 未知の名前空間（`text::`、`struct::`、`visual::`、`prepress::`、`codes::`、
  `fix::`、`data::`）
- check の外にある `assert` / `require`
- `fix::` の使用（`pdfl fix` でのみ動作します）

```bash
$ pdfl lint profile.pdfl
profile.pdfl: warning: variable 'LIMIT' declared and never used
profile.pdfl: warning: check "Fonts" declared 2 times
```

警告があれば終了コード `1` になります。CI で使えます。

---

## `pdfl fmt`

スクリプトを整形します：2スペースのインデント、一貫した空白、空行の圧縮。
コメントと単位（`3mm` は `3mm` のまま）は保持されます。

```bash
pdfl fmt <script.pdfl>            # その場で整形
pdfl fmt <script.pdfl> --check    # 書き換えず、未整形なら終了コード1
```

```bash
# CI でチーム標準を強制する
for f in profiles/*.pdfl; do pdfl fmt "$f" --check || exit 1; done
```

---

## `pdfl doc`

スクリプト自身からドキュメントを生成します。

```bash
pdfl doc <script.pdfl> [--output markdown|html|json]
```

出力内容：プロファイル、定数の表、関数、import、そして各 check のタグと
検証内容（`assert` のメッセージが説明になります）。

```bash
pdfl doc prepress.pdfl > docs/prepress-profile.md
pdfl doc prepress.pdfl --output html > profile.html
```

コードを読まない制作管理者に、プロファイルが何を検証しているかを伝えるための
成果物です。

---

## `pdfl pack`

スクリプトとデータを配布可能な `.pdflpkg` にまとめます。

```bash
pdfl pack <folder> [--name <name>] [--version <version>] [--output <file>]
```

フォルダ内の `.pdfl`、`.csv`、`.txt`、`.json` を再帰的に収集し、各ファイルの
SHA-256 を記録した `manifest.json` を付けます。パッケージは決定的です：同じ
フォルダからは同一のバイト列が生成されます。

表計算ファイル（`.xlsx`、`.xls`、`.ods`）は**含めません**。除外したファイル名は
`pack` が伝えます。`data::` のどの関数もそれらを開けないため、同梱すると、
インストールは成功するのに最初の参照で失敗するパッケージを配ることになります。

```bash
pdfl pack profiles/print-shop --name print-profile --version 1.0.0
```

---

## `pdfl add`

ローカルのパッケージをインストールし、マニフェストのハッシュを検証します。

```bash
pdfl add <package.pdflpkg> [--dir <folder>]
```

```bash
pdfl add print-profile.pdflpkg
# ./pdfl_profiles/print-profile@1.0.0/ にインストールされます

pdfl run pdfl_profiles/print-profile@1.0.0/prepress.pdfl file.pdf
```

いずれかのファイルのハッシュが記録と異なる場合、インストールは**拒否**され
ます。改ざんや破損したパッケージは入りません。

> リモートリポジトリと電子署名はこのバージョンには含まれません。`add` は
> ローカルファイルからインストールします。

---

## `pdfl test`

フォルダ内のすべての PDF に対してスクリプトを実行し、各レポートを隣に記録された
ものと比較します。プロファイルが違うものを見つけ始めたら、下流の誰かを驚かせる
前にテストが落ちます。

```bash
pdfl test <script.pdfl> [--dir <フォルダ>] [--update]
```

| オプション | 既定 | 動作 |
|---|---|---|
| `--dir <フォルダ>` | スクリプトの隣の `tests/` | ケースの PDF が置いてある場所 |
| `--update` | — | 比較せず、期待するレポートを記録する |
| `--jobs <n>` | `1` | 同時に走らせるケース数。`0` は CPU 1つにつき1件 |
| `--var 名前=値` | — | すべてのケースが `vars.名前` として読む値。繰り返し指定可 |

1つのケースは、PDF とそれに期待するレポートを並べたものです。

```
profiles/print-shop/
  prepress.pdfl
  tests/
    approved.pdf
    approved.expected.json
    heavy_ink.pdf
    heavy_ink.expected.json
```

```bash
# 最初の一度：いまスクリプトが見つけるものを記録する
pdfl test prepress.pdfl --update

# それ以降
pdfl test prepress.pdfl
```

```
ok   approved.pdf
FAIL heavy_ink.pdf
     error_count: expected 1, got 0
     missing:    PDFL-093751a2 [error] Ink coverage (line 12): page 7: 324% ink (limit 300%)
1 passed, 1 failed
```

失敗したときは、2つの JSON を並べて表示するのではなく、何が変わったのか——件数、
判定、そして現れた所見と消えた所見——を挙げます。

記録は常に意図的な操作です。自分のベースラインを勝手に更新する実行は、決して
失敗しません。まず差分を読み、その変更が意図したものであれば `--update` で
記録し直してください。

期待するレポートは `pdfl run` が出すものと同じで、`input_file` だけはファイル名
に縮めてあります。呼び出したディレクトリによって変わるベースラインは、ベース
ラインではないからです。開けない PDF はそのケースだけを失敗させ、残りは実行され
ます。

終了コード：`0` 全件合格、`2` 1件以上の不合格、`10` フォルダが読めないか PDF が
ない。

### ケースを同時に走らせる

各ケースはそれぞれ独立した `pdfl` プロセスとして走るので、`--jobs` は本当の並列
処理になります。41ページのファイル8つで、`--jobs 1` が8.9秒、`--jobs 8` が1.1秒
でした。1つのプロセス内のスレッドでは達成できません——pdfium はすべての呼び出しを
1つのミューテックスで直列化するため、スレッド版は逐次実行より*遅く*計測されました。

既定値が `1` なのは、各ジョブが1つの文書をメモリに抱えるプロセスであり、この
ツールが非常に大きくなりうるファイルのために存在するからです。ケースが普通の
大きさなら増やしてください。`--jobs 0` で CPU 1つにつき1件になります。

出力の順序は `--jobs` によって変わりません。どの子プロセスが先に終わっても、
ケースは見つかった順に判定されます。

PDF が読めないケースも他と同じように判定されます。そのレポートは理由を所見として
持つので、「このファイルは読めないものとして却下されるべき」ということ自体を
テストにできます。そのレポートはファイルを渡されたとおりの名前で記録するため、
ベースラインをバージョン管理に入れるなら `--dir` は**相対パス**で記録してくださ
い。

`--var` は各ケースに変わらず届きます——実行全体で1つの値であり、ファイルごとでは
ありません。これがなければ、`vars.*` を読むスクリプトは決してテストできません。
どの PDF であってもすべてのケースが「was not provided」で失敗します。

---

## `pdfl completions`

シェル用の補完スクリプトを標準出力に書き出します。

```bash
pdfl completions <bash|zsh|fish|elvish|powershell>
```

```bash
# bash（現在のユーザー）
pdfl completions bash > ~/.local/share/bash-completion/completions/pdfl

# zsh — $fpath 上のどこでも
pdfl completions zsh > ~/.zfunc/_pdfl

# fish
pdfl completions fish > ~/.config/fish/completions/pdfl.fish
```

標準出力にはこれ以外を書かないので、そのまま補完ディレクトリへリダイレクトできま
す。アップグレード後は生成し直してください。スクリプトは、それを出力したバイナリ
のコマンドとフラグから組み立てられます。

---

[← 標準ライブラリ](10-stdlib.md) · [目次](README.md) · [次：レシピ →](12-recipes.md)
