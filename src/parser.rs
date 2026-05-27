//! 解析 `<question id=...>...</question>` 格式的输入文件。

use std::fs;
use std::path::Path;

/// 三维点坐标。
#[derive(Debug, Clone, Copy)]
pub struct Point3 {
    /// x 分量，定义域 [-1, 1]。
    pub x: f64,
    /// y 分量，定义域 [-1, 1]。
    pub y: f64,
    /// z 分量，无界。
    pub z: f64,
}

/// 一题完整数据。
#[derive(Debug, Clone)]
pub struct Question {
    /// 题目 id（可为非数字字符串）。
    pub id: String,
    /// 顶点列表。
    pub points: Vec<Point3>,
    /// 连线索引序列（题面拼写 connets）。
    pub connets: Vec<Vec<usize>>,
    /// 窗口宽度（像素）。
    pub width: usize,
    /// 窗口高度（像素）。
    pub height: usize,
}

/// 从文件路径加载全部题目。
pub fn load_file(path: &str) -> Result<Vec<Question>, String> {
    let content = fs::read_to_string(Path::new(path))
        .map_err(|e| format!("无法打开 {path}: {e}"))?;
    parse_all(&content)
}

/// 解析全文中的所有题目块。
pub fn parse_all(content: &str) -> Result<Vec<Question>, String> {
    let mut questions = Vec::new();
    let mut search_from = 0;
    let bytes = content.as_bytes();

    while let Some(start_rel) = content[search_from..].find("<question") {
        let start = search_from + start_rel;
        let after_tag = &content[start..];
        let id = parse_question_id(after_tag)?;
        let body_start = after_tag
            .find('>')
            .ok_or("缺少 question 开始标签的 >")?
            + start
            + 1;
        let close_rel = content[body_start..]
            .find("</question>")
            .ok_or("缺少 </question>")?;
        let body = &content[body_start..body_start + close_rel];
        let q = parse_question_body(&id, body)?;
        questions.push(q);
        search_from = body_start + close_rel + "</question>".len();
        if search_from >= bytes.len() {
            break;
        }
    }

    if questions.is_empty() {
        return Err("未找到任何 <question> 块".into());
    }
    Ok(questions)
}

/// 从 `<question id=...` 片段提取 id。
fn parse_question_id(tag_part: &str) -> Result<String, String> {
    let lower = tag_part.to_lowercase();
    let id_pos = lower
        .find("id")
        .ok_or("question 标签缺少 id")?;
    let after_id = &tag_part[id_pos + 2..];
    let rest = after_id.trim_start_matches(|c: char| c == '=' || c.is_whitespace());
    if rest.starts_with('"') {
        let end = rest[1..]
            .find('"')
            .ok_or("id 引号未闭合")?;
        return Ok(rest[1..1 + end].trim().to_string());
    }
    if rest.starts_with('\'') {
        let end = rest[1..]
            .find('\'')
            .ok_or("id 引号未闭合")?;
        return Ok(rest[1..1 + end].trim().to_string());
    }
    let id: String = rest
        .chars()
        .take_while(|c| !c.is_whitespace() && *c != '>')
        .collect();
    if id.is_empty() {
        return Err("question id 为空".into());
    }
    Ok(id)
}

/// 解析题目正文中的 points、connets、width、height。
fn parse_question_body(id: &str, body: &str) -> Result<Question, String> {
    let points = parse_points(extract_section(body, "points")?)?;
    let connets = parse_connets(extract_section(body, "connets")?)?;
    let width = parse_usize_field(body, "width")?;
    let height = parse_usize_field(body, "height")?;
    Ok(Question {
        id: id.to_string(),
        points,
        connets,
        width,
        height,
    })
}

/// 提取 `key:` 之后到下一个已知字段或块尾的内容。
fn extract_section<'a>(body: &'a str, key: &str) -> Result<&'a str, String> {
    let pattern = format!("{key}:");
    let lower_body = body.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    let start = lower_body
        .find(&pattern_lower)
        .ok_or_else(|| format!("缺少 {key}:"))?;
    let value_start = start + pattern.len();
    let tail = &body[value_start..];
    let end = find_next_field_start(tail).unwrap_or(tail.len());
    Ok(tail[..end].trim())
}

/// 查找下一个字段 `points:` / `connets:` / `width:` / `height:` 的起始位置。
fn find_next_field_start(s: &str) -> Option<usize> {
    let markers = ["points:", "connets:", "width:", "height:"];
    let lower = s.to_lowercase();
    markers
        .iter()
        .filter_map(|m| lower.find(m))
        .filter(|&i| i > 0)
        .min()
}

/// 解析 points 区块内的 `{x,y,z}` 列表。
fn parse_points(section: &str) -> Result<Vec<Point3>, String> {
    let mut points = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = section.chars().collect();
    while i < chars.len() {
        if chars[i] == '{' && i + 1 < chars.len() && chars[i + 1] != '{' {
            if let Some((p, next)) = parse_point_brace(&chars, i) {
                points.push(p);
                i = next;
                continue;
            }
        }
        i += 1;
    }
    if points.is_empty() {
        return Err("points 为空".into());
    }
    Ok(points)
}

/// 解析单个 `{ x: ..., y: ..., z: ... }`。
fn parse_point_brace(chars: &[char], start: usize) -> Option<(Point3, usize)> {
    let mut i = start + 1;
    let mut x = None;
    let mut y = None;
    let mut z = None;
    while i < chars.len() {
        if chars[i] == '}' {
            let p = Point3 {
                x: x?,
                y: y?,
                z: z?,
            };
            return Some((p, i + 1));
        }
        if let Some((key, val, next)) = try_read_labeled_number(chars, i) {
            match key.as_str() {
                "x" => x = Some(val),
                "y" => y = Some(val),
                "z" => z = Some(val),
                _ => {}
            }
            i = next;
        } else {
            i += 1;
        }
    }
    None
}

/// 读取 `name: number` 形式。
fn try_read_labeled_number(chars: &[char], start: usize) -> Option<(String, f64, usize)> {
    let mut i = start;
    while i < chars.len() && chars[i].is_whitespace() || chars[i] == ',' {
        i += 1;
    }
    let name_start = i;
    while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
        i += 1;
    }
    if i == name_start {
        return None;
    }
    let name: String = chars[name_start..i].iter().collect();
    while i < chars.len() && (chars[i].is_whitespace() || chars[i] == ':') {
        i += 1;
    }
    let (val, next) = read_number(chars, i)?;
    Some((name, val, next))
}

/// 读取浮点数。
fn read_number(chars: &[char], start: usize) -> Option<(f64, usize)> {
    let mut i = start;
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    let num_start = i;
    if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
        i += 1;
    }
    let mut saw_digit = false;
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
        saw_digit = true;
        i += 1;
    }
    if !saw_digit {
        return None;
    }
    let s: String = chars[num_start..i].iter().collect();
    let val = s.parse().ok()?;
    Some((val, i))
}

/// 解析 connets：`{{0,1,...}, ...}`。
fn parse_connets(section: &str) -> Result<Vec<Vec<usize>>, String> {
    let mut groups = Vec::new();
    let chars: Vec<char> = section.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '{' {
            i += 1;
            continue;
        }
        let group_start = i;
        let mut depth = 0;
        let mut j = i;
        while j < chars.len() {
            if chars[j] == '{' {
                depth += 1;
            } else if chars[j] == '}' {
                depth -= 1;
                if depth == 0 {
                    let inner: String = chars[group_start + 1..j].iter().collect();
                    if inner.contains('{') {
                        groups.extend(parse_connets(&inner)?);
                    } else if inner.chars().any(|c| c.is_ascii_digit()) {
                        let indices = parse_index_list(&inner)?;
                        if !indices.is_empty() {
                            groups.push(indices);
                        }
                    }
                    i = j + 1;
                    break;
                }
            }
            j += 1;
        }
        if j >= chars.len() {
            break;
        }
    }
    Ok(groups)
}

/// 解析逗号分隔的非负整数索引。
fn parse_index_list(s: &str) -> Result<Vec<usize>, String> {
    let mut out = Vec::new();
    for part in s.split(',') {
        let t = part.trim();
        if t.is_empty() {
            continue;
        }
        let n: usize = t
            .parse()
            .map_err(|_| format!("无效索引: {t}"))?;
        out.push(n);
    }
    Ok(out)
}

/// 解析 `width:` / `height:` 整数字段。
fn parse_usize_field(body: &str, key: &str) -> Result<usize, String> {
    let section = extract_section(body, key)?;
    let digits: String = section
        .chars()
        .filter(|c| c.is_ascii_digit())
        .collect();
    digits
        .parse()
        .map_err(|_| format!("无效的 {key}: {section}"))
}

/// 按 id 选题；`id` 为 `None` 时返回最后一题。
pub fn select_question<'a>(
    questions: &'a [Question],
    id: Option<&str>,
) -> Result<&'a Question, String> {
    match id {
        None => questions.last().ok_or("题目列表为空".into()),
        Some(want) => questions
            .iter()
            .find(|q| q.id == want)
            .ok_or_else(|| format!("未找到 id={want} 的题目")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_two_questions() {
        let content = r#"
<question id=1>
points: { { x: 0.25, y: 0.25, z: 0.25 } }
connets: { {0, 1} }
width: 100
height: 200
</question>
<question id=2>
points: { { x: -0.25, y: 0.25, z: 0.25 } }
connets: { {0} }
width: 500
height: 500
</question>
"#;
        let qs = parse_all(content).unwrap();
        assert_eq!(qs.len(), 2);
        assert_eq!(qs[0].width, 100);
        assert_eq!(qs[1].id, "2");
    }
}
