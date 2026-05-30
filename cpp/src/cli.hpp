#pragma once

#include <optional>
#include <string>
#include <vector>

/// 解析后的命令行参数。
struct CliArgs {
    /// 是否显示帮助。
    bool help = false;
    /// 输入文件路径，默认 testpoint.in。
    std::string file_path = "testpoint.in";
    /// -id 指定的题目 id（优先于位置参数）。
    std::optional<std::string> question_id;
    /// 位置参数中的题目 id。
    std::optional<std::string> positional_id;
};

/// 从 argv（含程序名）解析命令行。
CliArgs parse_args(const std::vector<std::string>& argv);

/// 打印中文帮助信息。
void print_help();
