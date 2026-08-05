# 5. `visual::` 命名空间 — 图像与视觉比对

[← `struct::`](04-struct.md) · [目录](README.md) · [下一章：`prepress::` →](06-prepress.md)

处理文档图像和页面渲染外观的 16 个函数。

> 比对与质量类函数会把页面**渲染为灰度图**。每一页只渲染一次并被缓存。

---

## 5.1 图像清单

| 函数 | 功能 |
|---|---|
| `visual::detect_images()` | 存在图像则为真 |
| `visual::count_images()` | 图像总数 |
| `visual::get_image_resolution(n)` | 第 n 张图的有效 DPI（从 1 开始） |
| `visual::get_image_size(n)` | 像素尺寸 `[宽, 高]` |
| `visual::detect_image_color_space([n])` | 色彩空间列表，或第 n 张图的色彩空间 |
| `visual::detect_low_resolution([min_dpi])` | 存在低于最低 DPI（默认 300）的图则为真 |

```pdfl
check "Image inventory" {
  require visual::detect_images()
  print("total images:", visual::count_images())
  print("spaces present:", visual::detect_image_color_space().join(", "))

  // 胶印要求全部为 CMYK
  assert !visual::detect_image_color_space().contains("DeviceRGB"),
    "there are RGB images — convert to CMYK before printing"

  assert !visual::detect_low_resolution(300),
    "there are images below 300 DPI"
}
```

> 想知道**具体哪些**图像有问题，请遍历 `doc.images` — 见[第 2 章](02-types.md)：
>
> ```pdfl
> doc.images.each { |img|
>   assert img.dpi >= 300, "image on page #{img.page_number}: #{img.dpi} DPI"
> }
> ```

---

## 5.2 文件之间的视觉比对

这些函数将本文档的某一页与**另一个文件**的某一页比较。通用签名为：

```
function(page_here, "other.pdf" [, page_there])
```

省略对方页码时使用相同页码。尺寸不同的页面会先重采样再比较。

| 函数 | 功能 |
|---|---|
| `visual::measure_ssim(page, "other.pdf" [, page_b])` | 结构相似度（0.0–1.0） |
| `visual::compare_images(...)` / `visual::diff_pages(...)` | 同样的比较，取值 0–100 |
| `visual::pixel_diff(page, "other.pdf" [, page_b, tolerance])` | 不同像素的百分比 |
| `visual::calculate_perceptual_hash([page])` | 64 位 pHash（十六进制） |
| `visual::detect_image_replacement(page, "other.pdf" [, page_b, distance])` | 变化超出容差则为真 |

```pdfl
check "Approved proof vs final file" {
  approved = "approved/magazine_v1.pdf"

  doc.pages.each { |page|
    ssim = visual::measure_ssim(page.number, approved)
    assert ssim > 0.99,
      "page #{page.number} changed visually (SSIM #{ssim})"

    // 提高容差可以忽略抗锯齿差异
    smooth = visual::pixel_diff(page.number, approved, page.number, 30)
    assert smooth < 1.0, "significant change on page #{page.number}"

    assert !visual::detect_image_replacement(page.number, approved),
      "page #{page.number}: visual content was replaced"
  }
}
```

---

## 5.3 图像质量

| 函数 | 功能 |
|---|---|
| `visual::detect_image_artifacts([page])` | 存在 JPEG 块状伪影则为真 |
| `visual::estimate_image_quality([page])` | 由块状程度换算的 0–100 评分 |
| `visual::detect_posterization([page])` | 色阶级数不足则为真 |
| `visual::detect_banding([page])` | 渐变出现台阶则为真 |

> banding 的判定要求单调变化且有较宽的平台区，因此变化剧烈的普通文字页
> **不会误报**。

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

## 5.4 完整示例

```pdfl
// visual_approval.pdfl — 与已批准版本比对
// 用法: pdfl run visual_approval.pdfl new_version.pdf
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
      assert !visual::detect_banding(page.number), "page #{page.number} shows banding"
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

[← `struct::`](04-struct.md) · [目录](README.md) · [下一章：`prepress::` →](06-prepress.md)
