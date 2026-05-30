//! 程序入口：解析命令行、加载题目、启动渲染窗口。

mod cli;
mod math;
mod parser;
mod raster;
mod render;

use std::process;

fn main() {
    let args = cli::parse_args(std::env::args().collect());
    if args.help {
        cli::print_help();
        return;
    }

    let questions = match parser::load_file(&args.file_path) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("读取文件失败: {e}");
            process::exit(1);
        }
    };

    if questions.is_empty() {
        eprintln!("文件中没有题目");
        process::exit(1);
    }

    let question_id = args
        .question_id
        .or_else(|| args.positional_id.clone());

    let question = match parser::select_question(&questions, question_id.as_deref()) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("{e}");
            process::exit(1);
        }
    };

    if let Err(e) = render::run(question) {
        eprintln!("渲染错误: {e}");
        process::exit(1);
    }
}
