set shell := ["bash", "-c"]

rootdir := justfile_directory()
xdg_data_dir := `echo "${XDG_DATA_HOME:-$HOME/.local/share}/snowcap"`
root_xdg_data_dir := "/usr/share/snowcap"

lua_version := "5.4"

list:
    @just --list --unsorted

install: install-protos install-lua-lib

install-protos:
    #!/usr/bin/env bash
    set -euxo pipefail
    proto_dir="{{xdg_data_dir}}/protobuf"
    rm -rf "${proto_dir}"
    mkdir -p "{{xdg_data_dir}}"
    cp -r "{{rootdir}}/api/protobuf" "${proto_dir}"

install-lua-lib: gen-lua-pb-defs
    #!/usr/bin/env bash
    set -euxo pipefail
    cd "{{rootdir}}/api/lua"
    luarocks make --local --lua-version "{{lua_version}}"

clean:
    rm -rf "{{xdg_data_dir}}"
    -luarocks remove --local snowcap-api

# Generate the protobuf definitions Lua file
gen-lua-pb-defs:
    #!/usr/bin/env bash
    set -euxo pipefail
    cargo build --package lua-build
    ./target/debug/lua-build > "./api/lua/snowcap/grpc/defs.lua"
