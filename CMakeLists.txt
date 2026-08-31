cmake_minimum_required(VERSION 3.23)
project(rsact-clib)

include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/corrosion-rs/corrosion.git
    GIT_TAG v0.6 # Optionally specify a commit hash, version tag or branch here
)
FetchContent_MakeAvailable(Corrosion)

corrosion_import_crate(
    MANIFEST_PATH ${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml 
    CRATES rsact-ffi
)

add_library(rsact::rsact_ffi ALIAS rsact_ffi)
target_include_directories(
    rsact_ffi INTERFACE ${CMAKE_CURRENT_SOURCE_DIR}/crates/rsact-ffi/include
)