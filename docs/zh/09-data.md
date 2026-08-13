# 9. `data::` 命名空间 — 外部数据

[← `fix::`](08-fix.md) · [目录](README.md) · [下一章：标准库 →](10-stdlib.md)

把 PDF 内容与你自己的清单和表格进行核对的 8 个函数。全部在本地处理，
数据不会外传。

---

## 9.1 文件的存放位置

术语表和数据集接受**相对于执行目录**的路径：

```pdfl
data::load_glossary("terms/legal.txt")
data::load_dataset("data/batches.csv")
```

查询表（`query_gtin`、`query_medicamento`、`query_postal_code`）使用固定
文件名，按以下顺序查找：

1. `$PDFL_DATA_DIR`（环境变量）
2. `./dados/`
3. `./`
4. 由 `pdfl add` 安装的配置（`pdfl_profiles/*/dados/`）
5. 被分析 PDF 所在目录

```bash
PDFL_DATA_DIR=/opt/databases pdfl run profile.pdfl document.pdf
```

找不到时，错误消息会说明应把文件放在何处。要随配置一起分发，请使用
`pdfl pack`（[第 11 章](11-cli.md)）。

---

## 9.2 术语表与数据集

| 函数 | 功能 |
|---|---|
| `data::load_glossary(file)` | 术语列表（每行一个，`#` 为注释） |
| `data::validate_against_reference(file)` | 文档中**未出现**的术语列表 |
| `data::load_dataset(file)` | 把 CSV 读为行的列表 |
| `data::lookup_value(file, key)` | 首列为该键的行的第二列（找不到为 `null`） |

比较时忽略大小写和空白。

`terms/required.txt`：

```
# 每份保单都必须包含的术语
waiting period
covered benefits
general conditions
```

```pdfl
check "Glossary and dataset" {
  terms = data::load_glossary("terms/required.txt")
  print("terms in the glossary:", terms.length)

  // 最直接的用法
  missing = data::validate_against_reference("terms/required.txt")
  assert missing.length == 0,
    "clauses missing from the policy: #{missing.join("; ")}"

  rows = data::load_dataset("data/batches.csv")
  print("columns:", rows.first().join(" | "))   // 第一行是表头
  print("records:", rows.length - 1)

  // null 为假，因此可以直接校验
  batch = text::extract_from_region(1, region(400, 50, 150, 20)).trim()
  description = data::lookup_value("data/batches.csv", batch)
  assert description, "batch #{batch} is not in the approved list"
}
```

### JSON 数据集

以 `.json` 结尾的文件按 JSON 读取——`load_dataset` 与 `lookup_value` 都是如此。
接受两种形态，因为数据集实际上就是这两种写法。

数组的数组，就是行本身：

```json
[["batch", "description"],
 ["L2026-08", "Approved batch August/2026"]]
```

对象的数组会变成一行表头，加上每个对象一行。列的顺序取自**第一个**对象的书写
顺序，而不是字母序，因此第一个键仍然是 `lookup_value` 查找的那个键：

```json
[{"batch": "L2026-08", "description": "Approved batch August/2026"},
 {"batch": "L2026-09", "description": "Approved batch September/2026"}]
```

后面的对象少了某个键，留下的是**空单元格**，而不是错位的一行：空洞在报告里看得
见，错位看不见。数字保留写入时的数位，`null` 是空单元格——与 CSV 的空字段含义
相同。

在同一个文件里混用两种形态会报错，并指出是第几行。

---

## 9.3 查询表

按 9.1 的顺序查找固定名称的文件，返回**整行**列表（找不到则为 `null`）。

| 函数 | 参照文件 | 功能 |
|---|---|---|
| `data::query_gtin(code)` | `gtin.csv` | 按 GTIN 查询（忽略标点） |
| `data::query_medicamento(reg_or_name)` | `medicamentos.csv` | 按注册号或名称片段查询 |
| `data::query_postal_code(code)` | `ceps.csv` | 按邮编查询（8 位数字） |
| `data::validate_address(code, "fragment")` | `ceps.csv` | 该邮编的地址是否包含该片段 |

`dados/gtin.csv`：

```csv
gtin,description,manufacturer
7891234567895,Dipyrone 500mg 20 tablets,Example Labs
```

```pdfl
check "Lookup tables" {
  // 与包装上读取到的条码核对
  code = codes::decode_barcode(1)
  product = data::query_gtin(code)
  assert product, "GTIN #{code} is not in the product database"
  print("product:", product.get(2), "| manufacturer:", product.get(3))

  // 按注册号查询药品信息
  registration = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
  medicine = data::query_medicamento(registration)
  assert medicine, "registration #{registration} not found"

  // 处方药需要法定提示语
  band = medicine.get(4)
  assert band != "prescription" || text::require_text("PRESCRIPTION ONLY"),
    "prescription medicine without the mandatory text"

  // 印刷的地址与邮编是否一致
  assert data::validate_address("01310100", "Avenida Paulista"),
    "printed address does not match the given postal code"
}
```

---

## 9.4 完整示例

```pdfl
// insert_with_databases.pdfl — 与本地数据核对
// 用法: PDFL_DATA_DIR=./databases pdfl run insert_with_databases.pdfl insert.pdf
profile "insert-with-references" {

  check "Mandatory regulatory terms" tags: ["glossary"] {
    missing = data::validate_against_reference("databases/regulatory_terms.txt")
    assert missing.length == 0, "mandatory texts missing: #{missing.join("; ")}"
  }

  check "Product in the database" tags: ["data", "critical"] {
    code = codes::decode_barcode(1)
    product = data::query_gtin(code)
    assert product, "GTIN #{code} not approved"

    // 注册名称必须出现在印刷内容中
    name = product.get(2)
    assert text::require_text(name),
      "the name '#{name}' from the database does not appear on the insert"
    print("product verified:", name)
  }

  check "Registration and band" tags: ["regulatory"] {
    registration = text::extract_from_region(1, region(50, 780, 200, 15)).trim()
    med = data::query_medicamento(registration)
    assert med, "registration #{registration} not found"
    assert med.get(4) != "prescription" || text::require_text("PRESCRIPTION ONLY"),
      "prescription band requires the prescription notice"
  }

  check "Manufacturer address" tags: ["data"] {
    assert data::validate_address("01310100", "Avenida Paulista"),
      "manufacturer address does not match the postal code"
  }
}
```

---

[← `fix::`](08-fix.md) · [目录](README.md) · [下一章：标准库 →](10-stdlib.md)
