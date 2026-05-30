#include <iostream>
#include <string>
#include <vector>

#include "cli.hpp"
#include "parser.hpp"
#include "vulkan/app.hpp"

int main(int argc, char** argv) {
    std::vector<std::string> args;
    args.reserve(static_cast<std::size_t>(argc));
    for (int i = 0; i < argc; ++i) {
        args.emplace_back(argv[i]);
    }

    CliArgs cli = parse_args(args);
    if (cli.help) {
        print_help();
        return 0;
    }

    try {
        auto questions = load_file(cli.file_path);
        std::string id_str;
        const std::string* id_ptr = nullptr;
        if (cli.question_id) {
            id_str = *cli.question_id;
            id_ptr = &id_str;
        } else if (cli.positional_id) {
            id_str = *cli.positional_id;
            id_ptr = &id_str;
        }
        const Question* q = select_question(questions, id_ptr);
        if (!q) {
            std::cerr << "未找到题目\n";
            return 1;
        }
        run_vulkan_app(*q);
    } catch (const std::exception& e) {
        std::cerr << e.what() << '\n';
        return 1;
    }
    return 0;
}
