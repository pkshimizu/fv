# 一覧内インクリメンタル検索のルール

ファイル一覧やツリーなど「名前付きの並び」に対する `f` 系のインクリメンタル検索
（大文字小文字無視の循環部分一致）は、State ごとに巡回ロジックをコピーせず、共通の
`state::list_search::find_matching_index` を使う。

```rust
// 各 State は「件数・開始位置・方向・クエリ・index→名前の取り出し」を渡すだけ。
fn find_matching_index(&self, query: &str, start: usize, forward: bool) -> Option<usize> {
    super::list_search::find_matching_index(
        self.items.len(),
        start,
        forward,
        query,
        |i| self.items[i].name(), // Option<&str>
    )
}
```

- 巡回アルゴリズム（空クエリ/空リストの early-return、`start % len` 正規化、
  前方/後方の剰余巡回、`wrapping_add(1)`/`wrapping_sub(1)` での起点ずらし）は
  ヘルパに集約済み。新しいパネルで検索を足すときも同じヘルパに乗せる。
- カーソル移動の `select_matching` / `select_next_matching` / `select_prev_matching` も
  Filer / Tree で同型。新規パネルではこの 3 メソッド名・引数に合わせると、App 側の
  Search 振り分け（`SearchTarget`）にそのまま組み込める。
- App は Search 系アクション（`SearchUpdate`/`SearchNext`/`SearchPrev` と Esc 復元）を
  `SearchTarget`（ツリーパネルが開いていればツリー、なければ Filer）で 1 箇所に振り分ける。
  対象を増やすときは `SearchTarget` に追加し、各アームを分岐させない。
