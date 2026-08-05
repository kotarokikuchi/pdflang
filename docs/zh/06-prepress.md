# 6. `prepress::` 命名空间 — 印前检查

[← `visual::`](05-visual.md) · [目录](README.md) · [下一章：`codes::` →](07-codes.md)

涵盖印刷厂在上版前必须确认事项的 30 个函数：油墨总量、分色、字体、线宽、
页面框。

---

## 6.1 油墨总量（TAC）

TAC（Total Area Coverage）是某一点上四色油墨的总和。超过印刷机上限会导致
蹭脏、干燥不良和背面粘脏。铜版纸胶印通常以 300% 为上限。

测量方式有**两种**，差别很关键。

| 函数 | 功能 |
|---|---|
| `prepress::calculate_exact_tac([page])` | 依据文件中**声明的颜色**计算（准确） |
| `prepress::calculate_tac([page])` | 通过 RGB 渲染估算（**下界**） |
| `prepress::validate_tac_limits([limit])` | 所有页面均在上限内则为真（默认 300） |
| `prepress::calculate_ink_coverage([page])` | 平均油墨量（%） |
| `prepress::calculate_tac_by_region(page, region)` | 区域内的 `[最大TAC, 平均值]` |

估算值会把深色中性灰（丰富黑）压缩到 100% 附近。

```pdfl
check "Ink limit" {
  // 校验上限时始终使用「准确的 TAC」
  doc.pages.each { |page|
    tac = prepress::calculate_exact_tac(page.number)
    assert tac <= 300, "page #{page.number}: #{tac}% ink"
  }

  print("exact (declared in the file):", prepress::calculate_exact_tac(), "%")
  print("estimated (by rendering):", prepress::calculate_tac(), "%")
  // 真实文件实测：准确值 324%，估算值 299%
  // — 只有准确值才能发现超标。

  // 折线处油墨过多会在加工时开裂
  fold = region(290, 0, 15, 842, "center fold")
  measured = prepress::calculate_tac_by_region(1, fold)
  assert measured.first() < 240, "TAC of #{measured.first()}% on the fold (max 240%)"
}
```

---

## 6.2 颜色与分色

| 函数 | 功能 |
|---|---|
| `prepress::detect_spot_colors()` | 专色油墨列表（Separation / DeviceN） |
| `prepress::detect_color_mode()` | `"CMYK"` / `"RGB"` / `"Mixed"` / `"None"` / `"Other"` |
| `prepress::validate_color_space(space)` | 所有图像都在指定色彩空间则为真 |
| `prepress::compare_colors_delta_e(a, b)` | 两色的 Delta-E（CIE76） |
| `prepress::detect_rich_black()` | 存在多色叠印的黑则为真 |
| `prepress::validate_overprint_settings()` | 未启用叠印则为真 |
| `prepress::validate_output_intent([name])` | 是否声明输出意图 / 名称是否匹配 |
| `prepress::check_rendering_intent([expected])` | 列出或校验渲染意图 |

颜色以列表传入：4 个值 = CMYK，3 个 = RGB，1 个 = 灰度。Delta-E 的经验值：
小于 1 无法察觉，3 以内印刷可接受，大于 5 明显不同。

> 保留的分色 `All` 和 `None` 不会被列出：`All` 是套准标记而非油墨。

```pdfl
check "Colors" {
  spots = prepress::detect_spot_colors()
  assert spots.length == 0, "file uses an unquoted special ink: #{spots.join(", ")}"

  mode = prepress::detect_color_mode()
  assert mode == "CMYK" || mode == "None",
    "document is #{mode} — offset printing requires CMYK"

  // 品牌色的允差
  difference = prepress::compare_colors_delta_e([1.0, 0.6, 0.0, 0.1], [1.0, 0.62, 0.0, 0.12])
  assert difference < 3.0, "brand color out of tolerance (ΔE #{difference})"

  // 小字用丰富黑会让套印不准更明显
  assert !prepress::detect_rich_black(),
    "rich black found — use flat black (0/0/0/100) for text"

  // 意外的叠印会让元素消失
  assert prepress::validate_overprint_settings(),
    "overprint is enabled — verify that it is intentional"

  assert prepress::validate_output_intent(),
    "PDF has no Output Intent — the shop cannot know the target color profile"
}
```

---

## 6.3 线宽

| 函数 | 功能 |
|---|---|
| `prepress::detect_hairlines([limit])` | 存在低于阈值（默认 0.25 pt）的线则为真 |
| `prepress::detect_hairlines_exact()` | 存在线宽为 0 的线则为真 |
| `prepress::detect_fine_lines([limit])` | 同上（默认 1 pt） |
| `prepress::validate_minimum_stroke_width(min)` | 所有线均不低于最小值则为真 |

线宽 0 是 PostScript 的经典发丝线，设备会以最小可能宽度渲染（不可预测）。

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

## 6.4 字体

| 函数 | 功能 |
|---|---|
| `prepress::list_fonts()` | 使用的字体名称列表 |
| `prepress::validate_font_embedding()` | 全部字体均已嵌入则为真 |
| `prepress::detect_text_substitution()` | 未嵌入的字体列表 |
| `prepress::detect_missing_glyphs()` | 缺少宽度表的字体列表 |
| `prepress::subset_fonts()` | 嵌入字体全部为子集则为真 |
| `prepress::check_font_licensing()` | 有授权风险的字体（Type3 或未嵌入） |
| `prepress::validate_font_size([min])` | 没有低于最小字号（默认 6 pt）的文字则为真 |

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

  // 说明书和合同对最小字号有法规要求
  assert prepress::validate_font_size(6),
    "there is text below 6 pt — illegible once printed"
}
```

---

## 6.5 页面与页面框

PDF 的页面框定义了工作区域：**MediaBox**（纸张）、**BleedBox**（出血）、
**TrimBox**（成品）、**CropBox**（显示）、**ArtBox**（内容）。

| 函数 | 功能 |
|---|---|
| `prepress::get_page_size([page])` | `[宽, 高]`（点） |
| `prepress::get_page_boxes([page])` | 已定义页面框的列表 |
| `prepress::validate_media_box()` | 所有页面都有 MediaBox 则为真 |
| `prepress::validate_trim_box()` | 所有页面都有 TrimBox 则为真 |
| `prepress::validate_bleed_box()` | 所有页面都有 BleedBox 则为真 |
| `prepress::check_page_geometry([margin])` | 四周出血均达到指定量则为真（默认 3mm） |

```pdfl
check "Geometry" {
  size = prepress::get_page_size(1)
  assert abs(size.first() - 595.0) < 5, "width is outside A4"
  prepress::get_page_boxes(1).each { |box| print(box) }

  assert prepress::validate_trim_box(),
    "no TrimBox — the shop cannot know where to trim"
  assert prepress::validate_bleed_box(),
    "no BleedBox — no bleed area is defined"

  // 使用单位字面量既好读，换算也自动完成
  assert prepress::check_page_geometry(3mm),
    "bleed smaller than 3 mm on some page"
}
```

---

## 6.6 完整示例

```pdfl
// offset_magazine.pdfl — 胶印的完整印前检查
// 用法: pdfl run offset_magazine.pdfl magazine.pdf --output html --output-file report.html
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
      assert img.color_space != "DeviceRGB", "RGB image on page #{img.page_number}"
    }
  }
}
```

---

[← `visual::`](05-visual.md) · [目录](README.md) · [下一章：`codes::` →](07-codes.md)
