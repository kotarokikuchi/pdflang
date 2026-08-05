# 4. `struct::` 名前空間 — 構造とメタデータ

[← `text::`](03-text.md) · [目次](README.md) · [次：`visual::` →](05-visual.md)

ファイルそのものに関する23の関数：メタデータ、内部オブジェクト、セキュリティ、
追跡可能性。

> `list_objects` 以降の関数はファイルの内部構造を読みます。この解析は初回
> 使用時に**一度だけ**実行され、キャッシュされます。

---

## 4.1 メタデータ

| 関数 | 戻り値 |
|---|---|
| `struct::get_title()` | タイトル |
| `struct::get_author()` | 著者 |
| `struct::get_subject()` | 主題 |
| `struct::get_keywords()` | キーワード |
| `struct::get_creator()` | 元文書を作成したプログラム |
| `struct::get_producer()` | PDF を生成したプログラム |
| `struct::get_creation_date()` | 作成日時（`YYYY-MM-DD HH:MM:SS`） |
| `struct::get_modification_date()` | 更新日時（同形式） |
| `struct::list_metadata_entries()` | 空でない項目の一覧（`"キー: 値"`） |
| `struct::extract_xmp()` | カタログの XMP メタデータ |

値が無い項目は空文字列を返します。

```pdfl
check "Required metadata" {
  assert struct::get_title() != "", "PDF has no title"
  assert struct::get_author() != "", "PDF has no author"

  // Producer は生成元ツールを示します — 問題の追跡に有用です
  print("produced by:", struct::get_producer())

  created = struct::get_creation_date()
  assert created != "", "PDF has no creation date"
  // 形式が整列可能なので文字列比較が使えます
  assert created > "2026-01-01", "file is too old for this campaign"
}

check "XMP present" {
  xmp = struct::extract_xmp()
  assert xmp != "", "PDF has no XMP metadata"
  assert xmp.contains("pdfaid"), "no PDF/A identification in the XMP"
}
```

---

## 4.2 ファイルと追跡可能性

| 関数 | 動作 |
|---|---|
| `struct::file_size()` | サイズ（バイト） |
| `struct::calculate_sha256()` | ファイルの SHA-256 ハッシュ |
| `struct::detect_file_bloat([kb_per_page])` | 1ページあたりの上限（既定 1024 KB）超過なら真 |

```pdfl
check "File size and traceability" {
  mb = struct::file_size() / 1024 / 1024
  assert mb < 10, "file is #{round(mb)} MB (10 MB e-mail limit)"

  // ハッシュはどのファイルが承認されたかを証明します
  print("SHA-256:", struct::calculate_sha256())

  assert !struct::detect_file_bloat(1024),
    "heavy file: #{struct::file_size() / 1024} KB for #{doc.page_count} pages"
}
```

---

## 4.3 内部オブジェクト

| 関数 | 動作 |
|---|---|
| `struct::count_objects()` | ページ内のコンテンツオブジェクト数 |
| `struct::list_objects()` | 全オブジェクトの一覧（`"番号: 種類"`） |
| `struct::detect_unreferenced_objects()` | trailer から到達できないオブジェクト |
| `struct::detect_orphaned_resources()` | 到達できないリソース（フォント、画像） |
| `struct::measure_object_size(number)` | 指定オブジェクトの概算サイズ（バイト） |

> インフラ用のオブジェクト（`ObjStm`、`XRef`）は除外されます。定義上 trailer
> から参照されないため、報告すると誤検出になります。

```pdfl
check "File hygiene" {
  require struct::count_objects() > 0

  loose = struct::detect_unreferenced_objects()
  assert loose.length == 0,
    "#{loose.length} unreferenced object(s): #{loose.join(", ")}"

  orphans = struct::detect_orphaned_resources()
  assert orphans.length == 0,
    "unused embedded resources: #{orphans.join(", ")} — run 'pdfl fix' with remove_unused_resources()"

  print("size of object 5:", struct::measure_object_size(5), "bytes")
}
```

---

## 4.4 セキュリティ

| 関数 | 動作 |
|---|---|
| `struct::detect_javascript()` | JavaScript が埋め込まれていれば真 |
| `struct::detect_suspicious_actions()` | 危険なアクションの一覧 |
| `struct::check_encryption()` | 暗号化されていれば真 |
| `struct::validate_permissions()` | 権限の制限が無ければ真 |
| `struct::validate_signatures()` | 電子署名フィールドがあれば真 |

`detect_suspicious_actions` が検出するのは `JavaScript`、`Launch`（プログラム
実行）、`URI`、`SubmitForm`、`ImportData`、`GoToR` です。

> `validate_signatures` はフィールドの**存在**を確認します。証明書チェーンの
> 暗号学的検証はこのバージョンでは行いません。

```pdfl
check "Security" {
  // PDF 内の JavaScript は攻撃経路になり得ますし、
  // 印刷用のドキュメントには不要です
  assert !struct::detect_javascript(), "PDF contains embedded JavaScript"

  actions = struct::detect_suspicious_actions()
  assert actions.length == 0,
    "suspicious actions in the PDF: #{actions.join("; ")}"

  // 暗号化された PDF は印刷所の RIP で失敗することがあります
  assert !struct::check_encryption(),
    "PDF is encrypted — remove protection before sending it to print"
  assert struct::validate_permissions(),
    "PDF has permission restrictions that may block processing"
}
```

---

## 4.5 完全な例

```pdfl
// audit.pdfl — コンプライアンスとセキュリティの検証
profile "file-audit" {

  check "Identification" tags: ["metadata"] {
    assert struct::get_title() != "", "no title"
    assert struct::get_author() != "", "no author"
    assert struct::get_creation_date() != "", "no creation date"
    print("produced by:", struct::get_producer())
  }

  check "Traceability" tags: ["audit"] {
    print("SHA-256:", struct::calculate_sha256())
    print("size:", struct::file_size() / 1024, "KB")
  }

  check "Security" tags: ["security"] {
    assert !struct::detect_javascript(), "embedded JavaScript"
    assert !struct::check_encryption(), "encrypted file"
    actions = struct::detect_suspicious_actions()
    assert actions.length == 0, "suspicious actions: #{actions.join("; ")}"
  }

  check "File hygiene" tags: ["optimization"] {
    orphans = struct::detect_orphaned_resources()
    assert orphans.length == 0, "unused resources: #{orphans.join(", ")}"
    assert !struct::detect_file_bloat(1024), "bloated file"
  }
}
```

---

[← `text::`](03-text.md) · [目次](README.md) · [次：`visual::` →](05-visual.md)
