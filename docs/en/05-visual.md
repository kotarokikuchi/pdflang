# 5. `visual::` namespace — images and visual comparison

[← `struct::`](04-struct.md) · [Index](README.md) · [Next: `prepress::` →](06-prepress.md)

16 functions covering the document's images and the rendered appearance of its
pages.

> Comparison and quality functions **render the page** in grayscale. Each page is
> rendered once and cached.

---

## 5.1 Image inventory

### `visual::detect_images()` and `visual::count_images()`

```pdfl
check "Images in the document" {
  require visual::detect_images()
  print("total images:", visual::count_images())

  // A catalogue with no images is probably wrong
  assert visual::count_images() >= 10,
    "catalogue has only #{visual::count_images()} image(s)"
}
```

### `visual::get_image_resolution(n)`

Effective DPI of the nth image (1-based).

```pdfl
check "Cover image resolution" {
  dpi = visual::get_image_resolution(1)
  assert dpi >= 300, "cover image is #{round(dpi)} DPI (minimum 300)"
}
```

### `visual::get_image_size(n)`

Dimensions in pixels: `[width, height]`.

```pdfl
check "Image size" {
  size = visual::get_image_size(1)
  print("first image:", size.first(), "x", size.last(), "pixels")
  require size.first() >= 1000
}
```

### `visual::detect_image_color_space([n])`

Without an argument: the list of color spaces present in the document.
With `n`: the color space of the nth image.

```pdfl
check "Color spaces in use" {
  spaces = visual::detect_image_color_space()
  print("spaces present:", spaces.join(", "))

  // For offset, everything should be CMYK
  assert !spaces.contains("DeviceRGB"),
    "there are RGB images — convert to CMYK before printing"

  // Checking one specific image
  require visual::detect_image_color_space(1) == "DeviceCMYK"
}
```

### `visual::detect_low_resolution([min_dpi])`

True when **any** image falls below the minimum (default 300).

```pdfl
check "Overall resolution" {
  assert !visual::detect_low_resolution(300),
    "there are images below 300 DPI"

  // Large-format printing has a different threshold
  assert !visual::detect_low_resolution(150),
    "there are images below 150 DPI (banner minimum)"
}
```

> To find out **which** images are bad (not just whether any exist), iterate over
> `doc.images` — see [chapter 2](02-types.md#24-image--the-image):
>
> ```pdfl
> doc.images.each { |img|
>   assert img.dpi >= 300,
>     "image on page #{img.page_number}: #{img.dpi} DPI"
> }
> ```

---

## 5.2 Visual comparison between files

These functions compare a page of this document against a page of **another
file**. The general signature is:

```
function(page_here, "other.pdf" [, page_there])
```

If the other page number is omitted, the same number is used. Pages of different
sizes are resampled before comparison.

### `visual::measure_ssim(page, "other.pdf" [, page_b])`

Structural similarity, from `0.0` to `1.0`. This is the metric closest to the
human sense of "this is the same page".

```pdfl
check "Approved proof vs final file" {
  approved = "approved/magazine_v1.pdf"

  doc.pages.each { |page|
    ssim = visual::measure_ssim(page.number, approved)
    assert ssim > 0.99,
      "page #{page.number} changed visually (SSIM #{ssim})"
  }
}
```

### `visual::compare_images(...)` and `visual::diff_pages(...)`

The same comparison on a 0–100 scale. They are synonyms — use whichever name
reads better in your script.

```pdfl
check "Similarity as a percentage" {
  score = visual::diff_pages(1, "previous_version.pdf")
  assert score > 95, "cover changed by #{round(100 - score)}% since the last version"
}
```

### `visual::pixel_diff(page, "other.pdf" [, page_b, tolerance])`

Percentage of pixels that differ. Tolerance (default 10, on a 0–255 scale)
ignores negligible rendering variation.

```pdfl
check "How much of the page changed" {
  percent = visual::pixel_diff(4, "previous.pdf")
  print("pixels changed on page 4:", percent, "%")

  // Higher tolerance to ignore antialiasing
  smooth = visual::pixel_diff(4, "previous.pdf", 4, 30)
  assert smooth < 1.0, "significant change on page 4"
}
```

### `visual::calculate_perceptual_hash([page])`

Visual fingerprint of the page: 64 bits in hexadecimal. Similar pages produce
similar hashes.

```pdfl
check "Page fingerprints" {
  doc.pages.each { |page|
    print("page", page.number, "->", visual::calculate_perceptual_hash(page.number))
  }
}
```

### `visual::detect_image_replacement(page, "other.pdf" [, page_b, distance])`

True when the page changed visually beyond the tolerance. It compares perceptual
hashes; `distance` is how many bits may differ (default 10 of 64).

```pdfl
check "No image swapped between versions" {
  previous = "approved/catalogue_v1.pdf"

  doc.pages.each { |page|
    assert !visual::detect_image_replacement(page.number, previous),
      "page #{page.number}: visual content was replaced"
  }
}
```

---

## 5.3 Image quality

### `visual::detect_image_artifacts([page])`

True when the page shows blockiness typical of heavily compressed JPEG.

```pdfl
check "No compression artifacts" {
  doc.pages.each { |page|
    assert !visual::detect_image_artifacts(page.number),
      "page #{page.number} shows visible compression blockiness"
  }
}
```

### `visual::estimate_image_quality([page])`

A 0–100 score derived from the detected blockiness.

```pdfl
check "Quality score" {
  doc.pages.each { |page|
    score = visual::estimate_image_quality(page.number)
    assert score >= 70,
      "page #{page.number} scores #{score}/100 — recompressed too hard?"
  }
}
```

### `visual::detect_posterization([page])`

True when a page with a wide tonal range has too few distinct levels — a sign of
insufficient color depth.

```pdfl
check "Gradients intact" {
  doc.pages.each { |page|
    assert !visual::detect_posterization(page.number),
      "page #{page.number}: possible posterization (too few tones)"
  }
}
```

### `visual::detect_banding([page])`

True when a gradient shows visible steps instead of a smooth transition.

> Detection requires monotonic progression with wide plateaus — ordinary text
> pages, which have abrupt transitions, do **not** trigger it.

```pdfl
check "Gradients without banding" {
  // Gradient backgrounds are where banding shows up
  doc.pages.each { |page|
    assert !visual::detect_banding(page.number),
      "page #{page.number} shows banding in a gradient"
  }
}
```

---

## 5.4 Complete example

```pdfl
// visual_approval.pdfl — compares the file against the approved version
// Usage: pdfl run visual_approval.pdfl new_version.pdf
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

[← `struct::`](04-struct.md) · [Index](README.md) · [Next: `prepress::` →](06-prepress.md)
