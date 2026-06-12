//! 一覧のインクリメンタル検索（大文字小文字無視の循環部分一致）の共通ロジック。
//! Filer の一覧と Tree パネルの両方が利用する。State ごとに巡回ロジックを
//! コピーせず、ここに集約する。

/// `start` から始め、`forward` で前方/後方へ循環しながら、クエリに最初に
/// マッチする index を返す。マッチは大文字小文字を無視した部分一致。
/// `name_at(i)` は index `i` の表示名（`None` はマッチ対象外）。
/// 空クエリ・空リストでは `None`。
pub(crate) fn find_matching_index<'a>(
    len: usize,
    start: usize,
    forward: bool,
    query: &str,
    name_at: impl Fn(usize) -> Option<&'a str>,
) -> Option<usize> {
    if query.is_empty() || len == 0 {
        return None;
    }
    let start = start % len;
    let query_lower = query.to_lowercase();
    for step in 0..len {
        let i = if forward {
            (start + step) % len
        } else {
            (start + len - step) % len
        };
        if let Some(name) = name_at(i)
            && name.to_lowercase().contains(&query_lower)
        {
            return Some(i);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<&'static str> {
        vec!["alpha", "Beta", "gamma", "beta2"]
    }

    fn find(start: usize, forward: bool, query: &str) -> Option<usize> {
        let list = names();
        find_matching_index(list.len(), start, forward, query, |i| Some(list[i]))
    }

    #[test]
    fn matches_case_insensitive_substring() {
        assert_eq!(find(0, true, "beta"), Some(1));
    }

    #[test]
    fn empty_query_or_empty_list_returns_none() {
        assert_eq!(find(0, true, ""), None);
        assert_eq!(find_matching_index(0, 0, true, "x", |_| Some("x")), None);
    }

    #[test]
    fn forward_wraps_around() {
        // index 3 から前方検索すると先頭へ折り返して alpha(0) を見つける。
        assert_eq!(find(3, true, "alpha"), Some(0));
    }

    #[test]
    fn backward_search_finds_previous_match() {
        // index 0 から後方検索（start を wrapping_sub(1) した usize::MAX 相当）。
        assert_eq!(find(usize::MAX, false, "beta"), Some(3));
    }

    #[test]
    fn skips_entries_whose_name_is_none() {
        let got = find_matching_index(3, 0, true, "x", |i| if i == 1 { None } else { Some("x") });
        assert_eq!(got, Some(0));
    }
}
