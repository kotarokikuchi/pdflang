# 5. `visual::` 名前空間 — 画像と視覚比較

[← `struct::`](04-struct.md) · [目次](README.md) · [次：`prepress::` →](06-prepress.md)

ドキュメントの画像と、レンダリングされたページの見た目を扱う16の関数。

> 比較と品質の関数はページを**グレースケールでレンダリング**します。各ページ
> は一度だけレンダリングされ、キャッシュされます。

---

## 5.1 画像の一覧

| 関数 | 動作 |
|---|---|
| `visual::detect_images()` | 画像があれば真 |
| `visual::count_images()` | 画像の総数 |
| `visual::get_image_resolution(n)` | n番目の画像の実効 DPI（1 起点） |
| `visual::get_image_size(n)` | ピクセル寸法 `[幅, 高さ]` |
| `visual::detect_image_color_space([n])` | 色空間の一覧、または n番目の色空間 |
| `visual::detect_low_resolution([min_dpi])` | 最低 DPI（既定 300）未満があれば真 |

```pdfl
check "Image inventory" {
  require visual::detect_images()
  print("total images:", visual::count_images())
  print("spaces present:", visual::detect_image_color_space().join(", "))

  // オフセット印刷ではすべて CMYK であるべきです
  assert !visual::detect_image_color_space().contains("DeviceRGB"),
    "there are RGB images — convert to CMYK before printing"

  assert !visual::detect_low_resolution(300),
    "there are images below 300 DPI"
}
```

> **どの**画像が問題なのかを知るには `doc.images` を回します —
> [第2章](02-types.md#24-image--画像)を参照：
>
> ```pdfl
> doc.images.each { |img|
>   assert img.dpi >= 300,
>     "image on page #{img.page_number}: #{img.dpi} DPI"
> }
> ```

---

## 5.2 ファイル間の視覚比較

これらの関数は、このドキュメントのページを**別のファイル**のページと比較
します。共通のシグネチャは次のとおりです：

```
function(page_here, "other.pdf" [, page_there])
```

相手のページ番号を省略すると同じ番号を使います。サイズが違うページは比較前に
リサンプリングされます。

| 関数 | 動作 |
|---|---|
| `visual::measure_ssim(page, "other.pdf" [, page_b])` | 構造的類似度（0.0〜1.0） |
| `visual::compare_images(...)` / `visual::diff_pages(...)` | 同じ比較を 0〜100 で |
| `visual::pixel_diff(page, "other.pdf" [, page_b, tolerance])` | 異なるピクセルの割合（%） |
| `visual::calculate_perceptual_hash([page])` | pHash 64ビット（16進） |
| `visual::detect_image_replacement(page, "other.pdf" [, page_b, distance])` | 許容を超えて変化していれば真 |

```pdfl
check "Approved proof vs final file" {
  approved = "approved/magazine_v1.pdf"

  doc.pages.each { |page|
    ssim = visual::measure_ssim(page.number, approved)
    assert ssim > 0.99,
      "page #{page.number} changed visually (SSIM #{ssim})"

    // アンチエイリアスを無視するには許容値を上げます
    smooth = visual::pixel_diff(page.number, approved, page.number, 30)
    assert smooth < 1.0, "significant change on page #{page.number}"

    assert !visual::detect_image_replacement(page.number, approved),
      "page #{page.number}: visual content was replaced"
  }
}
```

---

## 5.3 画像品質

| 関数 | 動作 |
|---|---|
| `visual::detect_image_artifacts([page])` | JPEG 特有のブロックノイズがあれば真 |
| `visual::estimate_image_quality([page])` | ブロックノイズから算出した 0〜100 の評価 |
| `visual::detect_posterization([page])` | 階調段数が不足していれば真 |
| `visual::detect_banding([page])` | グラデーションに段差があれば真 |

> banding の検出には、単調な変化と広い平坦部が必要です。急な変化が多い通常の
> テキストページは**誤検出しません**。

```pdfl
check "Image quality" {
  doc.pages.each { |page|
    assert !visual::detect_image_artifacts(page.number),
      "page #{page.number} shows visible compression blockiness"

    score = visual::estimate_image_quality(page.number)
    assert score >= 70,
      "page #{page.number} scores #{score}/100 — recompressed too hard?"

    assert !visual::detect_posterization(page.number),
      "page #{page.number}: possible posterization (too few tones)"
    assert !visual::detect_banding(page.number),
      "page #{page.number} shows banding in a gradient"
  }
}
```

---

## 5.4 完全な例

```pdfl
// visual_approval.pdfl — 承認版との比較
// 使い方: pdfl run visual_approval.pdfl new_version.pdf
profile "visual-approval" {

  const APPROVED = "approved/catalogue_v1.pdf"
  const MIN_DPI = 300

  check "Inventory" tags: ["images"] {
    require visual::detect_images()
    print("images:", visual::count_images())
    print("color spaces:", visual::detect_image_color_space().join(", "))
  }

  check "Resolution" tags: ["images", "prepress"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI (minimum #{MIN_DPI})"
    }
  }

  check "Quality" tags: ["images"] {
    doc.pages.each { |page|
      assert !visual::detect_image_artifacts(page.number),
        "page #{page.number} has compression artifacts"
      assert !visual::detect_banding(page.number),
        "page #{page.number} shows banding"
    }
  }

  check "Fidelity to the approved version" tags: ["approval"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APPROVED)
      assert ssim > 0.99,
        "page #{page.number} differs from the approved one (SSIM #{ssim}, #{visual::pixel_diff(page.number, APPROVED)}% of pixels)"
    }
  }
}
```

---

[← `struct::`](04-struct.md) · [目次](README.md) · [次：`prepress::` →](06-prepress.md)
