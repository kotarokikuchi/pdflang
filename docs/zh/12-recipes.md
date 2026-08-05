# 12. 实用范例

[← 命令行](11-cli.md) · [目录](README.md)

可直接套用的完整案例，每一个都解决现场的真实问题。

---

## 12.1 印刷厂：胶印杂志的印前检查

**问题：** 客户交来文件，上版之前必须确认油墨、字体、图像和出血。事后才发现
的错误，会让整批印量报废。

`profiles/offset.pdfl`：

```pdfl
profile "offset-magazine" {

  const TAC_LIMIT = 300%       // 铜版纸的油墨上限
  const BLEED = 3mm            // 拼版要求
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress"] {
    // 准确的 TAC 读取文件中声明的颜色；基于渲染的估算会低估
    // 丰富黑，从而漏掉超标
    doc.pages.each { |page|
      tac = prepress::calculate_exact_tac(page.number)
      assert tac <= TAC_LIMIT,
        "page #{page.number}: #{tac}% ink (limit #{TAC_LIMIT}%)"
    }
  }

  check "Colors" tags: ["prepress"] {
    assert prepress::detect_color_mode() != "RGB",
      "document is in RGB — convert to CMYK"

    spots = prepress::detect_spot_colors()
    assert spots.length == 0, "unquoted special ink: #{spots.join(", ")}"

    assert !prepress::detect_rich_black(),
      "rich black detected — use 0/0/0/100 for text"
  }

  check "Fonts" tags: ["fonts"] {
    loose = prepress::detect_text_substitution()
    assert loose.length == 0,
      "fonts not embedded (text will change at the RIP): #{loose.join(", ")}"
    assert prepress::validate_font_size(6),
      "there is text below 6 pt — illegible once printed"
  }

  check "Strokes" tags: ["prepress"] {
    assert !prepress::detect_hairlines(0.25),
      "strokes below 0.25 pt disappear in print"
    assert !prepress::detect_hairlines_exact(),
      "there is a stroke with 0 width — set a real thickness"
  }

  check "Images" tags: ["images"] {
    doc.images.each { |img|
      assert img.dpi >= MIN_DPI,
        "image on page #{img.page_number}: #{round(img.dpi)} DPI (minimum #{MIN_DPI})"
      assert img.color_space != "DeviceRGB",
        "RGB image on page #{img.page_number}"
    }
  }

  check "Geometry" tags: ["prepress"] {
    assert prepress::validate_trim_box(),
      "no TrimBox — imposition cannot know where to trim"
    assert prepress::validate_bleed_box(), "no BleedBox — no bleed is defined"
    assert prepress::check_page_geometry(BLEED),
      "bleed smaller than 3 mm on some page"
  }
}
```

**在业务前台：**

```bash
# 返还给客户的 HTML 报告
pdfl run profiles/offset.pdfl client.pdf --output html --output-file report.html
```

**作为监视文件夹：** 操作员把文件放进去，报告就出现在旁边。

```bash
pdfl watch inbox/ --script profiles/offset.pdfl \
  --output-dir reports/ --report html
```

---

## 12.2 法律出版社：发布前的合同检查

**问题：** 合同和保单必须包含必备条款，不能残留草稿文字，不能泄露个人信息，
文本还必须可检索。

`profiles/legal.pdfl`：

```pdfl
profile "standard-contract" {

  check "Mandatory clauses" tags: ["legal"] {
    // 由法务部门维护的术语表
    missing = data::validate_against_reference("terms/clauses.txt")
    assert missing.length == 0, "missing clauses: #{missing.join("; ")}"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "document marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("X{3,}"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    // 纳税人号码只在校验位正确时才被检出，
    // 因此示例号码不会误报
    found = text::detect_personal_data()
    assert found.length == 0, "personal data in the document: #{found.join("; ")}"
  }

  check "Numbering and initials" tags: ["legal"] {
    doc.pages.each { |page|
      footer = region(0, 0, page.width, 60, "footer")
      content = text::extract_from_region(page.number, footer).trim()
      assert content != "",
        "page #{page.number} has no numbering/initials in the footer"
    }
  }

  check "Searchable text" tags: ["accessibility"] {
    assert !text::detect_rasterized_text(),
      "there are scanned pages — text cannot be searched or read by screen readers"
  }
}
```

---

## 12.3 制药实验室：带批号的说明书

**问题：** 说明书必须包含监管机构要求的文字，条码必须指向正确的产品。在产品
之间弄错条码，是这个行业代价最高的错误。

`profiles/insert.pdfl`：

```pdfl
profile "regulated-insert" {

  check "Mandatory texts" tags: ["regulatory"] {
    missing = data::validate_against_reference("databases/regulatory_texts.txt")
    assert missing.length == 0, "mandatory texts missing: #{missing.join("; ")}"
  }

  check "Legibility" tags: ["regulatory"] {
    assert prepress::validate_font_size(6), "there is text below 6 pt"
  }

  check "Barcode" tags: ["codes", "critical"] {
    assert codes::detect_barcodes(), "insert has no barcode"

    code = codes::decode_barcode(1)
    assert codes::validate_barcode_checksum(1), "invalid check digit: #{code}"

    // 这条检查抓住代价最高的错误：某产品的条码配另一产品的文字
    assert codes::compare_barcode_with_text(),
      "the code number does not appear in the insert text"
  }

  check "Approved product" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} is not in the product database"

    name = product.get(2)
    assert text::require_text(name),
      "the name '#{name}' does not appear on the insert"
    print("product verified:", name)
  }

  check "Code position" tags: ["layout"] {
    area = region(400, 20, 180, 90, "barcode area")
    assert codes::validate_barcode_position(area),
      "code outside the reserved area — risk of being trimmed off"
  }
}
```

```bash
PDFL_DATA_DIR=./databases pdfl run profiles/insert.pdfl insert_v3.pdf
```

---

## 12.4 审批：与已批准版本比较

**问题：** 客户批准了 v1；v2 送来时说「只改了一个词」。轻信的代价很高。

```bash
# 生成 HTML，展示实际改动
pdfl compare approved/catalogue_v1.pdf received/catalogue_v2.pdf \
  --normalize --ignore-dates \
  --output html --output-file differences.html

echo "exit: $?"   # 0 完全相同 · 1 仅元数据 · 2 内容有变
```

若还要确认**外观**（而不只是文字）：

```pdfl
// profiles/fidelity.pdfl
profile "visual-fidelity" {

  const APPROVED = "approved/catalogue_v1.pdf"

  check "Pages visually identical" tags: ["approval"] {
    doc.pages.each { |page|
      ssim = visual::measure_ssim(page.number, APPROVED)
      assert ssim > 0.99,
        "page #{page.number} changed visually (SSIM #{ssim}, #{visual::pixel_diff(page.number, APPROVED)}% of pixels)"
    }
  }

  check "No image replaced" tags: ["approval"] {
    doc.pages.each { |page|
      assert !visual::detect_image_replacement(page.number, APPROVED),
        "page #{page.number}: image swapped compared to the approved version"
    }
  }
}
```

---

## 12.5 CI/CD：批量校验

**问题：** 进入仓库的每个文件都必须通过印前检查，且无需任何人手动运行。

`.github/workflows/preflight.yml`：

```yaml
name: PDF preflight

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install pdfl
        run: |
          curl -sSL -o pdfl-linux-x64.tar.gz \
            https://github.com/kotarokikuchi/pdflang/releases/latest/download/pdfl-linux-x64.tar.gz
          tar xzf pdfl-linux-x64.tar.gz
          echo "$PWD/pdfl" >> $GITHUB_PATH

      - name: Check the scripts themselves
        run: |
          for f in profiles/*.pdfl; do
            pdfl lint "$f"
            pdfl fmt "$f" --check
          done

      - name: Preflight every PDF
        run: |
          pdfl watch files/ --script profiles/offset.pdfl \
            --output-dir reports/ --once

      - name: Publish the reports
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: reports
          path: reports/
```

---

## 12.6 把出版社的文件整理为送印版本

```pdfl
// profiles/prepare.pdfl
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask for the open version"
}

// 出版社未设置的制作用页面框
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// 清理
fix::remove_annotations()      // 校对批注
fix::remove_attachments()      // 只会让文件变大的附件
fix::flatten_layers()          // 防止图层被误开启
fix::remove_unused_resources()
```

```bash
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf --dry-run  # 确认
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf            # 执行
pdfl run profiles/offset.pdfl print.pdf                                    # 校验
```

---

## 12.7 向团队分发配置

**问题：** 五台机器要使用完全相同的配置和数据，并确保无人改动。

```bash
# 在维护配置的机器上
pdfl pack profiles/ --name print-profile --version 1.2.0

# 在生产机器上
pdfl add print-profile.pdflpkg
# 安装到 ./pdfl_profiles/print-profile@1.2.0/，逐一校验哈希

pdfl run pdfl_profiles/print-profile@1.2.0/offset.pdfl file.pdf
```

若软件包在传输途中被改动，`add` 会**拒绝安装**。

---

## 12.8 排查有问题的文件

不清楚问题出在哪里时的实用步骤：

```bash
# 1. 几秒钟掌握全貌
pdfl inspect suspect.pdf

# 2. 只用 print() 的调查脚本
cat > investigate.pdfl <<'EOF'
check "X-ray" {
  print("exact TAC:", prepress::calculate_exact_tac(), "%")
  print("estimated TAC:", prepress::calculate_tac(), "%")
  print("spots:", prepress::detect_spot_colors().join(", "))
  print("rich black?", prepress::detect_rich_black())
  print("overprint ok?", prepress::validate_overprint_settings())
  print("loose fonts:", prepress::detect_text_substitution().join(", "))

  doc.images.each { |img|
    print("image page", img.page_number, ":", img.width, "x", img.height,
          "@", round(img.dpi), "DPI", img.color_space)
  }
}
EOF

pdfl run investigate.pdfl suspect.pdf > /dev/null
# print() 输出到标准错误，因此可以丢弃报告，只看调查结果
```

---

[← 命令行](11-cli.md) · [目录](README.md)
