use similar::TextDiff;

/// OP_TYPE equivalent from WinMerge
#[derive(Debug, Clone, PartialEq)]
pub enum ThreeWayStatus {
    Equal,        // OP_NONE:     All three are the same
    LeftChanged,  // OP_1STONLY:  Only left changed (Base=Right)
    RightChanged, // OP_3RDONLY:  Only right changed (Base=Left)
    BothChanged,  // OP_2NDONLY:  Left=Right but differ from Base
    Conflict,     // OP_DIFF:     Left!=Right!=Base
}

impl ThreeWayStatus {
    pub fn as_i32(&self) -> i32 {
        match self {
            Self::Equal => 0,
            Self::LeftChanged => 1,
            Self::RightChanged => 2,
            Self::BothChanged => 3,
            Self::Conflict => 4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ThreeWayLine {
    pub base_text: String,
    pub left_text: String,
    pub right_text: String,
    pub status: ThreeWayStatus,
    pub base_line_no: Option<u32>,
    pub left_line_no: Option<u32>,
    pub right_line_no: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ThreeWayResult {
    pub lines: Vec<ThreeWayLine>,
    #[allow(dead_code)]
    pub conflict_count: u32,
    /// Index into `lines` where each diff block starts.
    pub conflict_positions: Vec<usize>,
}

/// A hunk from a 2-way diff: a range of "old" lines replaced by "new" lines.
#[derive(Debug, Clone)]
struct Hunk {
    old_start: usize, // inclusive (base side)
    old_end: usize,   // exclusive
    new_start: usize, // inclusive (file side)
    new_end: usize,   // exclusive
}

/// A merged 3-way diff block, analogous to WinMerge's DIFFRANGE.
#[derive(Debug)]
struct DiffBlock {
    left_start: usize,
    left_end: usize, // exclusive
    base_start: usize,
    base_end: usize, // exclusive
    right_start: usize,
    right_end: usize, // exclusive
    status: ThreeWayStatus,
}

/// Extract hunks from a TextDiff (old=base, new=file).
fn extract_hunks(diff: &TextDiff<'_, '_, '_, str>) -> Vec<Hunk> {
    let mut hunks = Vec::new();
    for op in diff.ops() {
        let old = op.old_range();
        let new = op.new_range();
        match op.tag() {
            similar::DiffTag::Equal => {}
            _ => {
                hunks.push(Hunk {
                    old_start: old.start,
                    old_end: old.end,
                    new_start: new.start,
                    new_end: new.end,
                });
            }
        }
    }
    hunks
}

/// Core 3-way diff: implements Make3wayDiff algorithm from WinMerge.
///
/// Runs base↔left and base↔right 2-way diffs, then merges the hunk lists
/// by walking them in base line-number space, grouping overlapping hunks.
pub fn compute_three_way_diff(
    base_text: &str,
    left_text: &str,
    right_text: &str,
) -> ThreeWayResult {
    let base_lines: Vec<&str> = base_text.lines().collect();
    let left_lines: Vec<&str> = left_text.lines().collect();
    let right_lines: Vec<&str> = right_text.lines().collect();

    // Step 1: Two pairwise 2-way diffs (base is "old" in both)
    let left_diff = TextDiff::from_lines(base_text, left_text);
    let right_diff = TextDiff::from_lines(base_text, right_text);

    let left_hunks = extract_hunks(&left_diff);
    let right_hunks = extract_hunks(&right_diff);

    // Step 2: Merge hunks using Make3wayDiff-style overlap detection
    let blocks = merge_hunks(
        &left_hunks,
        &right_hunks,
        &base_lines,
        &left_lines,
        &right_lines,
    );

    // Step 3: Build output lines with ghost-line alignment
    build_result_lines(&blocks, &base_lines, &left_lines, &right_lines)
}

/// Merge two hunk lists (base↔left, base↔right) into 3-way diff blocks.
/// Implements WinMerge's Make3wayDiff overlap-grouping algorithm.
fn merge_hunks(
    left_hunks: &[Hunk],
    right_hunks: &[Hunk],
    _base_lines: &[&str],
    left_lines: &[&str],
    right_lines: &[&str],
) -> Vec<DiffBlock> {
    let mut blocks = Vec::new();
    let mut li = 0usize; // cursor into left_hunks
    let mut ri = 0usize; // cursor into right_hunks

    // Track cumulative positions for computing file-side ranges
    // when one side has no hunk in a group
    let mut base_pos = 0usize;
    let mut left_pos = 0usize;
    let mut right_pos = 0usize;

    loop {
        let lh = left_hunks.get(li);
        let rh = right_hunks.get(ri);

        if lh.is_none() && rh.is_none() {
            break;
        }

        let l_base_start = lh.map(|h| h.old_start).unwrap_or(usize::MAX);
        let r_base_start = rh.map(|h| h.old_start).unwrap_or(usize::MAX);

        if l_base_start == usize::MAX && r_base_start == usize::MAX {
            break;
        }

        // Determine which hunk(s) start first in base space
        let first_base_start = l_base_start.min(r_base_start);

        // Advance positions up to this block's base start
        let skip = first_base_start - base_pos;
        base_pos = first_base_start;
        left_pos += skip;
        right_pos += skip;

        // Collect overlapping hunks into one group
        let mut group_base_end = base_pos;
        let mut group_has_left = false;
        let mut group_has_right = false;
        #[allow(unused_mut)]
        let mut group_left_start;
        #[allow(unused_mut)]
        let mut group_left_end;
        #[allow(unused_mut)]
        let mut group_right_start;
        #[allow(unused_mut)]
        let mut group_right_end;
        let group_li_start = li;
        let group_ri_start = ri;

        // Seed the group with the first hunk
        if l_base_start <= r_base_start {
            if let Some(h) = lh {
                group_base_end = h.old_end;
                group_has_left = true;
                li += 1;
            }
        } else {
            if let Some(h) = rh {
                group_base_end = h.old_end;
                group_has_right = true;
                ri += 1;
            }
        }

        // Expand group with overlapping hunks from both sides
        loop {
            let mut expanded = false;

            if let Some(h) = left_hunks.get(li) {
                if h.old_start <= group_base_end {
                    group_has_left = true;
                    group_base_end = group_base_end.max(h.old_end);
                    li += 1;
                    expanded = true;
                }
            }

            if let Some(h) = right_hunks.get(ri) {
                if h.old_start <= group_base_end {
                    group_has_right = true;
                    group_base_end = group_base_end.max(h.old_end);
                    ri += 1;
                    expanded = true;
                }
            }

            if !expanded {
                break;
            }
        }

        // Compute file-side ranges properly.
        // For a side with no hunk, it mirrors base exactly.
        // For a side with hunks, compute the total file-side span:
        //   start = file position at block's base_start
        //   end   = start + (equal lines before first hunk) + (hunk new lines) + (equal between hunks) + (equal after last hunk)
        // Simplified: file_end = file_start + base_count + net_insertions
        //   where net_insertions = sum(new_count - old_count) for each hunk in this group on this side.
        let base_count = group_base_end - base_pos;
        if !group_has_left {
            group_left_start = left_pos;
            group_left_end = left_pos + base_count;
        } else {
            // Recompute from left_pos: total file lines = base_count + net change from hunks
            group_left_start = left_pos;
            let mut net: isize = 0;
            // Walk left hunks that were consumed in this group (from the saved start to current li)
            for h in &left_hunks[group_li_start..li] {
                let old_count = (h.old_end - h.old_start) as isize;
                let new_count = (h.new_end - h.new_start) as isize;
                net += new_count - old_count;
            }
            group_left_end = ((left_pos + base_count) as isize + net).max(0) as usize;
            group_left_end = group_left_end.min(left_lines.len());
        }
        if !group_has_right {
            group_right_start = right_pos;
            group_right_end = right_pos + base_count;
        } else {
            group_right_start = right_pos;
            let mut net: isize = 0;
            for h in &right_hunks[group_ri_start..ri] {
                let old_count = (h.old_end - h.old_start) as isize;
                let new_count = (h.new_end - h.new_start) as isize;
                net += new_count - old_count;
            }
            group_right_end = ((right_pos + base_count) as isize + net).max(0) as usize;
            group_right_end = group_right_end.min(right_lines.len());
        }

        // Determine status (OP_TYPE)
        let status = if group_has_left && group_has_right {
            // Both sides changed — check if Left == Right (Comp02Functor)
            let left_new: Vec<&str> = left_lines[group_left_start..group_left_end].to_vec();
            let right_new: Vec<&str> = right_lines[group_right_start..group_right_end].to_vec();
            if left_new == right_new {
                ThreeWayStatus::BothChanged // OP_2NDONLY
            } else {
                ThreeWayStatus::Conflict // OP_DIFF
            }
        } else if group_has_left {
            ThreeWayStatus::LeftChanged // OP_1STONLY
        } else {
            ThreeWayStatus::RightChanged // OP_3RDONLY
        };

        blocks.push(DiffBlock {
            left_start: group_left_start,
            left_end: group_left_end,
            base_start: base_pos,
            base_end: group_base_end,
            right_start: group_right_start,
            right_end: group_right_end,
            status,
        });

        // Advance positions past this block
        base_pos = group_base_end;
        left_pos = group_left_end;
        right_pos = group_right_end;
    }

    blocks
}

/// Build the output ThreeWayResult from diff blocks, inserting ghost lines
/// so all 3 panes have equal line counts within each block.
fn build_result_lines(
    blocks: &[DiffBlock],
    base_lines: &[&str],
    left_lines: &[&str],
    right_lines: &[&str],
) -> ThreeWayResult {
    let mut result_lines = Vec::new();
    let mut conflict_positions = Vec::new();
    let mut conflict_count = 0u32;

    let mut base_pos = 0usize;
    let mut left_pos = 0usize;
    let mut right_pos = 0usize;

    for block in blocks {
        // Equal lines before this block
        while base_pos < block.base_start {
            result_lines.push(ThreeWayLine {
                left_text: left_lines.get(left_pos).unwrap_or(&"").to_string(),
                base_text: base_lines.get(base_pos).unwrap_or(&"").to_string(),
                right_text: right_lines.get(right_pos).unwrap_or(&"").to_string(),
                status: ThreeWayStatus::Equal,
                left_line_no: Some(left_pos as u32 + 1),
                base_line_no: Some(base_pos as u32 + 1),
                right_line_no: Some(right_pos as u32 + 1),
            });
            base_pos += 1;
            left_pos += 1;
            right_pos += 1;
        }

        // Diff block: insert lines with ghost-line alignment
        let left_count = block.left_end - block.left_start;
        let base_count = block.base_end - block.base_start;
        let right_count = block.right_end - block.right_start;
        let max_lines = left_count.max(base_count).max(right_count);

        conflict_positions.push(result_lines.len());
        if block.status == ThreeWayStatus::Conflict {
            conflict_count += 1;
        }

        for j in 0..max_lines {
            let (lt, l_no) = if j < left_count {
                let idx = block.left_start + j;
                (
                    left_lines.get(idx).unwrap_or(&"").to_string(),
                    Some(idx as u32 + 1),
                )
            } else {
                (String::new(), None) // Ghost line
            };
            let (bt, b_no) = if j < base_count {
                let idx = block.base_start + j;
                (
                    base_lines.get(idx).unwrap_or(&"").to_string(),
                    Some(idx as u32 + 1),
                )
            } else {
                (String::new(), None)
            };
            let (rt, r_no) = if j < right_count {
                let idx = block.right_start + j;
                (
                    right_lines.get(idx).unwrap_or(&"").to_string(),
                    Some(idx as u32 + 1),
                )
            } else {
                (String::new(), None)
            };

            result_lines.push(ThreeWayLine {
                left_text: lt,
                base_text: bt,
                right_text: rt,
                status: block.status.clone(),
                left_line_no: l_no,
                base_line_no: b_no,
                right_line_no: r_no,
            });
        }

        base_pos = block.base_end;
        left_pos = block.left_end;
        right_pos = block.right_end;
    }

    // Trailing equal lines
    while base_pos < base_lines.len() {
        result_lines.push(ThreeWayLine {
            left_text: left_lines.get(left_pos).unwrap_or(&"").to_string(),
            base_text: base_lines.get(base_pos).unwrap_or(&"").to_string(),
            right_text: right_lines.get(right_pos).unwrap_or(&"").to_string(),
            status: ThreeWayStatus::Equal,
            left_line_no: Some(left_pos as u32 + 1),
            base_line_no: Some(base_pos as u32 + 1),
            right_line_no: Some(right_pos as u32 + 1),
        });
        base_pos += 1;
        left_pos += 1;
        right_pos += 1;
    }

    ThreeWayResult {
        lines: result_lines,
        conflict_count,
        conflict_positions,
    }
}

/// Rebuild text for one pane from ThreeWayResult lines, skipping ghost lines.
/// pane: 0=left, 1=base, 2=right.
#[cfg(test)]
fn rebuild_pane_text(lines: &[ThreeWayLine], pane: i32) -> String {
    let mut out = Vec::new();
    for line in lines {
        let (line_no, text) = match pane {
            0 => (&line.left_line_no, &line.left_text),
            1 => (&line.base_line_no, &line.base_text),
            _ => (&line.right_line_no, &line.right_text),
        };
        if line_no.is_some() {
            out.push(text.as_str());
        }
    }
    if out.is_empty() {
        String::new()
    } else {
        out.join("\n") + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_conflicts() {
        let base = "aaa\nbbb\nccc\n";
        let left = "AAA\nbbb\nccc\n";
        let right = "aaa\nbbb\nCCC\n";
        let result = compute_three_way_diff(base, left, right);
        assert_eq!(result.conflict_count, 0);
        assert_eq!(result.lines[0].status, ThreeWayStatus::LeftChanged);
        assert_eq!(result.lines[1].status, ThreeWayStatus::Equal);
        assert_eq!(result.lines[2].status, ThreeWayStatus::RightChanged);
    }

    #[test]
    fn test_conflict() {
        let base = "aaa\nbbb\nccc\n";
        let left = "XXX\nbbb\nccc\n";
        let right = "YYY\nbbb\nccc\n";
        let result = compute_three_way_diff(base, left, right);
        assert_eq!(result.conflict_count, 1);
        assert_eq!(result.lines[0].status, ThreeWayStatus::Conflict);
    }

    #[test]
    fn test_both_same_change() {
        let base = "aaa\nbbb\nccc\n";
        let left = "XXX\nbbb\nccc\n";
        let right = "XXX\nbbb\nccc\n";
        let result = compute_three_way_diff(base, left, right);
        assert_eq!(result.conflict_count, 0);
        assert_eq!(result.lines[0].status, ThreeWayStatus::BothChanged);
    }

    #[test]
    fn test_left_insertion() {
        let base = "aaa\n";
        let left = "aaa\nbbb\n";
        let right = "aaa\n";
        let result = compute_three_way_diff(base, left, right);
        assert_eq!(result.lines[0].status, ThreeWayStatus::Equal);
        assert_eq!(result.lines[1].status, ThreeWayStatus::LeftChanged);
        assert_eq!(result.lines[1].left_text, "bbb");
        // Ghost lines: base and right should have no line number
        assert!(result.lines[1].base_line_no.is_none());
        assert!(result.lines[1].right_line_no.is_none());
    }

    #[test]
    fn test_overlapping_hunks() {
        // Left changes lines 1-2, Right changes lines 2-3
        // These overlap at line 2, so should merge into one conflict block
        let base = "aaa\nbbb\nccc\nddd\n";
        let left = "AAA\nBBB\nccc\nddd\n";
        let right = "aaa\nXXX\nYYY\nddd\n";
        let result = compute_three_way_diff(base, left, right);
        // Lines 0-2 should be one conflict block (overlapping at base line 1)
        let non_equal: Vec<_> = result
            .lines
            .iter()
            .filter(|l| l.status != ThreeWayStatus::Equal)
            .collect();
        assert!(!non_equal.is_empty());
        // Should be conflict since both sides changed overlapping region differently
        assert!(
            non_equal
                .iter()
                .any(|l| l.status == ThreeWayStatus::Conflict)
        );
    }

    #[test]
    fn test_different_line_counts() {
        // Left adds 2 lines, Right adds 1 line at same position
        let base = "aaa\nccc\n";
        let left = "aaa\nbbb1\nbbb2\nccc\n";
        let right = "aaa\nxxx\nccc\n";
        let result = compute_three_way_diff(base, left, right);
        // Both inserted after "aaa" — should be conflict
        // The diff block should have ghost lines to align
        let block_lines: Vec<_> = result
            .lines
            .iter()
            .filter(|l| l.status != ThreeWayStatus::Equal)
            .collect();
        assert!(!block_lines.is_empty());
        // Left has 2 lines, right has 1 → max is 2 → right gets 1 ghost line
        let max_block = block_lines.len();
        assert!(max_block >= 2);
    }

    #[test]
    fn test_empty_base() {
        let base = "\n";
        let left = "aaaa\naa\n";
        let right = "bbb\n";
        let result = compute_three_way_diff(base, left, right);
        // Both sides changed the single base line differently → conflict
        assert!(result.conflict_count > 0);
    }

    #[test]
    fn test_repro_seed0_step13() {
        let base = "\nbbb\nLLL\n\n\n";
        let left = "\n\n\n";
        let right = "aaa\nworld\nfoo\nRRR\ntest\n";
        let result = compute_three_way_diff(base, left, right);
        let rebuilt_left = rebuild_pane_text(&result.lines, 0);
        assert_eq!(rebuilt_left, left);
    }

    #[test]
    fn test_repro_seed0_step7() {
        // Reproduced from stress test: seed=0 step=7
        // All 3 sides completely different → one big conflict
        let base = "aaa\nbbb\n\nworld\nddd\n";
        let left = "XXX\nbbb\nLLL\n\n";
        let right = "aaa\nworld\nfoo\nRRR\ntest\n";
        let result = compute_three_way_diff(base, left, right);
        let rebuilt_right = rebuild_pane_text(&result.lines, 2);
        let expected_right = "aaa\nworld\nfoo\nRRR\ntest\n";
        assert_eq!(
            rebuilt_right, expected_right,
            "right text mismatch:\nlines: {:#?}",
            result.lines
        );
    }

    /// Stress test: simulate random user interactions (edit, backspace, F5, copy)
    /// on 3-way diff data and verify no panics or data corruption occur.
    #[test]
    fn test_random_operations_stress() {
        // Simple deterministic PRNG (no rand crate needed)
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                self.0 >> 33
            }
            fn range(&mut self, max: usize) -> usize {
                if max == 0 {
                    0
                } else {
                    self.next() as usize % max
                }
            }
        }

        let seeds: Vec<u64> = (0..100u64).collect();

        for seed in seeds {
            let mut rng = Rng(seed * 12345 + 67890);

            // Initial texts
            let mut left_lines: Vec<String> =
                vec!["aaa".into(), "bbb".into(), "ccc".into(), "ddd".into()];
            let mut base_lines: Vec<String> =
                vec!["aaa".into(), "bbb".into(), "ccc".into(), "ddd".into()];
            let mut right_lines: Vec<String> =
                vec!["aaa".into(), "bbb".into(), "ccc".into(), "ddd".into()];

            // Make some initial changes so there are diffs
            if seed % 3 == 0 {
                left_lines[0] = "XXX".into();
            }
            if seed % 3 == 1 {
                right_lines[1] = "YYY".into();
            }
            if seed % 2 == 0 {
                left_lines[2] = "LLL".into();
                right_lines[2] = "RRR".into();
            }

            let words = ["foo", "bar", "baz", "qux", "hello", "world", "test", ""];

            for step in 0..100 {
                let op = rng.range(5);
                match op {
                    // 0: Edit a random line on a random pane
                    0 => {
                        let pane = rng.range(3);
                        let lines = match pane {
                            0 => &mut left_lines,
                            1 => &mut base_lines,
                            _ => &mut right_lines,
                        };
                        if !lines.is_empty() {
                            let idx = rng.range(lines.len());
                            let word = words[rng.range(words.len())];
                            lines[idx] = word.to_string();
                        }
                    }
                    // 1: Backspace (delete a line if empty, or clear it)
                    1 => {
                        let pane = rng.range(3);
                        let lines = match pane {
                            0 => &mut left_lines,
                            1 => &mut base_lines,
                            _ => &mut right_lines,
                        };
                        if !lines.is_empty() {
                            let idx = rng.range(lines.len());
                            if lines[idx].is_empty() && lines.len() > 1 {
                                lines.remove(idx);
                            } else {
                                lines[idx] = String::new();
                            }
                        }
                    }
                    // 2: Insert a line
                    2 => {
                        let pane = rng.range(3);
                        let lines = match pane {
                            0 => &mut left_lines,
                            1 => &mut base_lines,
                            _ => &mut right_lines,
                        };
                        if lines.len() < 20 {
                            // cap to avoid blowup
                            let idx = rng.range(lines.len() + 1);
                            let word = words[rng.range(words.len())];
                            lines.insert(idx, word.to_string());
                        }
                    }
                    // 3: F5 (recompute diff from current texts)
                    3 => {
                        let left_text = if left_lines.is_empty() {
                            String::new()
                        } else {
                            left_lines.join("\n") + "\n"
                        };
                        let base_text = if base_lines.is_empty() {
                            String::new()
                        } else {
                            base_lines.join("\n") + "\n"
                        };
                        let right_text = if right_lines.is_empty() {
                            String::new()
                        } else {
                            right_lines.join("\n") + "\n"
                        };

                        let result = compute_three_way_diff(&base_text, &left_text, &right_text);

                        // Invariant checks
                        assert!(
                            !result.lines.is_empty()
                                || (left_lines.is_empty()
                                    && base_lines.is_empty()
                                    && right_lines.is_empty()),
                            "seed={} step={}: empty result with non-empty input",
                            seed,
                            step
                        );

                        // Rebuild text from result and verify it matches input
                        let rebuilt_left = rebuild_pane_text(&result.lines, 0);
                        let rebuilt_base = rebuild_pane_text(&result.lines, 1);
                        let rebuilt_right = rebuild_pane_text(&result.lines, 2);

                        assert_eq!(
                            rebuilt_left,
                            left_text,
                            "seed={} step={}: left text mismatch after F5\nbase={:?}\nleft={:?}\nright={:?}\nrebuilt_left={:?}\nlines={:#?}",
                            seed,
                            step,
                            base_text,
                            left_text,
                            right_text,
                            rebuilt_left,
                            result.lines
                        );
                        assert_eq!(
                            rebuilt_base,
                            base_text,
                            "seed={} step={}: base text mismatch after F5\nbase={:?}\nleft={:?}\nright={:?}\nrebuilt_base={:?}\nlines={:#?}",
                            seed,
                            step,
                            base_text,
                            left_text,
                            right_text,
                            rebuilt_base,
                            result.lines
                        );
                        assert_eq!(
                            rebuilt_right,
                            right_text,
                            "seed={} step={}: right text mismatch after F5\nbase={:?}\nleft={:?}\nright={:?}\nrebuilt_right={:?}\nlines={:#?}",
                            seed,
                            step,
                            base_text,
                            left_text,
                            right_text,
                            rebuilt_right,
                            result.lines
                        );

                        // All conflict positions must be valid indices
                        for &pos in &result.conflict_positions {
                            assert!(
                                pos < result.lines.len(),
                                "seed={} step={}: conflict_position {} out of bounds (len={})",
                                seed,
                                step,
                                pos,
                                result.lines.len()
                            );
                        }

                        // Every line in result must have at least one real line number
                        // (pure ghost rows shouldn't exist — at least one pane has content)
                        for (i, line) in result.lines.iter().enumerate() {
                            let has_any = line.left_line_no.is_some()
                                || line.base_line_no.is_some()
                                || line.right_line_no.is_some();
                            assert!(
                                has_any,
                                "seed={} step={}: line {} has no line numbers at all",
                                seed, step, i
                            );
                        }
                    }
                    // 4: Copy (simulate resolve: copy left→base or right→base for a diff block)
                    4 => {
                        let left_text = if left_lines.is_empty() {
                            String::new()
                        } else {
                            left_lines.join("\n") + "\n"
                        };
                        let base_text = if base_lines.is_empty() {
                            String::new()
                        } else {
                            base_lines.join("\n") + "\n"
                        };
                        let right_text = if right_lines.is_empty() {
                            String::new()
                        } else {
                            right_lines.join("\n") + "\n"
                        };

                        let result = compute_three_way_diff(&base_text, &left_text, &right_text);

                        if !result.conflict_positions.is_empty() {
                            let ci = rng.range(result.conflict_positions.len());
                            let pos = result.conflict_positions[ci];
                            let use_left = rng.range(2) == 0;

                            // Find all lines in this block (same status run starting at pos)
                            let block_status = result.lines[pos].status.clone();
                            let mut block_end = pos + 1;
                            while block_end < result.lines.len()
                                && result.lines[block_end].status == block_status
                            {
                                block_end += 1;
                            }

                            // Apply copy: update base_lines from the chosen side
                            for line in &result.lines[pos..block_end] {
                                if let Some(base_no) = line.base_line_no {
                                    let idx = (base_no - 1) as usize;
                                    if idx < base_lines.len() {
                                        base_lines[idx] = if use_left {
                                            line.left_text.clone()
                                        } else {
                                            line.right_text.clone()
                                        };
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Final sanity check: compute one last diff and verify rebuild
            let left_text = if left_lines.is_empty() {
                String::new()
            } else {
                left_lines.join("\n") + "\n"
            };
            let base_text = if base_lines.is_empty() {
                String::new()
            } else {
                base_lines.join("\n") + "\n"
            };
            let right_text = if right_lines.is_empty() {
                String::new()
            } else {
                right_lines.join("\n") + "\n"
            };

            let result = compute_three_way_diff(&base_text, &left_text, &right_text);
            let rebuilt_left = rebuild_pane_text(&result.lines, 0);
            let rebuilt_base = rebuild_pane_text(&result.lines, 1);
            let rebuilt_right = rebuild_pane_text(&result.lines, 2);

            assert_eq!(
                rebuilt_left, left_text,
                "seed={}: final left mismatch",
                seed
            );
            assert_eq!(
                rebuilt_base, base_text,
                "seed={}: final base mismatch",
                seed
            );
            assert_eq!(
                rebuilt_right, right_text,
                "seed={}: final right mismatch",
                seed
            );
        }
    }

    /// Targeted stress test: copy a conflict block then immediately F5.
    /// The middle (base) pane text must NEVER lose lines after this sequence.
    /// This directly addresses the reported bug where pressing F5 after copy
    /// caused the middle pane text to disappear.
    #[test]
    fn test_copy_then_f5_base_preserved() {
        struct Rng(u64);
        impl Rng {
            fn next(&mut self) -> u64 {
                self.0 = self
                    .0
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                self.0 >> 33
            }
            fn range(&mut self, max: usize) -> usize {
                if max == 0 {
                    0
                } else {
                    self.next() as usize % max
                }
            }
        }

        let words = ["foo", "bar", "baz", "qux", "hello", "world", "test", ""];

        for seed in 0..200u64 {
            let mut rng = Rng(seed * 99991 + 7);

            // Generate random initial texts (1-8 lines each)
            let gen_lines = |rng: &mut Rng| -> Vec<String> {
                let n = rng.range(8) + 1;
                (0..n)
                    .map(|_| words[rng.range(words.len())].to_string())
                    .collect()
            };

            let mut base_lines = gen_lines(&mut rng);
            let mut left_lines = gen_lines(&mut rng);
            let mut right_lines = gen_lines(&mut rng);

            // Do 50 cycles of: copy a conflict, then F5, verifying base every time
            for step in 0..50 {
                let left_text = left_lines.join("\n") + "\n";
                let base_text = base_lines.join("\n") + "\n";
                let right_text = right_lines.join("\n") + "\n";

                let result = compute_three_way_diff(&base_text, &left_text, &right_text);

                // Verify all 3 panes before copy
                let rebuilt_base = rebuild_pane_text(&result.lines, 1);
                assert_eq!(
                    rebuilt_base, base_text,
                    "seed={} step={}: base text lost BEFORE copy",
                    seed, step
                );

                // Copy a random conflict block (left→base or right→base)
                if !result.conflict_positions.is_empty() {
                    let ci = rng.range(result.conflict_positions.len());
                    let pos = result.conflict_positions[ci];
                    let block_status = result.lines[pos].status.clone();
                    let mut block_end = pos + 1;
                    while block_end < result.lines.len()
                        && result.lines[block_end].status == block_status
                    {
                        block_end += 1;
                    }

                    let use_left = rng.range(2) == 0;
                    for line in &result.lines[pos..block_end] {
                        if let Some(base_no) = line.base_line_no {
                            let idx = (base_no - 1) as usize;
                            if idx < base_lines.len() {
                                base_lines[idx] = if use_left {
                                    line.left_text.clone()
                                } else {
                                    line.right_text.clone()
                                };
                            }
                        }
                    }
                }

                // Immediately F5 after copy — the critical sequence
                let base_text_after = base_lines.join("\n") + "\n";
                let result2 = compute_three_way_diff(&base_text_after, &left_text, &right_text);

                let rebuilt_base2 = rebuild_pane_text(&result2.lines, 1);
                let rebuilt_left2 = rebuild_pane_text(&result2.lines, 0);
                let rebuilt_right2 = rebuild_pane_text(&result2.lines, 2);

                assert_eq!(
                    rebuilt_base2, base_text_after,
                    "seed={} step={}: BASE text lost after copy+F5!\nbase={:?}\nleft={:?}\nright={:?}",
                    seed, step, base_text_after, left_text, right_text
                );
                assert_eq!(
                    rebuilt_left2, left_text,
                    "seed={} step={}: left text lost after copy+F5",
                    seed, step
                );
                assert_eq!(
                    rebuilt_right2, right_text,
                    "seed={} step={}: right text lost after copy+F5",
                    seed, step
                );

                // Randomly edit one pane to create new diffs for next iteration
                let pane = rng.range(3);
                let lines = match pane {
                    0 => &mut left_lines,
                    1 => &mut base_lines,
                    _ => &mut right_lines,
                };
                if !lines.is_empty() {
                    let idx = rng.range(lines.len());
                    lines[idx] = words[rng.range(words.len())].to_string();
                }
            }
        }
    }

    /// Exact repro: user scenario where middle pane loses "aaaa" after F5.
    /// Steps:
    ///   1. Left="aaa\naaaa\n", Right="aaa\n", Base="" (new blank doc)
    ///   2. Copy all left→base → Base="aaa\naaaa\n"
    ///   3. Edit right: add "ccc" below "aaa" → Right="aaa\nccc\n"
    ///   4. F5 (rescan) with Base="aaa\naaaa\n", Left="aaa\naaaa\n", Right="aaa\nccc\n"
    ///   5. Base must still contain "aaa\naaaa\n"
    #[test]
    fn test_repro_copy_all_then_edit_right_then_f5() {
        // Step 1: initial state — new blank document, user types in left and right
        let base_initial = "\n";
        let left = "aaa\naaaa\n";
        let right_initial = "aaa\n";

        // Step 1 check: initial diff
        let r1 = compute_three_way_diff(base_initial, left, right_initial);
        let rb1 = rebuild_pane_text(&r1.lines, 1);
        assert_eq!(rb1, base_initial, "step1: base mismatch");

        // Step 2: copy all left→base
        let base_after_copy = "aaa\naaaa\n";

        // Step 3: edit right — add "ccc" under "aaa"
        let right_after_edit = "aaa\nccc\n";

        // Step 4: F5 rescan — THIS IS WHERE THE BUG WAS REPORTED
        let r2 = compute_three_way_diff(base_after_copy, left, right_after_edit);

        let rebuilt_base = rebuild_pane_text(&r2.lines, 1);
        let rebuilt_left = rebuild_pane_text(&r2.lines, 0);
        let rebuilt_right = rebuild_pane_text(&r2.lines, 2);

        assert_eq!(
            rebuilt_base, base_after_copy,
            "CRITICAL: base text lost after F5!\nexpected: {:?}\ngot: {:?}\nlines: {:#?}",
            base_after_copy, rebuilt_base, r2.lines
        );
        assert_eq!(rebuilt_left, left, "left text mismatch after F5");
        assert_eq!(
            rebuilt_right, right_after_edit,
            "right text mismatch after F5"
        );

        // Also verify line-by-line that "aaaa" exists in base
        let base_has_aaaa = r2
            .lines
            .iter()
            .any(|l| l.base_line_no.is_some() && l.base_text == "aaaa");
        assert!(base_has_aaaa, "base must contain 'aaaa' line after F5");
    }

    /// Edge cases: single-line, empty, and all-identical texts
    #[test]
    fn test_edge_cases_text_preservation() {
        let cases: Vec<(&str, &str, &str)> = vec![
            // All empty
            ("\n", "\n", "\n"),
            // Single line, all same
            ("x\n", "x\n", "x\n"),
            // Single line, all different
            ("a\n", "b\n", "c\n"),
            // One side empty
            ("\n", "a\nb\n", "a\nb\n"),
            ("a\nb\n", "\n", "a\nb\n"),
            ("a\nb\n", "a\nb\n", "\n"),
            // Large text differences
            ("a\nb\nc\nd\ne\nf\n", "x\n", "a\nb\nc\nd\ne\nf\n"),
            ("x\n", "a\nb\nc\nd\ne\nf\n", "y\n"),
            // Repeated lines
            ("a\na\na\n", "a\na\n", "a\na\na\na\n"),
            // All completely different, varying lengths
            ("a\nb\n", "x\ny\nz\n", "1\n2\n3\n4\n"),
        ];

        for (i, (base, left, right)) in cases.iter().enumerate() {
            let result = compute_three_way_diff(base, left, right);

            let rebuilt_left = rebuild_pane_text(&result.lines, 0);
            let rebuilt_base = rebuild_pane_text(&result.lines, 1);
            let rebuilt_right = rebuild_pane_text(&result.lines, 2);

            assert_eq!(rebuilt_left, *left, "case {}: left mismatch", i);
            assert_eq!(rebuilt_base, *base, "case {}: base mismatch", i);
            assert_eq!(rebuilt_right, *right, "case {}: right mismatch", i);
        }
    }
}
