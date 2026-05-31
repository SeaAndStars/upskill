#include "path_util.hpp"

#include <fstream>
#include <vector>

#ifdef _WIN32
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <Windows.h>
#endif

namespace {

bool file_readable(const std::string& path) {
    std::ifstream f(path, std::ios::binary);
    return f.good();
}

std::string parent_dir(const std::string& dir) {
    auto pos = dir.find_last_of("/\\");
    if (pos == std::string::npos) {
        return {};
    }
    return dir.substr(0, pos);
}

std::string exe_directory(const char* argv0) {
#ifdef _WIN32
    char buf[MAX_PATH]{};
    DWORD n = GetModuleFileNameA(nullptr, buf, MAX_PATH);
    if (n == 0 || n >= MAX_PATH) {
        return {};
    }
    std::string full(buf, n);
    return parent_dir(full);
#else
    if (!argv0 || !*argv0) {
        return {};
    }
    return parent_dir(argv0);
#endif
}

}  // namespace

std::string resolve_input_path(const std::string& path, const char* argv0) {
    if (file_readable(path)) {
        return path;
    }

    std::vector<std::string> candidates;
    candidates.push_back(path);

    const std::string exe_dir = exe_directory(argv0);
    if (!exe_dir.empty()) {
        candidates.push_back(exe_dir + "/" + path);
        candidates.push_back(exe_dir + "/../" + path);
        candidates.push_back(exe_dir + "/../../" + path);
    }

    if (path == "testpoint.in") {
        candidates.push_back("../testpoint.in");
        candidates.push_back("../../testpoint.in");
    }

    for (const auto& c : candidates) {
        if (file_readable(c)) {
            return c;
        }
    }
    return path;
}
