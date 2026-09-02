cmake_minimum_required(VERSION 3.24) # DOWNLOAD_EXTRACT_TIMESTAMP requires 3.24
project(rsact-cbin)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# ---------------------------------------------------------------------------
# Fetch the prebuilt rsact-ffi static lib + header from the GitHub Release
# instead of building the Rust workspace via Corrosion/cargo. Only the
# platforms Rsact's release workflow publishes are supported here.
# ---------------------------------------------------------------------------
set(RSACT_VERSION "v0.1.2")

if(CMAKE_SYSTEM_NAME STREQUAL "Linux" AND CMAKE_SYSTEM_PROCESSOR MATCHES "^(x86_64|amd64)$")
    set(RSACT_TARGET "x86_64-unknown-linux-gnu")
    set(RSACT_ARCHIVE_EXT "tar.gz")
    set(RSACT_ARCHIVE_SHA256 "0d23369f716d4c6ea7f855439789421f69bac128995644466cf7449c49c49b50")
    set(RSACT_LIB_NAME "librsact_ffi.a")
elseif(CMAKE_SYSTEM_NAME STREQUAL "Darwin" AND CMAKE_SYSTEM_PROCESSOR MATCHES "^(arm64|aarch64)$")
    set(RSACT_TARGET "aarch64-apple-darwin")
    set(RSACT_ARCHIVE_EXT "tar.gz")
    set(RSACT_ARCHIVE_SHA256 "b619abb0fcec0f47dc50a421edd390de6e7ed5df99946ccb1487892159a41caa")
    set(RSACT_LIB_NAME "librsact_ffi.a")
elseif(CMAKE_SYSTEM_NAME STREQUAL "Windows" AND CMAKE_SYSTEM_PROCESSOR MATCHES "^(AMD64|x86_64)$")
    set(RSACT_TARGET "x86_64-pc-windows-msvc")
    set(RSACT_ARCHIVE_EXT "zip")
    set(RSACT_ARCHIVE_SHA256 "50b98f8e69204eb9ed49eefcf399769f9fcdc89bdf553f1a9229aa898cc43cf6")
    set(RSACT_LIB_NAME "rsact_ffi.lib")
else()
    message(FATAL_ERROR
        "No prebuilt rsact-ffi binary is published for "
        "${CMAKE_SYSTEM_NAME}/${CMAKE_SYSTEM_PROCESSOR} in release ${RSACT_VERSION}. "
        "Build rsact-ffi from source (corrosion/cargo) instead, or add this "
        "target to Rsact's release workflow.")
endif()

include(FetchContent)
FetchContent_Declare(
    Rsact
    URL "https://github.com/MovingJu/Rsact/releases/download/${RSACT_VERSION}/rsact-${RSACT_TARGET}.${RSACT_ARCHIVE_EXT}"
    URL_HASH SHA256=${RSACT_ARCHIVE_SHA256}
    DOWNLOAD_EXTRACT_TIMESTAMP TRUE
)
FetchContent_MakeAvailable(Rsact)

add_library(rsact_ffi STATIC IMPORTED GLOBAL)
set_target_properties(rsact_ffi PROPERTIES
    IMPORTED_LOCATION "${rsact_SOURCE_DIR}/lib/${RSACT_LIB_NAME}"
    INTERFACE_INCLUDE_DIRECTORIES "${rsact_SOURCE_DIR}/include"
)

# The archive only ships librsact_ffi itself; rustc's own runtime deps
# (libc, libm, threads, dl, ...) normally get linked in by cargo/corrosion,
# so they have to be added by hand here instead.
if(CMAKE_SYSTEM_NAME STREQUAL "Linux")
    find_package(Threads REQUIRED)
    target_link_libraries(rsact_ffi INTERFACE Threads::Threads ${CMAKE_DL_LIBS} m)
elseif(CMAKE_SYSTEM_NAME STREQUAL "Darwin")
    target_link_libraries(rsact_ffi INTERFACE iconv)
endif()
# Windows (MSVC): rustc embeds the needed /DEFAULTLIB directives (kernel32,
# advapi32, ...) directly into rsact_ffi.lib, so link.exe resolves them
# automatically. If you hit unresolved externals, add the missing *.lib here.

add_library(rsact::rsact_ffi ALIAS rsact_ffi)

# And add executables as many as you want.

add_executable(animate-bin src/animate.c)
target_link_libraries(animate-bin PRIVATE rsact::rsact_ffi)

add_executable(basic-bin src/basic.c)
target_link_libraries(basic-bin PRIVATE rsact::rsact_ffi)

add_executable(input-bin src/input.c)
target_link_libraries(input-bin PRIVATE rsact::rsact_ffi)
