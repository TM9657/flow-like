use flow_like_types::{anyhow, Context, Result};
use calamine::{open_workbook_auto, Data, Range, Reader};
use csv::WriterBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::cmp::{max, min};
use std::path::{Path};
use std::collections::VecDeque;
use strsim::jaro_winkler;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Days};

/// ============================ Config ============================

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ExtractConfig {
    /// CSV delimiter (`,` or `;` typical)
    pub delimiter: u8,
    /// Use CRLF line endings (Excel-friendly) vs LF
    pub crlf: bool,
    /// Write UTF-8 BOM (Excel-friendly on Windows)
    pub bom: bool,
    /// Protect against CSV injection by prefixing a single quote to dangerous prefixes (=+-@)
    pub csv_injection_hardening: bool,
    /// Consider a row/col "empty" if non-empty density below this (0.0..1.0)
    pub empty_density_threshold: f32,
    /// Break between tables if we see >= this many consecutive empty rows
    pub gap_break_rows: usize,
    /// Break between tables if we see >= this many consecutive empty cols
    pub gap_break_cols: usize,
    /// Allow up to this many blank rows *inside* a table
    pub allow_internal_blank_rows: usize,
    /// Allow up to this many blank cols *inside* a table
    pub allow_internal_blank_cols: usize,
    /// Minimum non-empty cell count for a rectangle to be considered a table
    pub min_table_cells: usize,
    /// Max header rows to consider when flattening
    pub max_header_rows: usize,
    /// Max left header columns to detect
    pub max_left_header_cols: usize,
    /// Joiner to flatten multi-row headers
    pub header_joiner: String,
    /// Drop totals rows matching regex (case-insensitive)
    pub drop_totals: bool,
    /// Allow merging rectangles across large blank/merged "spacer" bands
    pub stitch_across_spacers: bool,
    /// Sample size for type inference per column when stitching
    pub stitch_type_sample_rows: usize,
    /// Minimum ratio of compatible column types to allow stitching (0.0..1.0)
    pub stitch_min_type_match_ratio: f32,
    /// Minimum required column-overlap (intersection / union) to consider adjacency
    pub stitch_min_col_overlap: f32,
    /// After extraction, merge non-adjacent tables if headers are very similar
    pub group_similar_headers: bool,
    /// Similarity threshold (0.0..1.0) to merge tables by header
    pub header_merge_threshold: f64,
    /// Transform every field to a string? -> Tables with changing types
    pub string_fields: bool,
}

impl Default for ExtractConfig {
    fn default() -> Self {
        Self {
            delimiter: b',',
            crlf: false,
            bom: false,
            csv_injection_hardening: true,
            empty_density_threshold: 0.05,
            gap_break_rows: 2,
            gap_break_cols: 2,
            allow_internal_blank_rows: 1,
            allow_internal_blank_cols: 1,
            min_table_cells: 8,
            max_header_rows: 3,
            max_left_header_cols: 3,
            header_joiner: " / ".to_string(),
            drop_totals: true,
            stitch_across_spacers: true,
            stitch_type_sample_rows: 16,
            stitch_min_type_match_ratio: 0.80,
            stitch_min_col_overlap: 0.70,
            group_similar_headers: true,
            header_merge_threshold: 0.97,
            string_fields: false,
        }
    }
}

/// ============================ Public API ============================

/// Extract all tables on a given sheet to CSV strings.
pub fn extract_tables_to_csv<P: AsRef<Path>>(
    path: P,
    sheet_name: &str,
    cfg: &ExtractConfig,
) -> Result<Vec<String>> {
    // 1) Read values via calamine
    let (grid_raw, height, width) = read_sheet_grid(&path, sheet_name)?;

    // 2) Read merges via umya-spreadsheet
    let merges = read_merged_cells(&path, sheet_name)
        .with_context(|| "Reading merged cells with umya-spreadsheet failed")?;

    // Build a merge map so we can treat merged cells smartly during extraction
    let merge_map = build_merge_map(height, width, &merges);

    // 3) Apply merges (propagate top-left) for segmentation/heuristics
    let mut grid = apply_merges(grid_raw, &merges);

    // 4) Segment into rectangles via row/col density cuts
    let rects_coarse = segment_rectangles(&grid, height, width, cfg);

    // 4b) Within each coarse rectangle, split by connectivity to avoid fusing islands.
    let mut rects: Vec<Rect> = Vec::new();
    for r in rects_coarse {
        let parts = split_rect_by_connectivity(&grid, &r, cfg);
        rects.extend(parts);
    }

    // 5) Build tables per rectangle
    let mut built: Vec<TableWithRect> = Vec::new();
    for rect in rects {
        if count_nonempty_in_rect(&grid, &rect) < cfg.min_table_cells {
            continue;
        }
    let table = build_table_from_rect(&mut grid, &rect, cfg, &merge_map);
        built.push(TableWithRect { rect, table });
    }

    // 6) Stitch across big blank/merged spacer bands if schema didn't change
    let stitched = stitch_tables(&grid, built, cfg);

    // 6b) Optionally group and merge by similar headers (non-adjacent, same sheet)
    let grouped = if cfg.group_similar_headers {
        group_tables_by_header_similarity(stitched, cfg)
    } else {
        stitched
    };

    // 7) Render CSVs
    let mut csvs = Vec::new();
    for twr in grouped {
        if let Some(csv) = render_table_to_csv(&twr.table, cfg)? {
            csvs.push(csv);
        }
    }
    Ok(csvs)
}

/// ============================ Types ============================

#[derive(Clone, Debug)]
struct Rect {
    r0: usize,
    c0: usize,
    r1: usize, // inclusive
    c1: usize, // inclusive
}

#[derive(Clone, Debug)]
struct Table {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

/// ============================ IO helpers ============================

fn read_sheet_grid<P: AsRef<Path>>(path: P, sheet: &str) -> Result<(Vec<Vec<String>>, usize, usize)> {
    let mut wb = open_workbook_auto(&path)
        .with_context(|| format!("Opening workbook {:?}", path.as_ref()))?;

    let range: Range<Data> = wb
        .worksheet_range(sheet)?;

    let height = range.get_size().0; // rows
    let width = range.get_size().1;  // cols
    let mut grid = vec![vec![String::new(); width]; height];

    for (r, row) in range.rows().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            grid[r][c] = data_to_string_iso(cell, false);
        }
    }
    Ok((grid, height, width))
}

fn excel_serial_to_iso(serial: f64, is_1904: bool) -> String {
    let days = serial.floor() as i64;
    let secs = ((serial - serial.floor()) * 86_400.0).round() as i64;

    // Excel epoch handling (with the 1900 leap-year bug)
    let base = if is_1904 {
        NaiveDate::from_ymd_opt(1904, 1, 1).unwrap()
    } else {
        // Serial 0 corresponds to 1899-12-30 in practice
        NaiveDate::from_ymd_opt(1899, 12, 30).unwrap()
    };

    let mut date = base + Days::new(days as u64);
    if !is_1904 && days >= 60 {
        // Skip the non-existent 1900-02-29
        date = date + Days::new(1);
    }

    let secs_norm = ((secs % 86_400) + 86_400) % 86_400;
    let time = NaiveTime::from_num_seconds_from_midnight_opt(secs_norm as u32, 0).unwrap();
    NaiveDateTime::new(date, time).format("%Y-%m-%dT%H:%M:%S").to_string()
}

fn data_to_string_iso(v: &Data, is_1904: bool) -> String {
    match v {
        Data::DateTime(serial) => excel_serial_to_iso(serial.as_f64(), is_1904),
        Data::DateTimeIso(s)   => s.clone(),
        Data::DurationIso(s)   => s.clone(),
        _ => data_to_string(v),
    }
}

fn data_to_string(v: &Data) -> String {
    match v {
        Data::Empty => String::new(),
        Data::String(s) => s.clone(),
        Data::Float(f) => {
            if f.fract() == 0.0
                && *f >= -9_007_199_254_740_992.0
                && *f <=  9_007_199_254_740_992.0
            {
                format!("{:.0}", f)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "TRUE".to_string() } else { "FALSE".to_string() },
        Data::DateTime(serial) => serial.to_string(),
        Data::DateTimeIso(s) => s.clone(),
        Data::DurationIso(s) => s.clone(),
        Data::Error(e) => format!("#ERROR:{:?}", e),
    }
}

/// ============================ Merges ============================

#[derive(Clone, Copy, Debug)]
struct Merge {
    r0: usize, c0: usize,
    r1: usize, c1: usize,
}

// Lightweight merge map for quick lookup.
type MergeMap = Vec<Vec<Option<Merge>>>;

fn build_merge_map(height: usize, width: usize, merges: &[Merge]) -> MergeMap {
    let mut map = vec![vec![None; width]; height];
    if height == 0 || width == 0 { return map; }

    for m in merges {
        // Clamp merge bounds to grid to avoid OOB
        let r0 = m.r0.min(height - 1);
        let c0 = m.c0.min(width - 1);
        let r1 = m.r1.min(height - 1);
        let c1 = m.c1.min(width - 1);
        if r0 >= height || c0 >= width { continue; }
        for r in r0..=r1 {
            for c in c0..=c1 {
                map[r][c] = Some(*m);
            }
        }
    }
    map
}

// Returns true when (r,c) belongs to a horizontally-merged area and is NOT the anchor (top-left).
fn is_horiz_merged_non_anchor(mm: &MergeMap, r: usize, c: usize) -> bool {
    if let Some(m) = mm[r][c] {
        (m.c1 > m.c0) && c != m.c0 && r >= m.r0 && r <= m.r1
    } else {
        false
    }
}

fn read_merged_cells<P: AsRef<Path>>(path: P, sheet: &str) -> Result<Vec<Merge>> {
    let book = umya_spreadsheet::reader::xlsx::read(path.as_ref())
        .with_context(|| "umya read failed")?;

    let ws = book
        .get_sheet_by_name(sheet)
        .ok_or_else(|| anyhow!("Sheet not found (umya): {}", sheet))?;

    let merged = ws.get_merge_cells(); // &[umya_spreadsheet::Range]
    let mut out = Vec::new();

    for r in merged {
        // 1) Prefer Display -> A1 like "A1:B2" if available
        let a1 = format!("{:?}", r);
        if let Some((r0, c0, r1, c1)) = parse_a1_range(&a1) {
            out.push(Merge { r0, c0, r1, c1 });
            continue;
        }

        // 2) Fallback: parse the Debug struct you showed
        let dbg = format!("{:?}", r);
        if let Some((r0, c0, r1, c1)) = parse_umya_debug_range(&dbg) {
            out.push(Merge { r0, c0, r1, c1 });
            continue;
        }

        // If both parsing strategies fail, log and skip.
        eprintln!("WARN: couldn't parse merge range: {}", dbg);
    }

    Ok(out)
}

/// Parse umya-spreadsheet Range debug output like:
/// `Range { start_col: Some(ColumnReference { num: 23, ... }),
///          start_row: Some(RowReference { num: 185, ... }),
///          end_col:   Some(ColumnReference { num: 24, ... }),
///          end_row:   Some(RowReference { num: 185, ... }) }`
/// Returns zero-based (r0,c0,r1,c1).
fn parse_umya_debug_range(s: &str) -> Option<(usize, usize, usize, usize)> {
    // Grab the first four `num: <int>` in order: start_col, start_row, end_col, end_row.
    let re = Regex::new(r"num:\s*(\d+)").ok()?;
    let nums: Vec<usize> = re
        .captures_iter(s)
        .filter_map(|cap| cap.get(1)?.as_str().parse::<usize>().ok())
        .collect();

    if nums.len() < 4 {
        return None;
    }

    let (start_col_1b, start_row_1b, end_col_1b, end_row_1b) =
        (nums[0], nums[1], nums[2], nums[3]);

    // Convert 1-based (umya) -> 0-based (your grid)
    let c0 = start_col_1b.saturating_sub(1);
    let r0 = start_row_1b.saturating_sub(1);
    let c1 = end_col_1b.saturating_sub(1);
    let r1 = end_row_1b.saturating_sub(1);

    Some((
        std::cmp::min(r0, r1),
        std::cmp::min(c0, c1),
        std::cmp::max(r0, r1),
        std::cmp::max(c0, c1),
    ))
}

fn col_letters_to_idx(s: &str) -> Option<usize> {
    let mut val: usize = 0;
    for ch in s.chars() {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        val = val * 26 + ((ch.to_ascii_uppercase() as u8 - b'A') as usize + 1);
    }
    Some(val - 1)
}

fn parse_a1_cell(a1: &str) -> Option<(usize, usize)> {
    // e.g., "BC23" -> (22, 54) zero-based
    let mut letters = String::new();
    let mut digits = String::new();
    for ch in a1.chars() {
        if ch.is_ascii_alphabetic() {
            if !digits.is_empty() { return None; }
            letters.push(ch);
        } else if ch.is_ascii_digit() {
            digits.push(ch);
        } else {
            return None;
        }
    }
    let c = col_letters_to_idx(&letters)?;
    let r: usize = digits.parse().ok()?;
    Some((r - 1, c))
}

fn parse_a1_range(r: &str) -> Option<(usize, usize, usize, usize)> {
    let parts: Vec<&str> = r.split(':').collect();
    if parts.len() != 2 { return None; }
    let (r0,c0) = parse_a1_cell(parts[0])?;
    let (r1,c1) = parse_a1_cell(parts[1])?;
    Some((min(r0,r1), min(c0,c1), max(r0,r1), max(c0,c1)))
}

fn apply_merges(mut grid: Vec<Vec<String>>, merges: &[Merge]) -> Vec<Vec<String>> {
    let height = grid.len();
    if height == 0 { return grid; }
    let width = grid[0].len();
    if width == 0 { return grid; }

    for m in merges {
        // Clamp to grid
        let r0 = m.r0.min(height - 1);
        let c0 = m.c0.min(width - 1);
        let r1 = m.r1.min(height - 1);
        let c1 = m.c1.min(width - 1);
        if r0 >= height || c0 >= width { continue; }

        let base = grid.get(r0).and_then(|row| row.get(c0)).cloned().unwrap_or_default();
        if base.is_empty() { continue; } // nothing to propagate
        for r in r0..=r1 {
            for c in c0..=c1 {
                grid[r][c] = base.clone();
            }
        }
    }
    grid
}

/// ============================ Segmentation ============================

fn segment_rectangles(
    grid: &Vec<Vec<String>>,
    height: usize,
    width: usize,
    cfg: &ExtractConfig,
) -> Vec<Rect> {
    if height == 0 || width == 0 { return vec![]; }

    let row_nonempty: Vec<usize> = (0..height)
        .map(|r| grid[r].iter().filter(|s| !s.trim().is_empty()).count())
        .collect();
    let col_nonempty: Vec<usize> = (0..width)
        .map(|c| (0..height).filter(|&r| !grid[r][c].trim().is_empty()).count())
        .collect();

    let row_cuts = find_cuts(&row_nonempty, width, cfg.empty_density_threshold, cfg.gap_break_rows);
    let col_cuts = find_cuts(&col_nonempty, height, cfg.empty_density_threshold, cfg.gap_break_cols);

    // Build rectangles as cartesian of row segments × col segments
    let mut rects = Vec::new();
    for (r0, r1) in row_cuts {
        for (c0, c1) in &col_cuts {
            let rect = Rect { r0, c0: *c0, r1, c1: *c1 };
            // skip rectangles that are effectively empty
            if count_nonempty_in_rect(grid, &rect) > 0 {
                rects.push(rect);
            }
        }
    }
    rects
}

fn find_cuts(
    counts: &Vec<usize>,
    denom: usize,
    thresh: f32,
    gap_break: usize,
) -> Vec<(usize, usize)> {
    // segments are [start,end] inclusive
    let mut out = Vec::new();
    let mut i = 0;
    while i < counts.len() {
        // skip empties to next non-empty
        while i < counts.len() && density(counts[i], denom) <= thresh { i += 1; }
        if i >= counts.len() { break; }
        let start = i;
        // grow until we get >=gap_break consecutive empties
        let mut consec_empty = 0;
        let mut end = i;
        while i < counts.len() {
            if density(counts[i], denom) <= thresh {
                consec_empty += 1;
                if consec_empty >= gap_break { break; }
            } else {
                consec_empty = 0;
                end = i;
            }
            i += 1;
        }
        out.push((start, end));
    }
    if out.is_empty() {
        // single full segment if everything sparse
        out.push((0, counts.len().saturating_sub(1)));
    }
    out
}

#[inline]
fn density(nz: usize, total: usize) -> f32 {
    if total == 0 { 0.0 } else { (nz as f32) / (total as f32) }
}

fn count_nonempty_in_rect(grid: &Vec<Vec<String>>, rect: &Rect) -> usize {
    let mut n = 0;
    for r in rect.r0..=rect.r1 {
        for c in rect.c0..=rect.c1 {
            if !grid[r][c].trim().is_empty() { n += 1; }
        }
    }
    n
}

/// ============================ Table Build ============================

fn build_table_from_rect(
    grid: &mut Vec<Vec<String>>,
    rect: &Rect,
    cfg: &ExtractConfig,
    merge_map: &MergeMap,
) -> Table {
    // detect header band (1..=max_header_rows), skipping merged banner rows
    let header_rows = detect_header_rows(grid, rect, cfg, merge_map);
    // detect left header (0..=max_left_header_cols)
    let left_cols = detect_left_header_cols(grid, rect, &header_rows, cfg);

    // flatten headers
    let mut headers = flatten_headers(grid, rect, &header_rows, left_cols, cfg, merge_map);

    // data rows start after header band
    let mut data_r0 = rect.r0 + header_rows.len();
    let mut rows: Vec<Vec<String>> = Vec::new();

    // regex to drop totals
    let totals_re = Regex::new(r"(?i)^\s*(total|summe|subtotal|gesamt)\b").unwrap();

    // Unit row: if directly under header looks like unit tokens, merge into header
    if data_r0 <= rect.r1 {
        if let Some(unit_row) = detect_unit_row(grid, rect, data_r0, &headers) {
            merge_unit_into_headers(&mut headers, &unit_row);
            data_r0 += 1; // skip the unit row in data
        }
    }

    let mut max_width = headers.len();

    let mut consec_blank_rows = 0usize;
    let blank_allowed = cfg.allow_internal_blank_rows;

    for r in data_r0..=rect.r1 {
        let mut row = Vec::new();
        for c in rect.c0..=rect.c1 {
            let mut v = grid[r][c].clone();
            // Suppress horizontally-merged non-anchor duplicates (keep only anchor column)
            if is_horiz_merged_non_anchor(merge_map, r, c) {
                v.clear();
            }
            row.push(v);
        }
        // pad/clip to left_cols + data columns
        row = project_row_for_headers(row, rect, left_cols, headers.len());

        let nonempty = row.iter().any(|s| !s.trim().is_empty());
        if !nonempty {
            consec_blank_rows += 1;
            if consec_blank_rows <= blank_allowed { continue; }
            else { break; } // assume table ended
        } else {
            consec_blank_rows = 0;
        }

        // drop repeated header rows inside body
        if row_eq_headers(&row, &headers) {
            continue;
        }

        // optionally drop totals rows
        if cfg.drop_totals {
            if let Some(first) = row.get(0) {
                if totals_re.is_match(first) { continue; }
            }
        }

        max_width = max(max_width, row.len());
        rows.push(row);
    }

    // normalize row widths
    for r in &mut rows {
        if r.len() < max_width { r.resize(max_width, String::new()); }
    }

    let mut table = Table { headers, rows };

    // Optional totals column drop
    if cfg.drop_totals {
        drop_totals_column(&mut table);
    }

    // Drop truly identical columns (header modulo " (n)" and identical cell values)
    dedup_identical_columns(&mut table);

    table
}

/// ============================ Units & Totals helpers ============================

fn detect_unit_row(
    grid: &Vec<Vec<String>>,
    rect: &Rect,
    r: usize,
    headers: &Vec<String>,
) -> Option<Vec<String>> {
    if r < rect.r0 || r > rect.r1 { return None; }
    let mut vals: Vec<String> = Vec::new();
    let mut tokens = 0usize;
    let mut nonempty = 0usize;
    for (i, c) in (rect.c0..=rect.c1).enumerate() {
        if i >= headers.len() { break; }
        let s = grid[r][c].trim();
        if s.is_empty() { vals.push(String::new()); continue; }
        nonempty += 1;
        let is_short = s.chars().count() <= 5;
        let has_space = s.contains(char::is_whitespace);
        let symbolic_units = ["%","‰","€","$","¥","£","kg","g","t","h","m","s","km","cm","mm","pcs","stk","l","ml"];
        let is_symbolic = symbolic_units.iter().any(|&u| s.eq_ignore_ascii_case(u));
        if (is_short && !has_space) || is_symbolic { tokens += 1; }
        vals.push(s.to_string());
    }
    if nonempty > 0 && tokens * 2 >= nonempty { Some(vals) } else { None }
}

fn merge_unit_into_headers(headers: &mut Vec<String>, unit_row: &Vec<String>) {
    for (h, u) in headers.iter_mut().zip(unit_row) {
        if u.trim().is_empty() { continue; }
        if h.trim().is_empty() { *h = u.clone(); }
        else { *h = format!("{} [{}]", h, u); }
    }
}

fn drop_totals_column(t: &mut Table) {
    if t.headers.is_empty() { return; }
    let re = Regex::new(r"(?i)^\s*(total|summe|subtotal|gesamt)\b").unwrap();
    let last = t.headers.len() - 1;
    if re.is_match(t.headers[last].as_str()) {
        t.headers.pop();
        for row in &mut t.rows { if row.len() > last { row.pop(); } }
    }
}

fn detect_header_rows(
    grid: &Vec<Vec<String>>,
    rect: &Rect,
    cfg: &ExtractConfig,
    merge_map: &MergeMap,
) -> Vec<usize> {
    let mut out = Vec::new();

    // banner skip stays as-is
    let mut start_r = rect.r0;
    for _ in 0..2 {
        if is_banner_row(grid, rect, start_r, merge_map) { start_r += 1; } else { break; }
    }

    let max = cfg.max_header_rows.min(rect.r1.saturating_sub(start_r) + 1);
    for i in 0..max {
        let r = start_r + i;
        let (alpha, numeric, nonempty) = row_type_stats(&grid[r], rect.c0, rect.c1);
        if nonempty == 0 { break; }

        // Stricter header criterion
        let looks_header = alpha >= 0.60 && numeric <= 0.30;
        if !looks_header { break; }

        // Guard: if the NEXT row looks similarly "alpha-ish", it's probably data — stop here.
        if i > 0 || r + 1 <= rect.r1 {
            if r + 1 <= rect.r1 {
                let (alpha_next, numeric_next, ne_next) = row_type_stats(&grid[r + 1], rect.c0, rect.c1);
                if ne_next > 0 {
                    // If next row is close in alpha-ness or quite numeric, don't keep stacking headers.
                    let similar_alpha = (alpha_next - alpha).abs() < 0.20;
                    if similar_alpha || numeric_next >= 0.34 {
                        out.push(r - rect.r0);
                        break;
                    }
                }
            }
        }

        out.push(r - rect.r0);
    }

    if out.is_empty() {
        // fallback: first non-empty row with decent alpha
        for r in start_r..=rect.r1 {
            let (alpha, _numeric, ne) = row_type_stats(&grid[r], rect.c0, rect.c1);
            if ne > 0 && alpha >= 0.60 { out.push(r - rect.r0); }
            break;
        }
    }
    out
}

fn is_banner_row(
    grid: &Vec<Vec<String>>,
    rect: &Rect,
    r: usize,
    merge_map: &MergeMap,
) -> bool {
    if r < rect.r0 || r > rect.r1 { return false; }
    let width = rect.c1.saturating_sub(rect.c0) + 1;
    if width < 3 { return false; }
    let min_span = std::cmp::max(2, width / 2); // span at least half the rect

    // Check for a merged anchor at this row that spans wide columns and has text
    for c in rect.c0..=rect.c1 {
        if let Some(m) = merge_map[r][c] {
            if m.r0 == r && m.c0 == c {
                let span = m.c1.saturating_sub(m.c0) + 1;
                if span >= min_span {
                    let s = grid[r][c].trim();
                    if !s.is_empty() { return true; }
                }
            }
        }
    }

    // Fallback: entire row has exactly one non-empty cell and others empty
    let mut nonempty_count = 0usize;
    for c in rect.c0..=rect.c1 { if !grid[r][c].trim().is_empty() { nonempty_count += 1; } }
    nonempty_count == 1
}

fn row_type_stats(row: &Vec<String>, c0: usize, c1: usize) -> (f32, f32, usize) {
    let mut alpha = 0usize;
    let mut numeric = 0usize;
    let mut nonempty = 0usize;
    for c in c0..=c1 {
        let s = row[c].trim();
        if s.is_empty() { continue; }
        nonempty += 1;
        let has_digit = s.chars().any(|ch| ch.is_ascii_digit());
        let has_alpha = s.chars().any(|ch| ch.is_ascii_alphabetic());
        if has_alpha { alpha += 1; }
        if has_digit && !has_alpha { numeric += 1; }
    }
    let w = max(1, (c1 + 1).saturating_sub(c0));
    (alpha as f32 / w as f32, numeric as f32 / w as f32, nonempty)
}

fn detect_left_header_cols(
    grid: &Vec<Vec<String>>,
    rect: &Rect,
    header_rows: &Vec<usize>,
    cfg: &ExtractConfig,
) -> usize {
    let mut count = 0usize;
    'cols: for c in rect.c0..=rect.c1 {
        if count >= cfg.max_left_header_cols { break; }
        let mut textish = 0usize;
        let mut nonempty = 0usize;
        let mut repeats = 0usize;
        let mut prev: Option<String> = None;
        for r in (rect.r0 + header_rows.len())..=rect.r1 {
            let s = grid[r][c].trim();
            if s.is_empty() { continue; }
            nonempty += 1;
            if s.chars().any(|ch| ch.is_ascii_alphabetic()) { textish += 1; }
            if let Some(p) = &prev {
                if p == s { repeats += 1; }
            }
            prev = Some(s.to_string());
        }
        // Heuristic: mostly text labels and some repeats (categories)
        if nonempty > 0 && textish * 2 >= nonempty && repeats >= nonempty / 4 {
            count += 1;
        } else {
            break 'cols;
        }
    }
    count
}

fn flatten_headers(
    grid: &Vec<Vec<String>>,
    rect: &Rect,
    header_rows: &Vec<usize>,
    left_cols: usize,
    cfg: &ExtractConfig,
    merge_map: &MergeMap,
) -> Vec<String> {
    let mut headers = Vec::new();
    for c in rect.c0..=rect.c1 {
        let mut parts = Vec::new();
        for off in header_rows {
            let r = rect.r0 + *off;
            let mut s = grid[r][c].clone();

            // Suppress horizontally-merged non-anchor duplicates across header rows
            if is_horiz_merged_non_anchor(merge_map, r, c) {
                s.clear();
            }

            if s.contains('\n') {
                let cleaned = s.replace('\r', "");
                let mut buf = String::new();
                for (i, part) in cleaned.split('\n').map(str::trim).filter(|x| !x.is_empty()).enumerate() {
                    if i > 0 { buf.push_str(&cfg.header_joiner); }
                    buf.push_str(part);
                }
                s = buf;
            }
            if !s.trim().is_empty() {
                parts.push(s.trim().to_string());
            }
        }
        let name = if parts.is_empty() { String::new() } else { parts.join(&cfg.header_joiner) };
        headers.push(name);
    }

    // If *all* headers are empty, keep full rect width (do NOT truncate).
    if headers.iter().any(|h| !h.trim().is_empty()) {
        // Otherwise trim only trailing *all-empty* columns.
        let mut last_nonempty = headers.len().saturating_sub(1);
        while last_nonempty > 0 && headers[last_nonempty].trim().is_empty() {
            last_nonempty -= 1;
        }
        headers.truncate(last_nonempty + 1);
    }

    // Ensure left header columns get reasonable names if empty
    for i in 0..left_cols.min(headers.len()) {
        if headers[i].trim().is_empty() {
            headers[i] = format!("RowHeader{}", i + 1);
        }
    }

    // Dedup duplicate header names
    let mut seen = std::collections::HashMap::<String, usize>::new();
    for h in &mut headers {
        let base = h.trim();
        if base.is_empty() {
            *h = "Column".to_string();
            continue;
        }
        let n = seen.entry(base.to_string()).or_insert(0);
        if *n > 0 {
            *h = format!("{} ({})", base, *n + 1);
        } else {
            *h = base.to_string();
        }
        *n += 1;
    }
    headers
}

fn project_row_for_headers(
    row: Vec<String>,
    _rect: &Rect,
    _left_cols: usize,
    header_len: usize,
) -> Vec<String> {
    // Row already represents rect.c0..=rect.c1 in order, so slice from 0
    let end = min(header_len, row.len());
    let mut out = row[0..end].to_vec();
    // Ensure left_cols exist (already in range). Pad if necessary.
    if out.len() < header_len {
        out.resize(header_len, String::new());
    }
    out
}

fn row_eq_headers(row: &Vec<String>, headers: &Vec<String>) -> bool {
    if row.len() != headers.len() { return false; }
    for (a, b) in row.iter().zip(headers) {
        if normalize(a) != normalize(b) { return false; }
    }
    true
}

/// ============================ Connectivity split ============================

fn is_nonempty(s: &str) -> bool { !s.trim().is_empty() }

fn split_rect_by_connectivity(
    grid: &Vec<Vec<String>>,
    rect: &Rect,
    cfg: &ExtractConfig,
) -> Vec<Rect> {
    let mut visited: Vec<Vec<bool>> = vec![vec![false; rect.c1 - rect.c0 + 1]; rect.r1 - rect.r0 + 1];
    let mut parts: Vec<Rect> = Vec::new();

    for r in rect.r0..=rect.r1 {
        for c in rect.c0..=rect.c1 {
            if !is_nonempty(&grid[r][c]) { continue; }
            let vr = r - rect.r0;
            let vc = c - rect.c0;
            if visited[vr][vc] { continue; }

            let mut q: VecDeque<(usize, usize)> = VecDeque::new();
            q.push_back((r, c));
            visited[vr][vc] = true;

            let mut comp_r0 = r;
            let mut comp_c0 = c;
            let mut comp_r1 = r;
            let mut comp_c1 = c;

            while let Some((cr, cc)) = q.pop_front() {
                comp_r0 = std::cmp::min(comp_r0, cr);
                comp_c0 = std::cmp::min(comp_c0, cc);
                comp_r1 = std::cmp::max(comp_r1, cr);
                comp_c1 = std::cmp::max(comp_c1, cc);

                // Explore immediate neighbors (4-neighborhood)
                let neigh = [
                    (cr.wrapping_sub(1), cc, cr > rect.r0),
                    (cr + 1, cc, cr < rect.r1),
                    (cr, cc.wrapping_sub(1), cc > rect.c0),
                    (cr, cc + 1, cc < rect.c1),
                ];
                for &(nr, nc, ok) in &neigh {
                    if !ok { continue; }
                    if is_nonempty(&grid[nr][nc]) {
                        let vr = nr - rect.r0; let vc = nc - rect.c0;
                        if !visited[vr][vc] {
                            visited[vr][vc] = true;
                            q.push_back((nr, nc));
                        }
                    }
                }

                // Horizontal bridging right
                let mut gaps = 0usize;
                let mut x = cc + 1;
                while x <= rect.c1 && gaps < cfg.allow_internal_blank_cols {
                    if is_nonempty(&grid[cr][x]) { break; }
                    gaps += 1; x += 1;
                }
                if x <= rect.c1 && gaps > 0 && gaps <= cfg.allow_internal_blank_cols && is_nonempty(&grid[cr][x]) {
                    let vr = cr - rect.r0; let vc = x - rect.c0;
                    if !visited[vr][vc] { visited[vr][vc] = true; q.push_back((cr, x)); }
                }

                // Horizontal bridging left
                gaps = 0; let mut x2 = cc;
                while x2 > rect.c0 {
                    let next = x2 - 1;
                    if is_nonempty(&grid[cr][next]) { break; }
                    gaps += 1; x2 = next;
                    if gaps >= cfg.allow_internal_blank_cols { break; }
                }
                if x2 > rect.c0.saturating_sub(1) && gaps > 0 {
                    let target = x2 - 1;
                    if target >= rect.c0 && is_nonempty(&grid[cr][target]) {
                        let vr = cr - rect.r0; let vc = target - rect.c0;
                        if !visited[vr][vc] { visited[vr][vc] = true; q.push_back((cr, target)); }
                    }
                }

                // Vertical bridging down
                gaps = 0; let mut y = cr + 1;
                while y <= rect.r1 && gaps < cfg.allow_internal_blank_rows {
                    if is_nonempty(&grid[y][cc]) { break; }
                    gaps += 1; y += 1;
                }
                if y <= rect.r1 && gaps > 0 && gaps <= cfg.allow_internal_blank_rows && is_nonempty(&grid[y][cc]) {
                    let vr = y - rect.r0; let vc = cc - rect.c0;
                    if !visited[vr][vc] { visited[vr][vc] = true; q.push_back((y, cc)); }
                }

                // Vertical bridging up
                gaps = 0; let mut y2 = cr;
                while y2 > rect.r0 {
                    let next = y2 - 1;
                    if is_nonempty(&grid[next][cc]) { break; }
                    gaps += 1; y2 = next;
                    if gaps >= cfg.allow_internal_blank_rows { break; }
                }
                if y2 > rect.r0.saturating_sub(1) && gaps > 0 {
                    let target = y2 - 1;
                    if target >= rect.r0 && is_nonempty(&grid[target][cc]) {
                        let vr = target - rect.r0; let vc = cc - rect.c0;
                        if !visited[vr][vc] { visited[vr][vc] = true; q.push_back((target, cc)); }
                    }
                }
            }

            parts.push(Rect { r0: comp_r0, c0: comp_c0, r1: comp_r1, c1: comp_c1 });
        }
    }

    // Small optimization: if no split happened, return the input rect.
    if parts.len() <= 1 {
        return vec![Rect { r0: rect.r0, c0: rect.c0, r1: rect.r1, c1: rect.c1 }];
    }
    parts
}

fn normalize(s: &str) -> String {
    s.trim().to_ascii_lowercase().replace(char::is_whitespace, "")
}

/// ============================ CSV render ============================

fn render_table_to_csv(table: &Table, cfg: &ExtractConfig) -> Result<Option<String>> {
    if table.rows.is_empty() && table.headers.iter().all(|h| h.trim().is_empty()) {
        return Ok(None);
    }
    let mut buf: Vec<u8> = Vec::new();
    if cfg.bom {
        buf.extend_from_slice(&[0xEF, 0xBB, 0xBF]);
    }
    let mut wtr = WriterBuilder::new()
        .delimiter(cfg.delimiter)
        .terminator(if cfg.crlf {
            csv::Terminator::CRLF
        } else {
            csv::Terminator::Any(b'\n') // use Any(u8) for LF
        })
        .from_writer(vec![]);

    let mut header = table.headers.clone();
    harden_fields(&mut header, cfg.csv_injection_hardening);
    wtr.write_record(header)?;

    for mut row in table.rows.clone() {
        harden_fields(&mut row, cfg.csv_injection_hardening);
        wtr.write_record(row)?;
    }
    let mut inner = wtr.into_inner()?;
    buf.append(&mut inner);
    let s = String::from_utf8(buf).context("CSV not UTF-8")?;
    Ok(Some(s))
}

fn harden_fields(fields: &mut [String], enable: bool) {
    if !enable { return; }
    for f in fields {
        let s = f.trim_start();
        if let Some(ch) = s.chars().next() {
            if matches!(ch, '=' | '+' | '-' | '@') {
                f.insert(0, '\'');
            }
        }
    }
}

#[derive(Clone, Debug)]
struct TableWithRect {
    rect: Rect,
    table: Table,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum Kind { Empty, Bool, DateTime, Numeric, Text, Mixed }

fn detect_kind(s: &str) -> Kind {
    let t = s.trim();
    if t.is_empty() { return Kind::Empty; }
    let tl = t.to_ascii_lowercase();
    if tl == "true" || tl == "false" { return Kind::Bool; }
    // very light ISO checks: YYYY-MM-DD or YYYY-MM-DDTHH:MM:SS
    if (t.len() >= 10 && t.chars().nth(4) == Some('-') && t.chars().nth(7) == Some('-'))
        || (t.len() >= 19 && t.chars().nth(10) == Some('T'))
    {
        return Kind::DateTime;
    }
    if t.parse::<i64>().is_ok() || t.parse::<f64>().is_ok() { return Kind::Numeric; }
    Kind::Text
}

fn infer_col_kinds(table: &Table, sample: usize) -> Vec<Kind> {
    let w = table.headers.len();
    let mut kinds = vec![Kind::Empty; w];
    for c in 0..w {
        let mut seen = std::collections::HashSet::<Kind>::new();
        for r in 0..table.rows.len().min(sample) {
            let k = detect_kind(table.rows[r].get(c).map(|s| s.as_str()).unwrap_or(""));
            if k != Kind::Empty { seen.insert(match k { Kind::Numeric => Kind::Numeric, _ => k }); }
            if seen.len() > 1 { // early mixed
                kinds[c] = Kind::Mixed;
                break;
            }
        }
        if kinds[c] != Kind::Mixed {
            kinds[c] = seen.iter().next().copied().unwrap_or(Kind::Empty);
        }
    }
    kinds
}

fn kinds_match_ratio(a: &[Kind], b: &[Kind]) -> f32 {
    let w = a.len().min(b.len());
    if w == 0 { return 1.0; }
    let mut considered = 0usize;
    let mut ok = 0usize;
    for i in 0..w {
        let (ka, kb) = (a[i], b[i]);
        // Empty is neutral, ignore unless both are empty
        if ka == Kind::Empty && kb == Kind::Empty { continue; }
        considered += 1;
        let compat = ka == kb
            || (ka == Kind::Numeric && kb == Kind::Numeric)
            || ka == Kind::Empty
            || kb == Kind::Empty;
        if compat { ok += 1; }
    }
    if considered == 0 { 1.0 } else { ok as f32 / considered as f32 }
}

fn col_overlap_ratio(a: &Rect, b: &Rect) -> f32 {
    let inter_c0 = std::cmp::max(a.c0, b.c0);
    let inter_c1 = std::cmp::min(a.c1, b.c1);
    let inter = if inter_c1 >= inter_c0 { inter_c1 - inter_c0 + 1 } else { 0 };
    let uni_c0 = std::cmp::min(a.c0, b.c0);
    let uni_c1 = std::cmp::max(a.c1, b.c1);
    let uni  = uni_c1 - uni_c0 + 1;
    if uni == 0 { 0.0 } else { inter as f32 / uni as f32 }
}

fn gap_has_headerish(
    grid: &Vec<Vec<String>>,
    a: &Rect,
    b: &Rect,
) -> bool {
    if b.r0 <= a.r1 + 1 { return false; }
    let c0 = std::cmp::max(a.c0, b.c0);
    let c1 = std::cmp::min(a.c1, b.c1);
    if c1 < c0 { return false; }
    for r in (a.r1 + 1)..b.r0 {
        let (alpha, numeric, nonempty) = row_type_stats(&grid[r], c0, c1);
        let looks_header = (alpha >= 0.5 && numeric <= 0.5) || (nonempty > 0 && alpha >= 0.3);
        if looks_header { return true; }
    }
    false
}

fn merge_tables(mut a: Table, mut b: Table) -> Table {
    let target_len = a.headers.len().max(b.headers.len());

    if a.headers.len() < target_len { a.headers.resize(target_len, String::new()); }
    if b.headers.len() < target_len { b.headers.resize(target_len, String::new()); }

    for r in &mut a.rows { if r.len() < target_len { r.resize(target_len, String::new()); } }
    for r in &mut b.rows { if r.len() < target_len { r.resize(target_len, String::new()); } }

    // Prefer non-empty header set; otherwise keep `a`'s.
    let a_has_names = a.headers.iter().any(|h| !h.trim().is_empty());
    let b_has_names = b.headers.iter().any(|h| !h.trim().is_empty());
    let headers = if a_has_names || !b_has_names { a.headers.clone() } else { b.headers.clone() };

    let mut rows = a.rows;
    rows.extend(b.rows.into_iter());
    Table { headers, rows }
}

fn can_stitch(
    grid: &Vec<Vec<String>>,
    prev: &TableWithRect,
    next: &TableWithRect,
    cfg: &ExtractConfig,
) -> bool {
    if col_overlap_ratio(&prev.rect, &next.rect) < cfg.stitch_min_col_overlap {
        return false;
    }
    if gap_has_headerish(grid, &prev.rect, &next.rect) {
        return false; // a real header showed up between them
    }

    // Either headers are similar, or types mostly match.
    let headers_ok = headers_similar(&prev.table.headers, &next.table.headers);

    let prev_k = infer_col_kinds(&prev.table, cfg.stitch_type_sample_rows);
    let next_k = infer_col_kinds(&next.table, cfg.stitch_type_sample_rows);
    let type_ratio = kinds_match_ratio(&prev_k, &next_k);

    headers_ok || type_ratio >= cfg.stitch_min_type_match_ratio
}

fn stitch_tables(
    grid: &Vec<Vec<String>>,
    mut items: Vec<TableWithRect>,
    cfg: &ExtractConfig,
) -> Vec<TableWithRect> {
    if items.is_empty() { return items; }
    items.sort_by_key(|t| (t.rect.r0, t.rect.c0));

    let mut out: Vec<TableWithRect> = Vec::new();
    let mut cur = items.remove(0);

    for nxt in items.into_iter() {
        if cfg.stitch_across_spacers && can_stitch(grid, &cur, &nxt, cfg) {
            // merge
            cur.table = merge_tables(cur.table, nxt.table);
            cur.rect = Rect {
                r0: std::cmp::min(cur.rect.r0, nxt.rect.r0),
                c0: std::cmp::min(cur.rect.c0, nxt.rect.c0),
                r1: std::cmp::max(cur.rect.r1, nxt.rect.r1),
                c1: std::cmp::max(cur.rect.c1, nxt.rect.c1),
            };
        } else {
            out.push(cur);
            cur = nxt;
        }
    }
    out.push(cur);
    out
}

/// ============================ (Optional) Schema stitching ============================
/// Example: if two rectangles are separated by 2 blank rows but headers are "the same",
/// you could merge them. Hook this in `segment_rectangles` if desired.
fn headers_similar(a: &[String], b: &[String]) -> bool {
    let w = min(a.len(), b.len());
    if w == 0 { return false; }
    let mut score = 0.0;
    for i in 0..w {
        let sa = a[i].to_ascii_lowercase();
        let sb = b[i].to_ascii_lowercase();
        score += jaro_winkler(&sa, &sb);
    }
    (score / (w as f64)) >= 0.92
}

/// Compute average Jaro-Winkler similarity of aligned headers (by index) between 0.0 and 1.0.
fn headers_similarity(a: &[String], b: &[String]) -> f64 {
    let w = min(a.len(), b.len());
    if w == 0 { return 0.0; }
    let mut score = 0.0;
    for i in 0..w {
        let sa = a[i].to_ascii_lowercase();
        let sb = b[i].to_ascii_lowercase();
        score += jaro_winkler(&sa, &sb);
    }
    score / (w as f64)
}

/// Merge tables with very similar headers (high threshold), preserving row order.
fn group_tables_by_header_similarity(
    mut items: Vec<TableWithRect>,
    cfg: &ExtractConfig,
) -> Vec<TableWithRect> {
    if items.len() <= 1 { return items; }
    // Sort by top-most position to keep stable concatenation order
    items.sort_by_key(|t| (t.rect.r0, t.rect.c0));

    let mut used = vec![false; items.len()];
    let mut out: Vec<TableWithRect> = Vec::new();

    for i in 0..items.len() {
        if used[i] { continue; }
        used[i] = true;
        let mut acc = items[i].clone();
        // Collect very similar tables and merge into acc
        for j in (i+1)..items.len() {
            if used[j] { continue; }
            // Quick skip if both headers are empty
            let a_has = acc.table.headers.iter().any(|h| !h.trim().is_empty());
            let b_has = items[j].table.headers.iter().any(|h| !h.trim().is_empty());
            if !a_has && !b_has { continue; }

            // Widths must be close to avoid accidental merges
            let wa = acc.table.headers.len();
            let wb = items[j].table.headers.len();
            let w_min = wa.min(wb) as f64;
            let w_max = wa.max(wb) as f64;
            if w_min == 0.0 { continue; }
            let width_ratio = w_min / w_max; // 1.0 == same width
            if width_ratio < 0.9 { continue; }

            let sim = headers_similarity(&acc.table.headers, &items[j].table.headers);
            if sim >= cfg.header_merge_threshold {
                // Merge and enlarge rect
                acc.table = merge_tables(acc.table, items[j].table.clone());
                acc.rect = Rect {
                    r0: min(acc.rect.r0, items[j].rect.r0),
                    c0: min(acc.rect.c0, items[j].rect.c0),
                    r1: max(acc.rect.r1, items[j].rect.r1),
                    c1: max(acc.rect.c1, items[j].rect.c1),
                };
                used[j] = true;
            }
        }
        out.push(acc);
    }
    out
}

/// ============================ De-dup helpers ============================

/// Drop columns that are identical in data and whose headers only differ by an automatic " (n)" suffix.
fn dedup_identical_columns(t: &mut Table) {
    let w = t.headers.len();
    if w <= 1 { return; }

    let mut keep = vec![true; w];
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();

    for c in 0..w {
        let mut sig = strip_header_dup_suffix(&t.headers[c]).to_ascii_lowercase();
        sig.push('\0');
        for r in 0..t.rows.len() {
            if let Some(val) = t.rows[r].get(c) {
                sig.push_str(val);
            }
            sig.push('\x1f');
        }
        if seen.contains_key(&sig) {
            keep[c] = false;
        } else {
            seen.insert(sig, c);
        }
    }

    if keep.iter().all(|&k| k) { return; }

    let mut new_headers = Vec::with_capacity(keep.iter().filter(|k| **k).count());
    for (c, h) in t.headers.iter().enumerate() {
        if keep[c] { new_headers.push(h.clone()); }
    }
    let mut new_rows = Vec::with_capacity(t.rows.len());
    for row in &t.rows {
        let mut nr = Vec::with_capacity(new_headers.len());
        for (c, v) in row.iter().enumerate() {
            if keep[c] { nr.push(v.clone()); }
        }
        new_rows.push(nr);
    }
    t.headers = new_headers;
    t.rows = new_rows;
}

fn strip_header_dup_suffix(h: &str) -> String {
    let s = h.trim();
    if let Some(open) = s.rfind(" (") {
        if s.ends_with(')') {
            let num = &s[(open + 2)..(s.len() - 1)];
            if !num.is_empty() && num.chars().all(|ch| ch.is_ascii_digit()) {
                return s[..open].to_string();
            }
        }
    }
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_out_of_bounds_are_clamped_in_map() {
        let height = 3usize;
        let width = 3usize;
        let merges = vec![Merge { r0: 1, c0: 1, r1: 10, c1: 10 }];
        let mm = build_merge_map(height, width, &merges);
        assert_eq!(mm.len(), height);
        assert_eq!(mm[0].len(), width);
        // Check that bottom-right cell is marked as merged
        assert!(mm[2][2].is_some());
        // And a cell outside the intended area remains None
        assert!(mm[0][0].is_none());
    }

    #[test]
    fn apply_merges_clamps_and_propagates_value() {
    let grid = vec![
            vec!["a".to_string(), "".to_string(), "".to_string()],
            vec!["".to_string(), "b".to_string(), "".to_string()],
            vec!["".to_string(), "".to_string(), "".to_string()],
        ];
        let merges = vec![Merge { r0: 1, c0: 1, r1: 10, c1: 10 }];
        let out = apply_merges(grid.clone(), &merges);
        // Value at (1,1) should be propagated to bottom-right within bounds
        assert_eq!(out[2][2], "b");
        // Other cells should remain unchanged outside the merged block
        assert_eq!(out[0][0], "a");
        assert_eq!(out[0][1], "");
    }
}