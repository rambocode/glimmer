# 微明 Fcitx5 插件：C++ 薄壳 glimmer.so，静态链接 Rust 半边 libglimmer_fcitx5.a（cargo build --release -p glimmer-fcitx5）。
cmake_minimum_required(VERSION 3.16)
project(glimmer-fcitx5 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

find_package(Fcitx5Core REQUIRED)
find_package(Fcitx5Utils REQUIRED)
find_package(Fcitx5Config REQUIRED)

set(GLIMMER_RUST_LIB "" CACHE FILEPATH "Rust 静态库 libglimmer_fcitx5.a 的路径（必填）")
set(GLIMMER_DATA_ROOT "/usr/lib/glimmer" CACHE PATH "随包资源根（词库、模型）；运行时环境变量 GLIMMER_DATA_ROOT 可覆盖")

if(NOT GLIMMER_RUST_LIB OR NOT EXISTS "${GLIMMER_RUST_LIB}")
    message(FATAL_ERROR "GLIMMER_RUST_LIB 未设置或文件不存在：${GLIMMER_RUST_LIB}")
endif()

add_library(glimmer MODULE
    src/action.cpp
    src/candidate.cpp
    src/engine.cpp
    src/state.cpp
)
# fcitx5 按 Library=glimmer 找 glimmer.so，不要 lib 前缀；C++ 符号缺省不导出，只留 FCITX_ADDON_FACTORY 标了导出的入口
set_target_properties(glimmer PROPERTIES PREFIX "" CXX_VISIBILITY_PRESET hidden VISIBILITY_INLINES_HIDDEN ON)
target_include_directories(glimmer PRIVATE "${CMAKE_CURRENT_SOURCE_DIR}/../include")
target_compile_definitions(glimmer PRIVATE GLIMMER_DATA_ROOT="${GLIMMER_DATA_ROOT}")
# 候选的 comment 字段 fcitx5 5.1.9 才有；更老的版本把译词拼在候选文字后面
if(Fcitx5Core_VERSION VERSION_GREATER_EQUAL 5.1.9)
    target_compile_definitions(glimmer PRIVATE GLIMMER_HAS_CANDIDATE_COMMENT)
endif()
target_link_libraries(glimmer PRIVATE
    Fcitx5::Core Fcitx5::Utils Fcitx5::Config
    "${GLIMMER_RUST_LIB}"
    # Rust 标准库与依赖要的系统库（cargo rustc -- --print native-static-libs）；native-tls 用系统 OpenSSL
    ssl crypto pthread dl m rt util gcc_s
)
# 静态库里的 Rust 符号不导出，免得与 fcitx5 进程里别的库撞名；有漏链的库在链接期就报出来
target_link_options(glimmer PRIVATE -Wl,--exclude-libs,ALL -Wl,--gc-sections -Wl,--no-undefined)

configure_file(glimmer-addon.conf.in "${CMAKE_CURRENT_BINARY_DIR}/glimmer-addon.conf" @ONLY)

install(TARGETS glimmer DESTINATION "${FCITX_INSTALL_ADDONDIR}")
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/glimmer-addon.conf" RENAME glimmer.conf
    DESTINATION "${FCITX_INSTALL_PKGDATADIR}/addon")
install(FILES glimmer.conf DESTINATION "${FCITX_INSTALL_PKGDATADIR}/inputmethod")
