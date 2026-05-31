#pragma once

#include <cstddef>
#include <string>
#include <vector>

/// 三维点坐标。
struct Point3 {
    double x = 0;  ///< x 分量，定义域 [-1, 1]。
    double y = 0;  ///< y 分量，定义域 [-1, 1]。
    double z = 0;  ///< z 分量，无界。
};

/// 一题完整数据。
struct Question {
    std::string id;                    ///< 题目 id（可为非数字）。
    std::vector<Point3> points;        ///< 顶点列表。
    std::vector<std::vector<std::size_t>> connets;  ///< 连线索引序列。
    std::size_t width = 500;           ///< 窗口宽度（像素）。
    std::size_t height = 500;          ///< 窗口高度（像素）。
};

/// 从文件路径加载全部题目。
std::vector<Question> load_file(const std::string& path);

/// 解析全文中的所有题目块。
std::vector<Question> parse_all(const std::string& content);

/// 按 id 选题；id 为空时返回最后一题。
const Question* select_question(const std::vector<Question>& questions,
                                const std::string* id);
