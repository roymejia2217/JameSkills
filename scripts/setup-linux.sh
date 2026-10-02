#!/usr/bin/env bash
set -euo pipefail

usage() {
    printf '%s\n' 'Usage: setup-linux.sh --check | --print-install-plan | --help'
}

mode="${1:-}"
if [[ "$#" -ne 1 ]]; then
    usage >&2
    exit 2
fi
if [[ "$mode" == "--help" ]]; then
    usage
    exit 0
fi
if [[ "$mode" != "--check" && "$mode" != "--print-install-plan" ]]; then
    usage >&2
    exit 2
fi

if [[ "$mode" == "--print-install-plan" ]]; then
    os_id="unknown"
    if [[ -r /etc/os-release ]]; then
        # This is the local operating system's trusted release metadata.
        # shellcheck disable=SC1091
        source /etc/os-release
        os_id="${ID:-unknown}"
    fi
    if [[ "$os_id" == "debian" || "$os_id" == "ubuntu" ]]; then
        cat <<'JSON'
{"schema_version":1,"platform":"linux","package_manager":"apt","packages":["build-essential","clang","cmake","pkg-config","libfontconfig1-dev","libfreetype-dev","libwayland-dev","libx11-xcb-dev","libxcb1-dev","libxkbcommon-dev","libxkbcommon-x11-dev","libssl-dev","libzstd-dev","libvulkan1"],"manual_steps":["Install the pinned Rust 1.95.0 toolchain with rustfmt and clippy using rustup.","Confirm a Vulkan driver and a Wayland or X11 session for opening the native window."],"command_preview":"sudo apt-get install build-essential clang cmake pkg-config libfontconfig1-dev libfreetype-dev libwayland-dev libx11-xcb-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev libssl-dev libzstd-dev libvulkan1","sources":["https://gpui-kit.com/docs/installation","https://packages.debian.org/","https://packages.ubuntu.com/"]}
JSON
    else
        printf '{"schema_version":1,"platform":"linux","package_manager":"manual","packages":[],"manual_steps":["Use the GPUI Kit installation guide and your distribution package index to provide Rust 1.95.0, CMake, fontconfig, freetype, Wayland/X11, XKB, Vulkan loader and compiler development packages."],"sources":["https://gpui-kit.com/docs/installation"]}\n'
    fi
    exit 0
fi

checks=()
required_missing=0
add_check() {
    local id="$1" status="$2" summary="$3" source="$4"
    checks+=("{\"id\":\"$id\",\"status\":\"$status\",\"summary\":\"$summary\",\"source\":\"$source\"}")
}
required_check() {
    local id="$1" status="$2" summary="$3" source="$4"
    add_check "$id" "$status" "$summary" "$source"
    if [[ "$status" != "pass" ]]; then
        required_missing=1
    fi
}
check_command() {
    local name="$1" source="$2"
    if command -v "$name" >/dev/null 2>&1; then
        required_check "$name" pass "$name is on PATH" "$source"
    else
        required_check "$name" missing "$name is not on PATH" "$source"
    fi
}
check_pkg_config() {
    local module="$1"
    if command -v pkg-config >/dev/null 2>&1 && pkg-config --exists "$module"; then
        required_check "pkg:$module" pass "pkg-config module $module is available" "https://gpui-kit.com/docs/installation"
    else
        required_check "pkg:$module" missing "pkg-config module $module is missing" "https://gpui-kit.com/docs/installation"
    fi
}

if command -v rustc >/dev/null 2>&1 && rustc --version | grep -Eq '^rustc 1\.95\.0 '; then
    required_check rust pass 'Rust 1.95.0 is active on PATH' 'https://doc.rust-lang.org/1.95.0/'
else
    required_check rust missing 'Rust 1.95.0 is not active on PATH' 'https://rustup.rs/'
fi
check_command cargo 'https://rustup.rs/'
check_command cmake 'https://gpui-kit.com/docs/installation'
check_command pkg-config 'https://gpui-kit.com/docs/installation'
check_command c++ 'https://gpui-kit.com/docs/installation'
check_command clang 'https://gpui-kit.com/docs/installation'
for module in fontconfig freetype2 wayland-client xcb xkbcommon xkbcommon-x11; do
    check_pkg_config "$module"
done
vulkan_loader_listing=''
if command -v ldconfig >/dev/null 2>&1; then
    vulkan_loader_listing="$(ldconfig -p 2>/dev/null || true)"
fi
if [[ "$vulkan_loader_listing" == *libvulkan.so.1* ]]; then
    required_check vulkan-loader pass 'Vulkan loader is registered with the dynamic linker' 'https://gpui-kit.com/docs/installation'
else
    required_check vulkan-loader missing 'Vulkan loader libvulkan.so.1 was not found' 'https://packages.debian.org/libvulkan1'
fi

if [[ -n "${WAYLAND_DISPLAY:-}" || -n "${DISPLAY:-}" ]]; then
    add_check display configured 'Wayland/X11 display environment is configured for this process' 'https://gpui-kit.com/docs/installation'
else
    add_check display unknown 'No Wayland/X11 display is configured for this process' 'https://gpui-kit.com/docs/installation'
fi
if [[ -d /dev/dri || -e /dev/nvidia0 ]]; then
    add_check gpu present 'A common Linux GPU device path is present; renderer still requires a native smoke test' 'https://gpui-kit.com/docs/installation'
else
    add_check gpu unknown 'No common GPU device path was observed; renderer is not verified' 'https://gpui-kit.com/docs/installation'
fi

printf '{"schema_version":1,"platform":"linux","architecture":"%s","checks":[%s]}\n' \
    "$(uname -m)" "$(IFS=,; printf '%s' "${checks[*]}")"
exit "$required_missing"
