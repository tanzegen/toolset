//! 文本 Diff：行级 LCS 比较，输出增/删/等行（自实现，无第三方依赖）。

use crate::error::AppResult;
use serde::Serialize;

#[derive(Serialize)]
pub struct DiffRow {
    pub tag: String, // "equal" | "insert" | "delete"
    pub left_no: Option<usize>,
    pub right_no: Option<usize>,
    pub text: String,
}

#[derive(Serialize)]
pub struct DiffResult {
    pub rows: Vec<DiffRow>,
    pub added: usize,
    pub removed: usize,
}

pub fn text_diff(left: String, right: String) -> AppResult<DiffResult> {
    let a: Vec<&str> = if left.is_empty() {
        Vec::new()
    } else {
        left.split('\n').collect()
    };
    let b: Vec<&str> = if right.is_empty() {
        Vec::new()
    } else {
        right.split('\n').collect()
    };
    let (n, m) = (a.len(), b.len());

    // LCS 长度表（自底向上）
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }

    let mut rows = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    let (mut la, mut lb) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            la += 1;
            lb += 1;
            rows.push(DiffRow {
                tag: "equal".into(),
                left_no: Some(la),
                right_no: Some(lb),
                text: a[i].to_string(),
            });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            la += 1;
            rows.push(DiffRow {
                tag: "delete".into(),
                left_no: Some(la),
                right_no: None,
                text: a[i].to_string(),
            });
            i += 1;
        } else {
            lb += 1;
            rows.push(DiffRow {
                tag: "insert".into(),
                left_no: None,
                right_no: Some(lb),
                text: b[j].to_string(),
            });
            j += 1;
        }
    }
    while i < n {
        la += 1;
        rows.push(DiffRow {
            tag: "delete".into(),
            left_no: Some(la),
            right_no: None,
            text: a[i].to_string(),
        });
        i += 1;
    }
    while j < m {
        lb += 1;
        rows.push(DiffRow {
            tag: "insert".into(),
            left_no: None,
            right_no: Some(lb),
            text: b[j].to_string(),
        });
        j += 1;
    }

    let added = rows.iter().filter(|r| r.tag == "insert").count();
    let removed = rows.iter().filter(|r| r.tag == "delete").count();
    Ok(DiffResult {
        rows,
        added,
        removed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_diff() {
        let r = text_diff("a\nb\nc".into(), "a\nx\nc".into()).unwrap();
        assert_eq!(r.added, 1);
        assert_eq!(r.removed, 1);
        // 应有: equal a / delete b / insert x / equal c
        assert_eq!(r.rows.len(), 4);
        assert_eq!(r.rows[0].tag, "equal");
    }

    #[test]
    fn identical() {
        let r = text_diff("x\ny".into(), "x\ny".into()).unwrap();
        assert_eq!(r.added, 0);
        assert_eq!(r.removed, 0);
        assert!(r.rows.iter().all(|x| x.tag == "equal"));
    }

    #[test]
    fn all_inserted() {
        let r = text_diff("".into(), "a\nb".into()).unwrap();
        assert_eq!(r.added, 2);
        assert_eq!(r.removed, 0);
    }
}
