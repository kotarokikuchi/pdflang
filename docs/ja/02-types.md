# 2. ドキュメント型

[← 言語](01-language.md) · [目次](README.md) · [次：`text::` →](03-text.md)

すべてのスクリプトは `doc` 変数を自動的に受け取ります。これが解析対象の PDF
です。ここからページ、フォント、画像にたどり着きます。

---

## 2.1 `doc` — ドキュメント

### プロパティ

| プロパティ | 型 | 内容 |
|---|---|---|
| `doc.page_count` | 数値 | ページ数 |
| `doc.title` | 文字列 | メタデータのタイトル（無い場合は空） |
| `doc.author` | 文字列 | メタデータの著者（無い場合は空） |
| `doc.filename` | 文字列 | 解析対象のファイル名 |
| `doc.pages` | リスト | すべてのページ |
| `doc.fonts` | リスト | 使用されているすべてのフォント |
| `doc.images` | リスト | 全ページのすべての画像 |

```pdfl
check "Document properties" {
  print("file:", doc.filename)
  print("pages:", doc.page_count)
  print("title:", doc.title)

  // コレクションは通常のリストです — すべてのリストメソッドが使えます
  require doc.pages.length == doc.page_count
  require doc.fonts.length > 0
  print("images in the whole document:", doc.images.length)
}
```

### メソッド

#### `doc.extract_text()`

ドキュメント全体のテキスト。ページは改行で区切られます。

```pdfl
check "Document text" {
  text = doc.extract_text()
  assert text.trim() != "", "PDF has no extractable text (images only?)"
  require text.contains("Agreement")
  print("total characters:", text.length)
}
```

---

## 2.2 `page` — ページ

ページは `doc.pages`（ブロック内）または `page` 変数（`rule` 内）から得ます。

### プロパティ

| プロパティ | 型 | 内容 |
|---|---|---|
| `page.number` | 数値 | ページ番号（**1** 起点） |
| `page.index` | 数値 | ページ索引（**0** 起点） |
| `page.width` | 数値 | 幅（ポイント） |
| `page.height` | 数値 | 高さ（ポイント） |
| `page.images` | リスト | このページの画像 |
| `page.tac` | 数値 | 推定インク総量の最大値（%） |
| `page.ink_coverage` | 数値 | 推定インク総量の平均（%） |
| `page.min_stroke_width` | 数値/null | 最も細い線幅（pt）。線が無ければ `null` |
| `page.has_media_box` | 真偽値 | MediaBox が定義されている |
| `page.has_crop_box` | 真偽値 | CropBox が定義されている |
| `page.has_trim_box` | 真偽値 | TrimBox が定義されている |
| `page.has_bleed_box` | 真偽値 | BleedBox が定義されている |
| `page.has_art_box` | 真偽値 | ArtBox が定義されている |

```pdfl
check "Page format" {
  doc.pages.each { |page|
    // number は人が見る番号、index は内部計算用
    assert page.width > 100mm,
      "page #{page.number} is too narrow: #{page.width}pt"

    // ボックス：印刷には必須です
    assert page.has_trim_box,
      "page #{page.number} has no TrimBox (trim area)"
    assert page.has_bleed_box,
      "page #{page.number} has no BleedBox (bleed area)"
  }
}

check "Ink and strokes" {
  doc.pages.each { |page|
    assert page.tac <= 300,
      "page #{page.number}: #{page.tac}% ink (limit 300%)"

    // min_stroke_width は null になり得ます（線の無いページ）。
    // null は偽なので、この書き方は安全です：
    assert !page.min_stroke_width || page.min_stroke_width >= 0.25,
      "page #{page.number} has a hairline stroke"
  }
}
```

### メソッド

#### `page.extract_text()`

このページだけのテキスト。

```pdfl
check "Blank pages" {
  blank = doc.pages.filter { |p| p.extract_text().trim() == "" }
  assert blank.length == 0,
    "#{blank.length} blank page(s): #{blank.map { |p| p.number }.join(", ")}"
}
```

---

## 2.3 `font` — フォント

フォントは `doc.fonts` から得ます。

| プロパティ | 型 | 内容 |
|---|---|---|
| `font.name` | 文字列 | フォント名 |
| `font.is_embedded` | 真偽値 | ファイルに埋め込まれているか |

```pdfl
check "Embedded fonts" {
  // 埋め込まれていないフォントはリーダーが代替します — 見た目が変わります
  doc.fonts.each { |font|
    assert font.is_embedded,
      "font '#{font.name}' is not embedded in the PDF"
  }
}

check "Font report" {
  print("fonts in use:", doc.fonts.map { |f| f.name }.join(", "))
  missing = doc.fonts.filter { |f| !f.is_embedded }
  print("not embedded:", missing.length)
}
```

---

## 2.4 `image` — 画像

画像は `doc.images`（全体）または `page.images`（1ページ分）から得ます。

| プロパティ | 型 | 内容 |
|---|---|---|
| `image.width` | 数値 | 幅（**ピクセル**） |
| `image.height` | 数値 | 高さ（**ピクセル**） |
| `image.dpi` | 数値 | 実効解像度（dpi_x と dpi_y の小さい方） |
| `image.dpi_x` | 数値 | 水平方向の実効解像度 |
| `image.dpi_y` | 数値 | 垂直方向の実効解像度 |
| `image.color_space` | 文字列 | `DeviceRGB`、`DeviceCMYK`、`Indexed`… |
| `image.page_number` | 数値 | 配置されているページ（1 起点） |
| `image.bits_per_pixel` | 数値 | ビット深度 |

> **DPI は実効値**です。ピクセル数 ÷ ページ上の印刷サイズで計算され、
> メタデータの公称値ではありません。印刷品質に効くのはこちらです。1000 px
> の画像を 20 cm に引き伸ばせば、メタデータが何と言おうと DPI は低くなります。

```pdfl
profile "images-for-offset" {
  const MIN_DPI = 300

  check "Resolution" {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image #{img.width}x#{img.height}px on page #{img.page_number}: #{img.dpi} DPI (minimum #{MIN_DPI})"
    }
  }

  check "Color space" {
    // オフセット印刷は CMYK です。RGB は変換が必要です
    doc.images.each { |img|
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number} — convert to CMYK"
    }
  }

  check "Images per page" {
    doc.pages.each { |page|
      // page.images はそのページの画像だけを持ちます
      print("page", page.number, "has", page.images.length, "image(s)")
    }
  }
}
```

---

## 2.5 `region` — ページ上の領域

領域は矩形でページの一部を指定し、フッター、ヘッダー、バーコード欄、医薬品の
表示帯などを個別に検証できます。

### 作成

```pdfl
// region(x, y, 幅, 高さ [, "名前"])
// 原点 (0,0) は PDF と同じく左下です。
header = region(0, 742, 595, 100, "header")
footer = region(0, 0, 595, 60, "footer")
band = region(20mm, 250mm, 60mm, 15mm, "red band")
```

### プロパティ

| プロパティ | 内容 |
|---|---|
| `region.name` | 作成時に付けた名前（省略時は空） |
| `region.x` / `region.y` | 左下の座標 |
| `region.width` / `region.height` | 寸法 |
| `region.right` / `region.top` | 右辺と上辺（計算値） |
| `region.area` | 面積（平方ポイント） |

### メソッド

| メソッド | 動作 |
|---|---|
| `region.contains_point(x, y)` | その点は内側か |
| `region.intersects(other)` | 2つの領域が重なるか |
| `region.expand(pt)` | 全方向に広げた新しい領域 |
| `region.inset(pt)` | 全方向に縮めた新しい領域 |
| `region.export_coordinates()` | `[x0, y0, x1, y1]` |

```pdfl
check "Working with regions" {
  footer = region(0, 0, 595, 60, "footer")

  require footer.name == "footer"
  require footer.top == 60.0
  require footer.right == 595.0
  require footer.area == 35700.0

  // 点がフッター内にあるか
  require footer.contains_point(300, 30)
  require !footer.contains_point(300, 500)

  // 重なり：要素が禁止領域に侵入していないかの確認に使えます
  header = region(0, 780, 595, 62)
  require !footer.intersects(header)

  // expand/inset は「新しい」領域を返します（元は変わりません）
  slack = footer.expand(5mm)      // 各辺 5mm 広い
  safe = footer.inset(3mm)        // 各辺 3mm 狭い
  require slack.area > footer.area
  require safe.area < footer.area
}
```

### 検証での使い方

```pdfl
profile "medicine-label" {

  check "Prescription band" {
    // 表示帯は上部にあり、法定文言を含む必要があります
    band = region(0, 700, 595, 142, "band")
    content = text::extract_from_region(1, band)
    assert content.contains("PRESCRIPTION ONLY"),
      "band is missing the mandatory text"
  }

  check "Ink in the fold area" {
    // 折り目のインクが多すぎると加工で割れます
    fold = region(290, 0, 15, 842, "center fold")
    measured = prepress::calculate_tac_by_region(1, fold)
    assert measured.first() < 240,
      "too much ink on the fold: #{measured.first()}%"
  }

  check "Barcode in the right place" {
    code_area = region(400, 20, 180, 80, "barcode area")
    assert codes::validate_barcode_position(code_area),
      "barcode outside the reserved area"
  }
}
```

---

[← 言語](01-language.md) · [目次](README.md) · [次：`text::` →](03-text.md)
