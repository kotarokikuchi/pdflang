# 12. レシピ

[← CLI コマンド](11-cli.md) · [目次](README.md)

そのまま応用できる実例集。それぞれが現場の実際の課題を解決します。

---

## 12.1 印刷会社：オフセット雑誌のプリフライト

**課題：** 顧客からファイルが届き、刷版に回す前にインク・フォント・画像・
塗り足しを確認する必要があります。後で見つかった間違いは、その刷り全体の
損失になります。

`profiles/offset.pdfl`:

```pdfl
profile "offset-magazine" {

  const TAC_LIMIT = 300%       // コート紙のインク上限
  const BLEED = 3mm            // 面付けの要件
  const MIN_DPI = 300

  check "Ink coverage" tags: ["prepress"] {
    // 正確な TAC はファイルに宣言された色を読みます。レンダリングによる
    // 推定はリッチブラックを低く見積もり、超過を見逃します
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
    assert spots.length == 0,
      "unquoted special ink: #{spots.join(", ")}"

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
    assert prepress::validate_bleed_box(),
      "no BleedBox — no bleed is defined"
    assert prepress::check_page_geometry(BLEED),
      "bleed smaller than 3 mm on some page"
  }
}
```

**受付での使い方：**

```bash
# 顧客に返す HTML レポート
pdfl run profiles/offset.pdfl client.pdf --output html --output-file report.html
```

**ウォッチフォルダとして：** 担当者がフォルダに置くと、隣にレポートが出ます。

```bash
pdfl watch inbox/ --script profiles/offset.pdfl \
  --output-dir reports/ --report html
```

---

## 12.2 法務出版社：公開前の契約書チェック

**課題：** 契約書や約款には必須の条項があり、下書きの文言が残っていてはならず、
個人情報を露出してはならず、テキストは検索可能である必要があります。

`profiles/legal.pdfl`:

```pdfl
profile "standard-contract" {

  check "Mandatory clauses" tags: ["legal"] {
    // 法務部が管理する用語集
    missing = data::validate_against_reference("terms/clauses.txt")
    assert missing.length == 0, "missing clauses: #{missing.join("; ")}"
  }

  check "No drafts" tags: ["legal"] {
    assert text::forbid_text("DRAFT"), "document marked as draft"
    assert text::forbid_text("lorem ipsum"), "placeholder text present"
    assert text::forbid_match("X{3,}"), "unfilled fields (XXX)"
  }

  check "Privacy" tags: ["compliance"] {
    // 納税者番号はチェックディジットが正しい場合のみ検出されるため、
    // サンプル番号で誤検出しません
    found = text::detect_personal_data()
    assert found.length == 0,
      "personal data in the document: #{found.join("; ")}"
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

## 12.3 製薬会社：ロットコード付きの添付文書

**課題：** 添付文書には規制当局が求める文言が必要で、バーコードは正しい製品を
指していなければなりません。製品間でコードを取り違えるのは、この業界で最も
高くつくミスです。

`profiles/insert.pdfl`:

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
    assert codes::validate_barcode_checksum(1),
      "invalid check digit: #{code}"

    // 最も高くつくミスを捕まえます：ある製品のコードに別製品の文言
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

## 12.4 承認：承認版との比較

**課題：** 顧客が v1 を承認しました。v2 が「1語だけ直した」と言って届きます。
それを信じるのは高くつきます。

```bash
# 実際に何が変わったかを HTML で
pdfl compare approved/catalogue_v1.pdf received/catalogue_v2.pdf \
  --normalize --ignore-dates \
  --output html --output-file differences.html

echo "exit: $?"   # 0 同一 · 1 メタデータのみ · 2 内容が変化
```

テキストだけでなく**見た目**も確認するには：

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

## 12.5 CI/CD：一括検証

**課題：** リポジトリに入るすべてのファイルがプリフライトを通る必要があり、
誰も手作業で実行しないようにしたい。

`.github/workflows/preflight.yml`:

```yaml
name: PDF preflight

on: [push, pull_request]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install pdfl
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}   # Actions の自動トークン。設定は不要
        run: |
          gh release download --repo kotarokikuchi/pdflang \
            --pattern 'pdfl-*-linux-x64.tar.gz'
          mkdir pdfl && tar xzf pdfl-*-linux-x64.tar.gz --strip-components=1 -C pdfl
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

## 12.6 印刷所向けにファイルを整える

```pdfl
// profiles/prepare.pdfl
check "Preconditions" {
  require doc.page_count > 0
  assert !struct::check_encryption(),
    "file is encrypted — ask for the open version"
}

// 出版社が設定しなかった制作用ボックス
fix::set_trim_box(8.5, 8.5, 586.5, 833.5)
fix::set_bleed_box(0, 0, 595, 842)

// 整理
fix::remove_annotations()      // 校正コメント
fix::remove_attachments()      // ファイルを重くするだけの添付
fix::flatten_layers()          // レイヤーの再表示事故を防ぐ
fix::remove_unused_resources()
```

```bash
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf --dry-run  # 確認
pdfl fix publisher.pdf profiles/prepare.pdfl --output print.pdf            # 適用
pdfl run profiles/offset.pdfl print.pdf                                    # 検証
```

---

## 12.7 チームへのプロファイル配布

**課題：** 5台の端末でまったく同じプロファイルとデータを使い、誰も改変して
いないことを保証したい。

```bash
# プロファイルを管理している端末で
pdfl pack profiles/ --name print-profile --version 1.2.0

# 制作端末で
pdfl add print-profile.pdflpkg
# ./pdfl_profiles/print-profile@1.2.0/ に、各ハッシュを検証してインストール

pdfl run pdfl_profiles/print-profile@1.2.0/offset.pdfl file.pdf
```

途中でパッケージが改変されていれば、`add` は**インストールを拒否**します。

---

## 12.8 問題のあるファイルを調べる

原因がわからないときの実践的な手順：

```bash
# 1. 数秒で全体像を把握
pdfl inspect suspect.pdf

# 2. print() だけの調査用スクリプト
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
# print() は標準エラー出力なので、レポートは捨てて
# 調査結果だけを見られます
```

---

[← CLI コマンド](11-cli.md) · [目次](README.md)
