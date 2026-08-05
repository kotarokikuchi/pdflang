# 8. `fix::` 名前空間 — 正規化

[← `codes::`](07-codes.md) · [目次](README.md) · [次：`data::` →](09-data.md)

PDF を**変更して**新しいファイルを保存する19の操作。元のファイルは決して
書き換えられません。

---

## 8.1 使い方

`fix::` は書き込みを行う唯一の名前空間なので、専用のコマンドで実行します：

```bash
pdfl fix input.pdf script.pdfl --output fixed.pdf
```

| オプション | 動作 |
|---|---|
| `--output <file>` | 出力 PDF（必須） |
| `--dry-run` | 保存せず、操作の一覧だけ表示 |
| `--report json\|csv\|html\|pdf` | レポート形式 |
| `--report-file <file>` | レポートをファイルに書き出す |

`pdfl run` で `fix::` を呼ぶと、正しいコマンドを案内するエラーになります。
検証のつもりで修正を適用してしまう事故を防ぐためです。

### 操作の動き

```pdfl
// このスクリプトに check は不要です：順に実行されるコマンドです。
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::add_page_numbers()
fix::add_watermark("DRAFT")
```

各呼び出しは**その場で検証**され（存在しないページ、不正な回転角、無いファイル）、
そのあとで適用されます。レポートには実施内容が `fixes` として入ります：

```json
"fixes": [
  "TrimBox set to [8.5, 8.5, 586.5, 833.5]",
  "page numbering added",
  "watermark \"DRAFT\" added"
]
```

同じスクリプトで検証と修正を混ぜても構いません：

```pdfl
// 修正の前に検証 — 前提が崩れていればレポートに出ます
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(), "file is encrypted, cannot fix it"
}

fix::add_page_numbers()
```

---

## 8.2 ページボックス

| 操作 | 動作 |
|---|---|
| `fix::set_page_size(width, height)` | 全ページの MediaBox を設定 |
| `fix::set_crop_box(x0, y0, x1, y1)` | 全ページの CropBox を設定 |
| `fix::set_trim_box(x0, y0, x1, y1)` | 全ページの TrimBox を設定 |
| `fix::set_bleed_box(x0, y0, x1, y1)` | 全ページの BleedBox を設定 |

座標はポイント、左下から右上の順です。

```pdfl
// A4 をポイントで — 単位を使えば変換は自動です
fix::set_page_size(210mm, 297mm)

// 出版社から制作用ボックス無しで届いた場合：
// TrimBox = 仕上がり、BleedBox = 塗り足し 3mm を含む範囲
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)
```

---

## 8.3 ページ

| 操作 | 動作 |
|---|---|
| `fix::rotate_page([page,] degrees)` | 90/180/270 度回転（ページ省略で全ページ） |
| `fix::delete_page(n)` | ページを削除 |
| `fix::duplicate_page(n)` | ページを複製（直後に挿入） |
| `fix::reorder_pages([...])` | 並べ替え（各ページをちょうど一度ずつ） |
| `fix::split_document(from, to, "out.pdf")` | 範囲を別ファイルに保存 |
| `fix::merge_documents("other.pdf")` | 別 PDF のページを末尾に追加 |

唯一のページを削除しようとすると、明確なメッセージで拒否されます。

```pdfl
fix::rotate_page(90)        // 全ページ
fix::rotate_page(3, 180)    // 3ページ目だけ
fix::delete_page(1)         // 下書きの表紙を削除
fix::reorder_pages([4, 1, 2, 3])

// 表紙と本文を別の業者へ送るために分割
fix::split_document(1, 2, "cover.pdf")
fix::split_document(3, 50, "body.pdf")

fix::merge_documents("attachments/warranty.pdf")
```

---

## 8.4 コンテンツ

| 操作 | 動作 |
|---|---|
| `fix::add_watermark("text")` | 全ページに斜めのグレーの透かし |
| `fix::add_stamps("text")` | 各ページ右上に赤いスタンプ |
| `fix::add_page_numbers()` | フッターに `n / total` |
| `fix::remove_annotations()` | すべての注釈を削除 |
| `fix::remove_attachments()` | すべての添付ファイルを削除 |
| `fix::flatten_layers()` | オプショナルコンテンツ（OCG）を解除 |

```pdfl
fix::add_watermark("DRAFT — DO NOT PRINT")
fix::add_stamps("APPROVED 2026-08-02")
fix::add_page_numbers()

// 印刷所へ送る前に：校正コメントは出てはいけませんし、
// 添付はファイルを重くするだけです
fix::remove_annotations()
fix::remove_attachments()

// 「英語版」を非表示にしたレイヤーが誤って再表示される事故を防ぎます
fix::flatten_layers()
```

---

## 8.5 最適化

> この節の操作は**ファイルが小さくなる場合にのみ**書き込みます。書き直しの
> 結果が大きくなる場合は元のまま保持されます。

| 操作 | 動作 |
|---|---|
| `fix::remove_unused_resources()` | trailer から到達できないオブジェクトを破棄 |
| `fix::downsample_images([dpi])` | 目標 DPI（既定 300）超の画像をリサンプル |
| `fix::compress_images([quality])` | 画像を JPEG で再エンコード（1〜100、既定 85） |

DPI はページ上の**実際の印刷サイズ**から計算されます。

> **CMYK 画像はそのまま保持されます。** リサンプルには RGB への変換が必要で、
> 印刷用の分版が失われてしまうためです。印刷所のファイルでは RGB 画像から
> 削減効果が得られます。

```pdfl
// メール承認用の版に 300 DPI は不要です
fix::downsample_images(96)
fix::compress_images(70)
fix::remove_unused_resources()
```

### 利用できない操作

`subset_fonts` と `linearize_document` は `fix::` の操作として**存在せず**、
未知の関数としてエラーになります。

- **subset_fonts**：実装して計測しました。プロ用の生成ツールは既に使用する
  グリフだけを埋め込むため、削減効果は最良でも 0.5%、他ではゼロでした。
  フォント破損のリスクに見合いません。サブセットかどうかを*確認*するには
  [`prepress::subset_fonts()`](06-prepress.md#64-フォント) を使ってください。
- **linearize_document**：ヒントテーブル（PDF 仕様 §7.14）の生成が必要です。
  これを行う Rust ライブラリは存在せず、部分的な実装ではリーダーが
  「Fast Web View」と認識しません。

---

## 8.6 完全な例

### 出版社のファイルを印刷所向けに整える

```pdfl
// prepare_for_print.pdfl
// 使い方: pdfl fix publisher.pdf prepare_for_print.pdfl --output print.pdf

check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask the publisher for the open version"
}

// 出版社が設定しなかった制作用ボックス
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// 整理：校正コメントと添付は印刷に回しません
fix::remove_annotations()
fix::remove_attachments()
fix::flatten_layers()
fix::remove_unused_resources()
```

### メール承認用の軽量版

```pdfl
// email_version.pdfl
// 使い方: pdfl fix final.pdf email_version.pdfl --output approval.pdf

fix::downsample_images(96)
fix::compress_images(70)
fix::add_watermark("PROOF — NOT THE FINAL VERSION")
fix::add_page_numbers()
```

`pdfl` 自身で結果を確認します：

```bash
pdfl fix final.pdf email_version.pdfl --output approval.pdf
pdfl inspect approval.pdf          # 生成されたファイルのサイズ・DPI・警告
```

---

[← `codes::`](07-codes.md) · [目次](README.md) · [次：`data::` →](09-data.md)
