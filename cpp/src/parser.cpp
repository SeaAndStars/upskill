#include "parser.hpp"

#include <algorithm>
#include <cctype>
#include <cstdio>
#include <fstream>
#include <optional>
#include <sstream>
#include <stdexcept>

namespace {

std::string read_file_all(const std::string& path) {
#ifdef _MSC_VER
    FILE* f = nullptr;
    if (fopen_s(&f, path.c_str(), "rb") != 0 || !f) {
        throw std::runtime_error("无法打开 " + path);
    }
#else
    FILE* f = fopen(path.c_str(), "rb");
    if (!f) {
        throw std::runtime_error("无法打开 " + path);
    }
#endif
    fseek(f, 0, SEEK_END);
    long sz = ftell(f);
    fseek(f, 0, SEEK_SET);
    std::string data(static_cast<std::size_t>(sz > 0 ? sz : 0), '\0');
    if (sz > 0) {
        fread(data.data(), 1, static_cast<std::size_t>(sz), f);
    }
    fclose(f);
    return data;
}

std::string to_lower(std::string s) {
    std::transform(s.begin(), s.end(), s.begin(),
                   [](unsigned char c) { return static_cast<char>(std::tolower(c)); });
    return s;
}

std::string parse_question_id(const std::string& tag_part) {
    std::string lower = to_lower(tag_part);
    auto id_pos = lower.find("id");
    if (id_pos == std::string::npos) {
        throw std::runtime_error("question 标签缺少 id");
    }
    std::string rest = tag_part.substr(id_pos + 2);
    while (!rest.empty() && (rest[0] == '=' || std::isspace(static_cast<unsigned char>(rest[0])))) {
        rest.erase(rest.begin());
    }
    if (!rest.empty() && rest[0] == '"') {
        auto end = rest.find('"', 1);
        if (end == std::string::npos) {
            throw std::runtime_error("id 引号未闭合");
        }
        return rest.substr(1, end - 1);
    }
    if (!rest.empty() && rest[0] == '\'') {
        auto end = rest.find('\'', 1);
        if (end == std::string::npos) {
            throw std::runtime_error("id 引号未闭合");
        }
        return rest.substr(1, end - 1);
    }
    std::string id;
    for (char c : rest) {
        if (std::isspace(static_cast<unsigned char>(c)) || c == '>') {
            break;
        }
        id += c;
    }
    if (id.empty()) {
        throw std::runtime_error("question id 为空");
    }
    return id;
}

std::optional<std::size_t> find_next_field_start(const std::string& s) {
    const char* markers[] = {"points:", "connets:", "connects:", "width:", "height:"};
    std::string lower = to_lower(s);
    std::optional<std::size_t> best;
    for (const char* m : markers) {
        auto pos = lower.find(m);
        if (pos != std::string::npos && pos > 0) {
            if (!best || pos < *best) {
                best = pos;
            }
        }
    }
    return best;
}

std::string extract_section(const std::string& body, const std::string& key) {
    std::string pattern = key + ":";
    std::string lower_body = to_lower(body);
    std::string pattern_lower = to_lower(pattern);
    auto start = lower_body.find(pattern_lower);
    if (start == std::string::npos) {
        throw std::runtime_error("缺少 " + key + ":");
    }
    std::size_t value_start = start + pattern.size();
    std::string tail = body.substr(value_start);
    auto end_opt = find_next_field_start(tail);
    std::size_t end = end_opt.value_or(tail.size());
    std::string val = tail.substr(0, end);
    while (!val.empty() && std::isspace(static_cast<unsigned char>(val.back()))) {
        val.pop_back();
    }
    while (!val.empty() && std::isspace(static_cast<unsigned char>(val.front()))) {
        val.erase(val.begin());
    }
    return val;
}

std::string extract_connets_section(const std::string& body) {
    std::string lower = to_lower(body);
    std::size_t start = std::string::npos;
    std::string key_used;
    for (const char* key : {"connets", "connects"}) {
        std::string pattern = std::string(key) + ":";
        auto pos = lower.find(pattern);
        if (pos != std::string::npos && (start == std::string::npos || pos < start)) {
            start = pos;
            key_used = key;
        }
    }
    if (start == std::string::npos) {
        throw std::runtime_error("缺少 connets:/connects:");
    }
    std::size_t value_start = start + key_used.size() + 1;
    std::string tail = body.substr(value_start);
    auto end_opt = find_next_field_start(tail);
    std::size_t end = end_opt.value_or(tail.size());
    std::string val = tail.substr(0, end);
    while (!val.empty() && std::isspace(static_cast<unsigned char>(val.back()))) {
        val.pop_back();
    }
    while (!val.empty() && std::isspace(static_cast<unsigned char>(val.front()))) {
        val.erase(val.begin());
    }
    return val;
}

bool read_number(const std::string& s, std::size_t& i, double& out) {
    while (i < s.size() && std::isspace(static_cast<unsigned char>(s[i]))) {
        ++i;
    }
    std::size_t start = i;
    if (i < s.size() && (s[i] == '+' || s[i] == '-')) {
        ++i;
    }
    bool saw = false;
    while (i < s.size() && (std::isdigit(static_cast<unsigned char>(s[i])) || s[i] == '.')) {
        saw = true;
        ++i;
    }
    if (!saw) {
        return false;
    }
    out = std::stod(s.substr(start, i - start));
    return true;
}

bool parse_point_brace(const std::string& section, std::size_t& i, Point3& p) {
    if (i >= section.size() || section[i] != '{') {
        return false;
    }
    if (i + 1 < section.size() && section[i + 1] == '{') {
        return false;
    }
    ++i;
    bool has_x = false, has_y = false, has_z = false;
    while (i < section.size()) {
        if (section[i] == '}') {
            ++i;
            if (!has_x || !has_y || !has_z) {
                return false;
            }
            return true;
        }
        while (i < section.size() &&
               (std::isspace(static_cast<unsigned char>(section[i])) || section[i] == ',')) {
            ++i;
        }
        std::size_t name_start = i;
        while (i < section.size() &&
               (std::isalnum(static_cast<unsigned char>(section[i])) || section[i] == '_')) {
            ++i;
        }
        if (i == name_start) {
            if (section[i] == '}') {
                continue;
            }
            ++i;
            continue;
        }
        std::string name = section.substr(name_start, i - name_start);
        while (i < section.size() &&
               (std::isspace(static_cast<unsigned char>(section[i])) || section[i] == ':')) {
            ++i;
        }
        double val = 0;
        if (!read_number(section, i, val)) {
            continue;
        }
        if (name == "x") {
            p.x = val;
            has_x = true;
        } else if (name == "y") {
            p.y = val;
            has_y = true;
        } else if (name == "z") {
            p.z = val;
            has_z = true;
        }
    }
    return false;
}

std::vector<Point3> parse_points(const std::string& section) {
    std::vector<Point3> points;
    std::size_t i = 0;
    while (i < section.size()) {
        Point3 p;
        if (parse_point_brace(section, i, p)) {
            points.push_back(p);
        } else {
            ++i;
        }
    }
    if (points.empty()) {
        throw std::runtime_error("points 为空");
    }
    return points;
}

std::vector<std::size_t> parse_index_list(const std::string& s) {
    std::vector<std::size_t> out;
    std::size_t i = 0;
    while (i < s.size()) {
        while (i < s.size() && (std::isspace(static_cast<unsigned char>(s[i])) || s[i] == ',')) {
            ++i;
        }
        if (i >= s.size()) {
            break;
        }
        std::size_t start = i;
        while (i < s.size() && std::isdigit(static_cast<unsigned char>(s[i]))) {
            ++i;
        }
        if (i == start) {
            ++i;
            continue;
        }
        out.push_back(static_cast<std::size_t>(std::stoull(s.substr(start, i - start))));
    }
    return out;
}

std::vector<std::vector<std::size_t>> parse_connets(const std::string& section) {
    std::vector<std::vector<std::size_t>> groups;
    std::size_t i = 0;
    while (i < section.size()) {
        if (section[i] != '{') {
            ++i;
            continue;
        }
        std::size_t group_start = i;
        int depth = 0;
        std::size_t j = i;
        for (; j < section.size(); ++j) {
            if (section[j] == '{') {
                ++depth;
            } else if (section[j] == '}') {
                --depth;
                if (depth == 0) {
                    std::string inner = section.substr(group_start + 1, j - group_start - 1);
                    if (inner.find('{') != std::string::npos) {
                        auto nested = parse_connets(inner);
                        groups.insert(groups.end(), nested.begin(), nested.end());
                    } else {
                        bool has_digit = false;
                        for (char c : inner) {
                            if (std::isdigit(static_cast<unsigned char>(c))) {
                                has_digit = true;
                                break;
                            }
                        }
                        if (has_digit) {
                            auto indices = parse_index_list(inner);
                            if (!indices.empty()) {
                                groups.push_back(std::move(indices));
                            }
                        }
                    }
                    i = j + 1;
                    break;
                }
            }
        }
        if (j >= section.size()) {
            break;
        }
    }
    return groups;
}

std::size_t parse_usize_field(const std::string& body, const std::string& key) {
    std::string section = extract_section(body, key);
    std::string digits;
    for (char c : section) {
        if (std::isdigit(static_cast<unsigned char>(c))) {
            digits += c;
        }
    }
    if (digits.empty()) {
        throw std::runtime_error("无效的 " + key + ": " + section);
    }
    return static_cast<std::size_t>(std::stoull(digits));
}

Question parse_question_body(const std::string& id, const std::string& body) {
    Question q;
    q.id = id;
    q.points = parse_points(extract_section(body, "points"));
    q.connets = parse_connets(extract_connets_section(body));
    q.width = parse_usize_field(body, "width");
    q.height = parse_usize_field(body, "height");
    return q;
}

}  // namespace

std::vector<Question> load_file(const std::string& path) {
    return parse_all(read_file_all(path));
}

std::vector<Question> parse_all(const std::string& content) {
    std::vector<Question> questions;
    std::size_t search_from = 0;
    while (true) {
        auto rel = content.find("<question", search_from);
        if (rel == std::string::npos) {
            break;
        }
        std::size_t start = rel;
        std::string after_tag = content.substr(start);
        std::string id = parse_question_id(after_tag);
        auto gt = after_tag.find('>');
        if (gt == std::string::npos) {
            throw std::runtime_error("缺少 question 开始标签的 >");
        }
        std::size_t body_start = start + gt + 1;
        auto close_rel = content.find("</question>", body_start);
        if (close_rel == std::string::npos) {
            throw std::runtime_error("缺少 </question>");
        }
        std::string body = content.substr(body_start, close_rel - body_start);
        questions.push_back(parse_question_body(id, body));
        search_from = close_rel + 11;
        if (search_from >= content.size()) {
            break;
        }
    }
    if (questions.empty()) {
        throw std::runtime_error("未找到任何 <question> 块");
    }
    return questions;
}

const Question* select_question(const std::vector<Question>& questions,
                                const std::string* id) {
    if (questions.empty()) {
        return nullptr;
    }
    if (!id) {
        return &questions.back();
    }
    for (const auto& q : questions) {
        if (q.id == *id) {
            return &q;
        }
    }
    return nullptr;
}
