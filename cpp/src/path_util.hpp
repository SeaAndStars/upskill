#pragma once

#include <string>

/// 解析输入文件路径：当前目录、可执行文件目录、仓库根目录等。
std::string resolve_input_path(const std::string& path, const char* argv0);
