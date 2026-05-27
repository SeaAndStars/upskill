//! 手写命令行参数解析，兼容题面 `-file`/`-id`/`-help` 与标准形式。

/// 解析后的命令行参数。
#[derive(Debug, Clone)]
pub struct CliArgs {
    /// 是否显示帮助。
    pub help: bool,
    /// 输入文件路径，默认 `testpoint.in`。
    pub file_path: String,
    /// `-id` / `--id` 指定的题目 id（优先于位置参数）。
    pub question_id: Option<String>,
    /// 位置参数中的题目 id（仅当未指定 `-id` 时生效）。
    pub positional_id: Option<String>,
}

/// 判断 token 是否为需要取下一参数的 flag。
fn flag_takes_value(arg: &str) -> bool {
    matches!(
        arg,
        "-file" | "--file" | "-f" | "-id" | "--id"
    )
}

/// 判断 token 是否为帮助 flag。
fn is_help(arg: &str) -> bool {
    matches!(arg, "-help" | "--help" | "-h")
}

/// 从 `argv`（含程序名）解析命令行。
pub fn parse_args(argv: Vec<String>) -> CliArgs {
    let mut help = false;
    let mut file_path = "testpoint.in".to_string();
    let mut question_id: Option<String> = None;
    let mut positional_id: Option<String> = None;

    let mut i = 1;
    while i < argv.len() {
        let arg = argv[i].as_str();
        if is_help(arg) {
            help = true;
            i += 1;
            continue;
        }
        if flag_takes_value(arg) {
            if i + 1 >= argv.len() {
                i += 1;
                continue;
            }
            let value = argv[i + 1].clone();
            match arg {
                "-file" | "--file" | "-f" => file_path = value,
                "-id" | "--id" => question_id = Some(value),
                _ => {}
            }
            i += 2;
            continue;
        }
        if !arg.starts_with('-') && positional_id.is_none() {
            positional_id = Some(arg.to_string());
        }
        i += 1;
    }

    CliArgs {
        help,
        file_path,
        question_id,
        positional_id,
    }
}

/// 打印中文帮助信息。
pub fn print_help() {
    println!(
        r#"用法: upskill [-file <路径>] [<题目id>] [-id <id>] [-help]

选项:
  -file, --file, -f <路径>   输入文件（默认 testpoint.in）
  -id, --id <id>             题目 id（优先于位置参数）
  -help, --help, -h           显示此帮助

规则:
  - 未指定题目 id 时使用文件中最后一题
  - 同时给出位置参数与 -id 时，以 -id 为准

示例:
  upskill
  upskill -file testpoint.in 1
  upskill 1 -id 2          # 加载 id=2
"#
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_flag_overrides_positional() {
        let args = parse_args(vec![
            "upskill".into(),
            "1".into(),
            "-id".into(),
            "2".into(),
        ]);
        assert_eq!(args.question_id.as_deref(), Some("2"));
        assert_eq!(args.positional_id.as_deref(), Some("1"));
    }

    #[test]
    fn default_file() {
        let args = parse_args(vec!["upskill".into()]);
        assert_eq!(args.file_path, "testpoint.in");
    }
}
