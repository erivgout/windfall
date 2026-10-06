#!/usr/bin/env bash
# Source this from Git Bash before running cargo on Windows:
#
#   source scripts/msvc-env.sh
#
# Rust picks the newest Visual Studio install. If that install lacks the x64
# C++ libraries, linking fails. This script finds an install that has them and
# loads its vcvars64 environment, which makes rustc and the cc crate use it.

_windfall_msvc_env() {
  local vswhere="/c/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe"
  if [ ! -x "$vswhere" ]; then
    echo "msvc-env: vswhere.exe not found; install the Visual Studio Build Tools" >&2
    return 1
  fi

  local install vcvars="" found=""
  while IFS= read -r install; do
    install="${install%$'\r'}"
    [ -z "$install" ] && continue
    local unix_install
    unix_install="$(cygpath -u "$install")"
    local tools
    for tools in "$unix_install"/VC/Tools/MSVC/*/; do
      if [ -f "${tools}lib/x64/msvcrt.lib" ] && [ -f "$unix_install/VC/Auxiliary/Build/vcvars64.bat" ]; then
        vcvars="$install\\VC\\Auxiliary\\Build\\vcvars64.bat"
        found="$install"
        break 2
      fi
    done
  done < <("$vswhere" -all -products '*' -property installationPath)

  if [ -z "$vcvars" ]; then
    echo "msvc-env: no Visual Studio install with the x64 C++ libraries was found." >&2
    echo "msvc-env: add the 'Desktop development with C++' workload." >&2
    return 1
  fi

  # Quoting a path with spaces and parentheses through cmd.exe /c from Git
  # Bash is unreliable, so the call goes through a throwaway batch file.
  local bat
  bat="$(mktemp --suffix=.bat)"
  printf '@echo off\r\ncall "%s" >nul 2>&1\r\nset\r\n' "$vcvars" >"$bat"

  local line name value
  while IFS= read -r line; do
    line="${line%$'\r'}"
    name="${line%%=*}"
    value="${line#*=}"
    case "$name" in
      PATH | Path) export PATH="$(cygpath -u -p "$value")" ;;
      LIB | LIBPATH | INCLUDE | EXTERNAL_INCLUDE | VCINSTALLDIR | VCToolsInstallDir | \
        VCToolsVersion | VSINSTALLDIR | VisualStudioVersion | WindowsSdkDir | \
        WindowsSDKVersion | WindowsSDKLibVersion | WindowsSdkBinPath | \
        WindowsSdkVerBinPath | UCRTVersion | UniversalCRTSdkDir | \
        VSCMD_ARG_HOST_ARCH | VSCMD_ARG_TGT_ARCH | VSCMD_VER | Platform)
        export "$name=$value"
        ;;
    esac
  done < <(cmd.exe //d //c "$(cygpath -w "$bat")")
  rm -f "$bat"

  export PATH="$HOME/.cargo/bin:$PATH"
  echo "msvc-env: using $found" >&2
}

_windfall_msvc_env
unset -f _windfall_msvc_env
