cmake_minimum_required(VERSION 3.20)
project(upskill_cpp LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

if(DEFINED ENV{VULKAN_SDK})
    list(APPEND CMAKE_PREFIX_PATH "$ENV{VULKAN_SDK}")
    if(WIN32)
        set(Vulkan_INCLUDE_DIR "$ENV{VULKAN_SDK}/Include" CACHE PATH "")
        set(Vulkan_LIBRARY "$ENV{VULKAN_SDK}/Lib/vulkan-1.lib" CACHE FILEPATH "")
    elseif(APPLE)
        set(Vulkan_INCLUDE_DIR "$ENV{VULKAN_SDK}/include" CACHE PATH "")
        set(Vulkan_LIBRARY "$ENV{VULKAN_SDK}/lib/libvulkan.dylib" CACHE FILEPATH "")
    else()
        set(Vulkan_INCLUDE_DIR "$ENV{VULKAN_SDK}/include" CACHE PATH "")
        set(Vulkan_LIBRARY "$ENV{VULKAN_SDK}/lib/libvulkan.so" CACHE FILEPATH "")
    endif()
endif()

find_package(Vulkan REQUIRED)

find_program(GLSLC glslc
    HINTS
        "$ENV{VULKAN_SDK}/Bin"
        "$ENV{VULKAN_SDK}/bin"
        "$ENV{VULKAN_SDK}/Bin32"
)
if(NOT GLSLC)
    message(FATAL_ERROR "未找到 glslc，请安装 Vulkan SDK")
endif()

set(SHADER_DIR "${CMAKE_SOURCE_DIR}/../shaders")
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

configure_file(
    "${CMAKE_SOURCE_DIR}/../testpoint.in"
    "${CMAKE_BINARY_DIR}/testpoint.in"
    COPYONLY
)
