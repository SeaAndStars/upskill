#include "cli.hpp"

#include <iostream>

namespace {

bool flag_takes_value(const std::string& arg) {
    return arg == "-file" || arg == "--file" || arg == "-f" || arg == "-id" ||
           arg == "--id";
}

bool is_help(const std::string& arg) {
    return arg == "-help" || arg == "--help" || arg == "-h";
}

}  // namespace

CliArgs parse_args(const std::vector<std::string>& argv) {
    CliArgs out;
    std::size_t i = 1;
    while (i < argv.size()) {
        const std::string& arg = argv[i];
        if (is_help(arg)) {
            out.help = true;
            ++i;
            continue;
        }
        if (flag_takes_value(arg)) {
            if (i + 1 >= argv.size()) {
                ++i;
                continue;
            }
            const std::string& value = argv[i + 1];
            if (arg == "-file" || arg == "--file" || arg == "-f") {
                out.file_path = value;
            } else if (arg == "-id" || arg == "--id") {
                out.question_id = value;
            }
            i += 2;
            continue;
        }
        if (!arg.empty() && arg[0] != '-' && !out.positional_id) {
            out.positional_id = arg;
        }
        ++i;
    }
    return out;
}

void print_help() {
    std::cout << R"(用法: upskill_cpp [-file <路径>] [<题目id>] [-id <id>] [-help]

选项:
  -file, --file, -f <路径>   输入文件（默认 testpoint.in）
  -id, --id <id>             题目 id（优先于位置参数）
  -help, --help, -h           显示此帮助

规则:
  - 未指定题目 id 时使用文件中最后一题
  - 同时给出位置参数与 -id 时，以 -id 为准

示例:
  upskill_cpp
  upskill_cpp -file testpoint.in 1
  upskill_cpp 1 -id 2
)";
}
