# イテレーションとアロケーションのルール

繰り返し処理での無駄なアロケーション・再計算を避ける。

## ループ不変な変換はループ外で 1 回だけ

ループ本体（`filter` / `map` / `for`）の各要素で同じ結果になる変換は、ループ外で
1 回だけ計算してから使い回す。特に `to_lowercase()` / `to_uppercase()` などは新規
`String` を確保するため、要素ごとに呼ぶと件数 N に比例した無駄な確保になる。

```rust
// NG: 要素ごとにクエリを小文字化（N 回アロケート）
files.iter().filter(|f| f.name().to_lowercase().contains(&query.to_lowercase()))

// OK: クエリ側はループ外で 1 回だけ
let lower_query = query.to_lowercase();
files.iter().filter(|f| f.name().to_lowercase().contains(&lower_query))
```

大文字小文字を無視した部分一致は `FilerState::find_matching_index` /
`FilerFilter::matches_name` が先例。新しい照合を書くときはこの形に倣う。

## 退避は clone ではなく mem::take

フィールドの中身を別フィールドへ移して直後に元を作り直す場合、`clone()` ではなく
`std::mem::take(&mut self.field)` を使う（コピーを避け、空にする処理も兼ねる）。
捨てるだけなら `Vec::clear()`（確保済み容量を再利用）を使い、`Vec::new()` 代入は避ける。
