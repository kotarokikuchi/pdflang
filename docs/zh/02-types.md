# 2. 文档类型

[← 语言](01-language.md) · [目录](README.md) · [下一章：`text::` →](03-text.md)

每个脚本都会自动获得 `doc` 变量，代表正在分析的 PDF。从它可以访问页面、
字体和图像。

---

## 2.1 `doc` — 文档

| 属性 | 类型 | 含义 |
|---|---|---|
| `doc.page_count` | 数字 | 页数 |
| `doc.title` | 文本 | 元数据中的标题（缺失时为空） |
| `doc.author` | 文本 | 元数据中的作者（缺失时为空） |
| `doc.filename` | 文本 | 被分析文件的名称 |
| `doc.pages` | 列表 | 所有页面 |
| `doc.fonts` | 列表 | 使用的所有字体 |
| `doc.images` | 列表 | 所有页面上的全部图像 |

方法：`doc.extract_text()` — 整个文档的文本，页面之间以换行分隔。

```pdfl
check "Document properties" {
  print("file:", doc.filename)
  print("pages:", doc.page_count)

  // 这些集合就是普通列表 — 所有列表方法都适用
  require doc.pages.length == doc.page_count
  require doc.fonts.length > 0

  text = doc.extract_text()
  assert text.trim() != "", "PDF has no extractable text (images only?)"
  print("total characters:", text.length)
}
```

---

## 2.2 `page` — 页面

页面来自 `doc.pages`（在块中）或 `page` 变量（在 `rule` 中）。

| 属性 | 类型 | 含义 |
|---|---|---|
| `page.number` | 数字 | 页码，从 **1** 开始 |
| `page.index` | 数字 | 索引，从 **0** 开始 |
| `page.width` / `page.height` | 数字 | 宽 / 高（点） |
| `page.images` | 列表 | 本页的图像 |
| `page.tac` | 数字 | 估算的最大油墨总量（%） |
| `page.ink_coverage` | 数字 | 估算的平均油墨量（%） |
| `page.min_stroke_width` | 数字/null | 最细线宽（pt）；没有线条时为 `null` |
| `page.has_media_box` 等 | 布尔 | `has_crop_box`、`has_trim_box`、`has_bleed_box`、`has_art_box` |

方法：`page.extract_text()` — 仅本页的文本。

```pdfl
check "Page format" {
  doc.pages.each { |page|
    // number 是人看的页码，index 用于内部计算
    assert page.width > 100mm,
      "page #{page.number} is too narrow: #{page.width}pt"

    // 页面框：印刷必需
    assert page.has_trim_box, "page #{page.number} has no TrimBox (trim area)"
    assert page.has_bleed_box, "page #{page.number} has no BleedBox (bleed area)"

    assert page.tac <= 300,
      "page #{page.number}: #{page.tac}% ink (limit 300%)"

    // min_stroke_width 可能为 null（本页没有线条）。
    // null 为假，所以这样写是安全的：
    assert !page.min_stroke_width || page.min_stroke_width >= 0.25,
      "page #{page.number} has a hairline stroke"
  }
}

check "Blank pages" {
  blank = doc.pages.filter { |p| p.extract_text().trim() == "" }
  assert blank.length == 0,
    "#{blank.length} blank page(s): #{blank.map { |p| p.number }.join(", ")}"
}
```

---

## 2.3 `font` — 字体

来自 `doc.fonts`。属性：`font.name`（名称）、`font.is_embedded`（是否嵌入）。

```pdfl
check "Embedded fonts" {
  // 未嵌入的字体会被阅读器替换 — 文字外观随之改变
  doc.fonts.each { |font|
    assert font.is_embedded,
      "font '#{font.name}' is not embedded in the PDF"
  }
  print("fonts in use:", doc.fonts.map { |f| f.name }.join(", "))
}
```

---

## 2.4 `image` — 图像

来自 `doc.images`（全部）或 `page.images`（单页）。

| 属性 | 含义 |
|---|---|
| `image.width` / `image.height` | 宽 / 高（**像素**） |
| `image.dpi` | 有效分辨率（dpi_x 与 dpi_y 中较小者） |
| `image.dpi_x` / `image.dpi_y` | 水平 / 垂直有效分辨率 |
| `image.color_space` | `DeviceRGB`、`DeviceCMYK`、`Indexed`…… |
| `image.page_number` | 所在页码（从 1 开始） |
| `image.bits_per_pixel` | 位深 |

> **DPI 是有效值**，按「像素数 ÷ 页面上的印刷尺寸」计算，而不是元数据里的
> 标称值。这才是影响印刷质量的数字：一张 1000 px 的图被拉伸到 20 cm，
> 无论元数据怎么写，DPI 都很低。

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
    // 胶印使用 CMYK；RGB 需要转换
    doc.images.each { |img|
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number} — convert to CMYK"
    }
  }

  check "Images per page" {
    doc.pages.each { |page|
      print("page", page.number, "has", page.images.length, "image(s)")
    }
  }
}
```

---

## 2.5 `region` — 页面区域

区域用矩形界定页面的一部分，便于校验页脚、页眉、条码区、药品警示带等。

创建：`region(x, y, 宽, 高 [, "名称"])`，原点 (0,0) 与 PDF 一致，位于左下角。

| 属性 | 含义 | | 方法 | 功能 |
|---|---|---|---|---|
| `region.name` | 创建时给的名称 | | `contains_point(x, y)` | 点是否在内部 |
| `region.x` / `region.y` | 左下角坐标 | | `intersects(other)` | 两个区域是否重叠 |
| `region.width` / `region.height` | 尺寸 | | `expand(pt)` | 各边扩大后的新区域 |
| `region.right` / `region.top` | 右边 / 上边（计算值） | | `inset(pt)` | 各边缩小后的新区域 |
| `region.area` | 面积（平方点） | | `export_coordinates()` | `[x0, y0, x1, y1]` |

```pdfl
check "Working with regions" {
  footer = region(0, 0, 595, 60, "footer")

  require footer.name == "footer"
  require footer.top == 60.0
  require footer.right == 595.0
  require footer.area == 35700.0
  require footer.contains_point(300, 30)
  require !footer.contains_point(300, 500)

  // 重叠检测：可用于发现元素侵入保留区
  header = region(0, 780, 595, 62)
  require !footer.intersects(header)

  // expand/inset 返回「新的」区域（原区域不变）
  require footer.expand(5mm).area > footer.area
  require footer.inset(3mm).area < footer.area
}

profile "medicine-label" {
  check "Prescription band" {
    // 警示带必须位于上方并包含法定文字
    band = region(0, 700, 595, 142, "band")
    assert text::extract_from_region(1, band).contains("PRESCRIPTION ONLY"),
      "band is missing the mandatory text"
  }

  check "Ink in the fold area" {
    // 折线处油墨过多会在加工时开裂
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

[← 语言](01-language.md) · [目录](README.md) · [下一章：`text::` →](03-text.md)
