cmake_minimum_required(VERSION 3.20)
project(upskill_cpp LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# 解析 Vulkan SDK 根目录：环境变量 > -DVULKAN_SDK= > 常见安装路径
set(_vulkan_sdk "")
if(DEFINED ENV{VULKAN_SDK} AND EXISTS "$ENV{VULKAN_SDK}")
    set(_vulkan_sdk "$ENV{VULKAN_SDK}")
elseif(DEFINED VULKAN_SDK AND EXISTS "${VULKAN_SDK}")
    set(_vulkan_sdk "${VULKAN_SDK}")
elseif(WIN32)
    foreach(_cand IN ITEMS "D:/vulkan" "C:/VulkanSDK")
        if(EXISTS "${_cand}/Include/vulkan/vulkan.h")
            set(_vulkan_sdk "${_cand}")
            break()
        endif()
        file(GLOB _sdk_versions RELATIVE "${_cand}" "${_cand}/*")
        list(SORT _sdk_versions ORDER DESCENDING)
        foreach(_ver IN LISTS _sdk_versions)
            if(EXISTS "${_cand}/${_ver}/Include/vulkan/vulkan.h")
                set(_vulkan_sdk "${_cand}/${_ver}")
                break()
            endif()
        endforeach()
        if(_vulkan_sdk)
            break()
        endif()
    endforeach()
elseif(APPLE AND EXISTS "/usr/local/share/vulkan")
    set(_vulkan_sdk "/usr/local/share/vulkan")
endif()

if(_vulkan_sdk)
    message(STATUS "Vulkan SDK: ${_vulkan_sdk}")
    list(APPEND CMAKE_PREFIX_PATH "${_vulkan_sdk}")
    if(WIN32)
        set(Vulkan_INCLUDE_DIR "${_vulkan_sdk}/Include" CACHE PATH "" FORCE)
        set(Vulkan_LIBRARY "${_vulkan_sdk}/Lib/vulkan-1.lib" CACHE FILEPATH "" FORCE)
    elseif(APPLE)
        set(Vulkan_INCLUDE_DIR "${_vulkan_sdk}/include" CACHE PATH "" FORCE)
        set(Vulkan_LIBRARY "${_vulkan_sdk}/lib/libvulkan.dylib" CACHE FILEPATH "" FORCE)
    else()
        set(Vulkan_INCLUDE_DIR "${_vulkan_sdk}/include" CACHE PATH "" FORCE)
        set(Vulkan_LIBRARY "${_vulkan_sdk}/lib/libvulkan.so" CACHE FILEPATH "" FORCE)
    endif()
endif()

find_package(Vulkan REQUIRED)
if(NOT Vulkan_FOUND)
    message(FATAL_ERROR
        "未找到 Vulkan SDK。请安装 https://vulkan.lunarg.com/ 并设置环境变量 VULKAN_SDK，"
        "或运行: cmake -B build -DVULKAN_SDK=\"D:/vulkan\"")
endif()

find_program(GLSLC glslc
    HINTS
        "${_vulkan_sdk}/Bin"
        "${_vulkan_sdk}/bin"
        "$ENV{VULKAN_SDK}/Bin"
        "$ENV{VULKAN_SDK}/bin"
)
if(NOT GLSLC)
    message(FATAL_ERROR "未找到 glslc，请安装 Vulkan SDK 或将 Bin 加入 PATH")
endif()

get_filename_component(SHADER_DIR "${CMAKE_CURRENT_SOURCE_DIR}/../shaders" ABSOLUTE)
if(NOT EXISTS "${SHADER_DIR}/color.vert")
    message(FATAL_ERROR "缺少 GLSL 着色器: ${SHADER_DIR}/color.vert（与 .wgsl 并存，供 upskill_cpp 使用）")
endif()
set(SPV_DIR "${CMAKE_BINARY_DIR}/spv")
file(MAKE_DIRECTORY ${SPV_DIR})

set(SHADER_SOURCES
    "${SHADER_DIR}/color.vert"
    "${SHADER_DIR}/line.frag"
    "${SHADER_DIR}/point.frag"
)
set(SPV_OUTPUTS "")
foreach(SRC ${SHADER_SOURCES})
    get_filename_component(NAME ${SRC} NAME)
    set(OUT "${SPV_DIR}/${NAME}.spv")
    add_custom_command(
        OUTPUT ${OUT}
        COMMAND ${GLSLC} --target-env=vulkan1.2 ${SRC} -o ${OUT}
        DEPENDS ${SRC}
        COMMENT "glslc ${NAME}"
    )
    list(APPEND SPV_OUTPUTS ${OUT})
endforeach()
add_custom_target(shaders_spv DEPENDS ${SPV_OUTPUTS})

include(FetchContent)
set(GLFW_BUILD_DOCS OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_TESTS OFF CACHE BOOL "" FORCE)
set(GLFW_BUILD_EXAMPLES OFF CACHE BOOL "" FORCE)
FetchContent_Declare(
    glfw
    GIT_REPOSITORY https://github.com/glfw/glfw.git
    GIT_TAG 3.4
)
FetchContent_MakeAvailable(glfw)

FetchContent_Declare(
    VulkanMemoryAllocator
    GIT_REPOSITORY https://github.com/GPUOpen-LibrariesAndSDKs/VulkanMemoryAllocator.git
    GIT_TAG v3.0.1
)
FetchContent_MakeAvailable(VulkanMemoryAllocator)

set(SOURCES
    src/main.cpp
    src/cli.cpp
    src/path_util.cpp
    src/parser.cpp
    src/math.cpp
    src/frame_geom.cpp
    src/vulkan/instance.cpp
    src/vulkan/device.cpp
    src/vulkan/swapchain.cpp
    src/vulkan/pipeline.cpp
    src/vulkan/app.cpp
)

add_executable(upskill_cpp ${SOURCES})
add_dependencies(upskill_cpp shaders_spv)

target_include_directories(upskill_cpp PRIVATE
    ${CMAKE_CURRENT_SOURCE_DIR}/src
    ${VulkanMemoryAllocator_SOURCE_DIR}/include
)

target_compile_definitions(upskill_cpp PRIVATE
    SPV_DIR="${SPV_DIR}"
    GLFW_INCLUDE_VULKAN
)

if(MSVC)
    target_compile_options(upskill_cpp PRIVATE /W4 /utf-8)
else()
    target_compile_options(upskill_cpp PRIVATE -Wall -Wextra)
endif()

target_link_libraries(upskill_cpp PRIVATE
    Vulkan::Vulkan
    glfw
)

if(WIN32)
    target_compile_definitions(upskill_cpp PRIVATE NOMINMAX VK_USE_PLATFORM_WIN32_KHR)
endif()

set(TESTPOINT_IN "${CMAKE_CURRENT_SOURCE_DIR}/../testpoint.in")
configure_file("${TESTPOINT_IN}" "${CMAKE_BINARY_DIR}/testpoint.in" COPYONLY)

add_custom_command(
    TARGET upskill_cpp POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${TESTPOINT_IN}"
        "$<TARGET_FILE_DIR:upskill_cpp>/testpoint.in"
    COMMENT "copy testpoint.in next to upskill_cpp"
)
