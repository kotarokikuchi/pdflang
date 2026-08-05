# 6. `prepress::` 名前空間 — 印刷前検査

[← `visual::`](05-visual.md) · [目次](README.md) · [次：`codes::` →](07-codes.md)

刷版に回す前に印刷会社が確認すべき項目を扱う30の関数：インク総量、分版、
フォント、線幅、ページボックス。

---

## 6.1 インク総量（TAC）

TAC（Total Area Coverage）は、ある一点における4色インクの合計です。印刷機の
上限を超えると、汚れ、乾燥不良、裏移りが起こります。コート紙のオフセットでは
一般に 300% が上限です。

測定方法は**2つ**あり、その違いが重要です。

| 関数 | 動作 |
|---|---|
| `prepress::calculate_exact_tac([page])` | ファイルに**宣言された色**から算出（正確） |
| `prepress::calculate_tac([page])` | RGB レンダリングによる推定（**下限値**） |
| `prepress::validate_tac_limits([limit])` | 全ページが上限内なら真（既定 300） |
| `prepress::calculate_ink_coverage([page])` | 平均インク量（%） |
| `prepress::calculate_tac_by_region(page, region)` | 領域内の `[最大TAC, 平均]` |

推定値では、暗い無彩色（リッチブラック）が 100% 付近に潰れます。

```pdfl
check "Ink limit" {
  // 上限の検証には常に「正確な TAC」を使います
  doc.pages.each { |page|
    tac = prepress::calculate_exact_tac(page.number)
    assert tac <= 300, "page #{page.number}: #{tac}% ink"
  }

  print("exact (declared in the file):", prepress::calculate_exact_tac(), "%")
  print("estimated (by rendering):", prepress::calculate_tac(), "%")
  // 実ファイルでの実測：正確 324% に対し推定 299%
  // — 上限超過は正確な値でしか判りません。

  // 折り目のインクが多すぎると加工で割れます
  fold = region(290, 0, 15, 842, "center fold")
  measured = prepress::calculate_tac_by_region(1, fold)
  assert measured.first() < 240,
    "TAC of #{measured.first()}% on the fold (max 240%)"
}
```

---

## 6.2 色と分版

| 関数 | 動作 |
|---|---|
| `prepress::detect_spot_colors()` | 特色インク（Separation / DeviceN）の一覧 |
| `prepress::detect_color_mode()` | `"CMYK"` / `"RGB"` / `"Mixed"` / `"None"` / `"Other"` |
| `prepress::validate_color_space(space)` | 全画像が指定の色空間なら真 |
| `prepress::compare_colors_delta_e(a, b)` | 2色の Delta-E（CIE76） |
| `prepress::detect_rich_black()` | 複数インクで構成された黒があれば真 |
| `prepress::validate_overprint_settings()` | オーバープリントが無効なら真 |
| `prepress::validate_output_intent([name])` | 出力インテントの有無／名前一致 |
| `prepress::check_rendering_intent([expected])` | レンダリングインテントの一覧／検証 |

色はリストで渡します：4値 = CMYK、3値 = RGB、1値 = グレー。Delta-E の目安は、
1未満で知覚不能、3までは印刷で許容、5超で明確に異なります。

> 予約された分版 `All` と `None` は一覧から除外されます。`All` はレジスト
> マークであってインクではありません。

```pdfl
check "Colors" {
  spots = prepress::detect_spot_colors()
  assert spots.length == 0,
    "file uses an unquoted special ink: #{spots.join(", ")}"

  mode = prepress::detect_color_mode()
  assert mode == "CMYK" || mode == "None",
    "document is #{mode} — offset printing requires CMYK"

  // ブランドカラーの許容差
  difference = prepress::compare_colors_delta_e([1.0, 0.6, 0.0, 0.1], [1.0, 0.62, 0.0, 0.12])
  assert difference < 3.0, "brand color out of tolerance (ΔE #{difference})"

  // 小さな文字のリッチブラックは見当ずれが目立ちます
  assert !prepress::detect_rich_black(),
    "rich black found — use flat black (0/0/0/100) for text"

  // 意図しないオーバープリントは要素を消してしまいます
  assert prepress::validate_overprint_settings(),
    "overprint is enabled — verify that it is intentional"

  assert prepress::validate_output_intent(),
    "PDF has no Output Intent — the shop cannot know the target color profile"
}
```

---

## 6.3 線幅

| 関数 | 動作 |
|---|---|
| `prepress::detect_hairlines([limit])` | 限界（既定 0.25 pt）未満の線があれば真 |
| `prepress::detect_hairlines_exact()` | 線幅 0 の線があれば真 |
| `prepress::detect_fine_lines([limit])` | 同上（既定 1 pt） |
| `prepress::validate_minimum_stroke_width(min)` | 全ての線が最小値以上なら真 |

線幅 0 は PostScript 由来のヘアラインで、装置の最小幅で描かれます（予測不能）。

```pdfl
check "Strokes" {
  assert !prepress::detect_hairlines(0.25),
    "there are strokes below 0.25 pt — they will disappear in print"
  assert !prepress::detect_hairlines_exact(),
    "there is a stroke with 0 width — set a real thickness"
  assert prepress::validate_minimum_stroke_width(0.5),
    "the shop contract requires strokes of at least 0.5 pt"
}
```

---

## 6.4 フォント

| 関数 | 動作 |
|---|---|
| `prepress::list_fonts()` | 使用フォント名の一覧 |
| `prepress::validate_font_embedding()` | 全フォントが埋め込み済みなら真 |
| `prepress::detect_text_substitution()` | 埋め込まれていないフォントの一覧 |
| `prepress::detect_missing_glyphs()` | 幅テーブルの無いフォントの一覧 |
| `prepress::subset_fonts()` | 埋め込みフォントがすべてサブセットなら真 |
| `prepress::check_font_licensing()` | ライセンス上のリスク（Type3・非埋め込み） |
| `prepress::validate_font_size([min])` | 最小サイズ（既定 6 pt）未満が無ければ真 |

```pdfl
check "Fonts" {
  print("fonts:", prepress::list_fonts().join(", "))

  missing = prepress::detect_text_substitution()
  assert missing.length == 0,
    "fonts not embedded (text will change at the RIP): #{missing.join(", ")}"

  problems = prepress::detect_missing_glyphs()
  assert problems.length == 0,
    "fonts without a widths table: #{problems.join(", ")}"

  assert prepress::subset_fonts(),
    "a full font is embedded — the file is larger than it needs to be"

  risky = prepress::check_font_licensing()
  assert risky.length == 0, "fonts with licensing risk: #{risky.join(", ")}"

  // 添付文書や契約書には最小文字サイズの規制があります
  assert prepress::validate_font_size(6),
    "there is text below 6 pt — illegible once printed"
}
```

---

## 6.5 ページとボックス

PDF のボックスは作業領域を定義します：**MediaBox**（用紙）、**BleedBox**
（塗り足し）、**TrimBox**（仕上がり）、**CropBox**（表示）、**ArtBox**（内容）。

| 関数 | 動作 |
|---|---|
| `prepress::get_page_size([page])` | `[幅, 高さ]`（ポイント） |
| `prepress::get_page_boxes([page])` | 定義されているボックスの一覧 |
| `prepress::validate_media_box()` | 全ページに MediaBox があれば真 |
| `prepress::validate_trim_box()` | 全ページに TrimBox があれば真 |
| `prepress::validate_bleed_box()` | 全ページに BleedBox があれば真 |
| `prepress::check_page_geometry([margin])` | 塗り足しが四方とも指定量以上なら真（既定 3mm） |

```pdfl
check "Geometry" {
  size = prepress::get_page_size(1)
  assert abs(size.first() - 595.0) < 5, "width is outside A4"
  prepress::get_page_boxes(1).each { |box| print(box) }

  assert prepress::validate_trim_box(),
    "no TrimBox — the shop cannot know where to trim"
  assert prepress::validate_bleed_box(),
    "no BleedBox — no bleed area is defined"

  // 単位リテラルを使うと読みやすく、変換も自動です
  assert prepress::check_page_geometry(3mm),
    "bleed smaller than 3 mm on some page"
}
```

---

## 6.6 完全な例

```pdfl
// offset_magazine.pdfl — オフセット印刷の完全なプリフライト
// 使い方: pdfl run offset_magazine.pdfl magazine.pdf --output html --output-file report.html
profile "offset-magazine" {

  const TAC_LIMIT = 300%
  const BLEED = 3mm
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress", "colors"] {
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMIT,
        "page #{page.number}: #{tac}% ink (limit #{TAC_LIMIT}%)"
    }
    print("average coverage:", prepress::calculate_ink_coverage(), "%")
  }

  check "Colors" tags: ["prepress", "colors"] {
    assert prepress::detect_color_mode() != "RGB", "document is in RGB"
    spots = prepress::detect_spot_colors()
    assert spots.length == 0, "unquoted special ink: #{spots.join(", ")}"
    assert !prepress::detect_rich_black(), "rich black in text"
    assert prepress::validate_output_intent(), "no Output Intent"
  }

  check "Fonts" tags: ["fonts"] {
    missing = prepress::detect_text_substitution()
    assert missing.length == 0, "fonts not embedded: #{missing.join(", ")}"
    assert prepress::validate_font_size(6), "text below 6 pt"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25), "strokes below 0.25 pt"
    assert !prepress::detect_hairlines_exact(), "stroke with 0 width"
  }

  check "Geometry" tags: ["prepress", "boxes"] {
    require prepress::validate_trim_box()
    require prepress::validate_bleed_box()
    assert prepress::check_page_geometry(BLEED), "bleed smaller than 3 mm"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI"
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number}"
    }
  }
}
```

---

[← `visual::`](05-visual.md) · [目次](README.md) · [次：`codes::` →](07-codes.md)
