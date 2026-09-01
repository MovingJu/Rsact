cmake_minimum_required(VERSION 3.23)
project(rsact-cbin)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

include(FetchContent)
FetchContent_Declare(
    Rsact
    GIT_REPOSITORY https://github.com/MovingJu/Rsact.git
    GIT_TAG c4f87cc8fce2e146b5aa3c17c5447d866264cfe7
)
FetchContent_MakeAvailable(Rsact)

add_executable(rsact-cbin src/input.c)
target_link_libraries(rsact-cbin PRIVATE rsact::rsact_ffi)