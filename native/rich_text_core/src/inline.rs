//! 行内样式 run 与编辑原语（block-format-input.md §4 样式化编辑层地基）。
//!
//! 坐标统一为 **UTF-8 字节偏移**（与 Dart 的 UTF-16 code unit 在 ASCII 域等价，
//! 非 BMP 字符下有差异；跨语言边界按字节统一，见 native-rewrite-eval.md §4.2）。

use crate::ast::*;

/// 文本变更区间（旧坐标）：最长公共前/后缀夹出「删旧 + 插新」窗口。
/// 纯函数，供 [adjust_runs] 与会话层随动使用。返回 (prefix, old_end) 字节偏移。
pub fn span_change_range(old_text: &str, new_text: &str) -> (usize, usize) {
    let ob = old_text.as_bytes();
    let nb = new_text.as_bytes();
    let mut p = 0;
    while p < ob.len() && p < nb.len() && ob[p] == nb[p] {
        p += 1;
    }
    let mut s_old = ob.len();
    let mut s_new = nb.len();
    while s_old > p && s_new > p && ob[s_old - 1] == nb[s_new - 1] {
        s_old -= 1;
        s_new -= 1;
    }
    (p, s_old)
}

/// 文字变更时 run 随动（纯函数）：删除区裁剪、其后位移、run 内插入延展。
/// 重叠模型下不做合并——渲染按字符合成，语义即正确。
pub fn adjust_runs(
    runs: &[InlineRun],
    start: usize,
    old_end: usize,
    insert_len: usize,
) -> Vec<InlineRun> {
    let start = start as i64;
    let old_end = old_end as i64;
    let insert_len = insert_len as i64;
    let delta = insert_len - (old_end - start);
    let mut out = Vec::new();
    for r in runs {
        let mut s = r.start as i64;
        let mut e = r.end as i64;
        if e <= start {
            // 完全在变更区前：不动
        } else if s >= old_end {
            // 完全在变更区后：整体位移
            s += delta;
            e += delta;
        } else if start == old_end {
            // 纯插入且严格位于 run 内部：延展
            if s < start {
                e += insert_len;
            }
        } else {
            // 删除/替换：横跨（或触及）替换窗的 run 涵盖插入段，尾部在窗内的 run 裁掉窗内部分。
            let ns = if s < start { s } else { start };
            let ne = if e >= old_end {
                start + insert_len + (e - old_end)
            } else {
                ns + dart_clamp(e - start, 0, start - s)
            };
            s = ns;
            e = ne;
            if e <= s {
                continue; // run 被删空
            }
        }
        out.push(InlineRun {
            start: s as usize,
            end: e as usize,
            mark: r.mark,
            url: r.url.clone(),
        });
    }
    out
}

/// 镜像 Dart `num.clamp(lower, upper)` = `min(max(v, lower), upper)`（lower>upper 时不 panic）。
fn dart_clamp(v: i64, lo: i64, hi: i64) -> i64 {
    v.max(lo).min(hi)
}

/// 选区套格式（编辑态样式化层）：在 [start, end) 上切换 [mark]。
/// 重叠模型：先按边界切分跨 start/end 的 mark run，整段已覆盖则摘掉、否则清掉
/// 重叠部分后整段盖上——纯函数。
pub fn toggle_inline_mark_runs(
    runs: &[InlineRun],
    mark: InlineMark,
    start: usize,
    end: usize,
) -> Vec<InlineRun> {
    if end <= start {
        return runs.to_vec();
    }
    let start = start as i64;
    let end = end as i64;
    // 1. 边界切分：把跨 start/end 的 mark run 断开，避免局部覆盖残留
    let mut split: Vec<InlineRun> = Vec::new();
    for r in runs {
        let rs = r.start as i64;
        let re = r.end as i64;
        if r.mark != mark {
            split.push(r.clone());
            continue;
        }
        if rs < start && start < re {
            split.push(InlineRun {
                start: r.start,
                end: start as usize,
                mark,
                url: r.url.clone(),
            });
            split.push(InlineRun {
                start: start as usize,
                end: r.end,
                mark,
                url: r.url.clone(),
            });
            continue;
        }
        if rs < end && end < re {
            split.push(InlineRun {
                start: r.start,
                end: end as usize,
                mark,
                url: r.url.clone(),
            });
            split.push(InlineRun {
                start: end as usize,
                end: r.end,
                mark,
                url: r.url.clone(),
            });
            continue;
        }
        split.push(r.clone());
    }
    // 2. 区间内 mark 覆盖度判定
    let mut intervals: Vec<(usize, usize)> = Vec::new();
    for r in &split {
        if r.mark != mark {
            continue;
        }
        let s = r.start.clamp(start as usize, end as usize);
        let e = r.end.clamp(start as usize, end as usize);
        if e > s {
            intervals.push((s, e));
        }
    }
    intervals.sort_by_key(|a| a.0);
    let mut covered = start as usize;
    for (s, e) in &intervals {
        if *s > covered {
            break;
        }
        if *e > covered {
            covered = *e;
        }
    }
    let fully_covered = covered >= end as usize;
    // 3. 清掉区间内 mark run（无论开/关都先清，避免重叠残留）
    let cleared: Vec<InlineRun> = split
        .into_iter()
        .filter(|r| !(r.mark == mark && r.end > start as usize && r.start < end as usize))
        .collect();
    if fully_covered {
        return cleared;
    }
    let mut result = cleared;
    result.push(InlineRun {
        start: start as usize,
        end: end as usize,
        mark,
        url: None,
    });
    result
}

/// runs + plain → 行内节点树（序列化回写与块重建的唯一出口）。
/// 重叠 run 按字符合成标记集合，相邻同集合（含同 link url）段合并；
/// 嵌套优先级固定为 [InlineMark::all] 顺序（bold > italic > code > underline > link）。
/// link run 的 url 从载荷还原——编辑链接文字不丢 url（数据无损防线）。
pub fn inline_nodes_of(plain: &str, runs: &[InlineRun]) -> Vec<InlineNode> {
    let n = plain.len();
    let mut sets: Vec<Vec<InlineMark>> = vec![Vec::new(); n];
    let mut urls: Vec<Option<String>> = vec![None; n];
    for r in runs {
        let rs = r.start.min(n);
        let re = r.end.min(n);
        for i in rs..re {
            sets[i].push(r.mark);
            if r.mark == InlineMark::Link {
                urls[i] = r.url.clone();
            }
        }
    }

    let styled = |text: &str, marks: &[InlineMark], url: &Option<String>| -> InlineNode {
        let mut node = InlineNode::Text(text.to_string());
        for m in InlineMark::all() {
            if !marks.contains(m) {
                continue;
            }
            node = match m {
                InlineMark::Bold => InlineNode::Strong(vec![node]),
                InlineMark::Italic => InlineNode::Em(vec![node]),
                InlineMark::Code => InlineNode::Code(text.to_string()),
                InlineMark::Underline => InlineNode::Underline(vec![node]),
                InlineMark::Link => InlineNode::Link {
                    label: text.to_string(),
                    url: url.clone().unwrap_or_default(),
                    autolink: false,
                },
                InlineMark::Strikethrough => InlineNode::Strikethrough(vec![node]),
                InlineMark::Highlight => InlineNode::Highlight(vec![node]),
                InlineMark::Math => InlineNode::Math(text.to_string()),
            };
        }
        node
    };

    let mut out: Vec<InlineNode> = Vec::new();
    let mut i = 0;
    while i < n {
        let mut j = i + 1;
        while j < n
            && sets[j].len() == sets[i].len()
            && set_contains_all(&sets[j], &sets[i])
            && urls[j] == urls[i]
        {
            j += 1;
        }
        let seg = &plain[i..j];
        out.push(styled(seg, &sets[i], &urls[i]));
        i = j;
    }
    out
}

fn set_contains_all(a: &[InlineMark], b: &[InlineMark]) -> bool {
    b.iter().all(|x| a.contains(x))
}
