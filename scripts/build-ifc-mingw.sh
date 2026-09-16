#!/usr/bin/env bash
# Cross-build the IFC4-only helper. Never install compiler/runtime files globally.
set -euo pipefail
project_root=$(cd "$(dirname "$0")/.." && pwd)
ifc_cache=${MESHTHUMBS_IFC_CACHE:-"$HOME/.cache/meshthumbs-ifc"}
occt_prefix=${MESHTHUMBS_OCCT_PREFIX:-"$ifc_cache/occt-install"}
output_dir=${1:-"$project_root/.tools/ifc-mingw"}
jobs=${MESHTHUMBS_BUILD_JOBS:-4}
mkdir -p "$ifc_cache" "$output_dir"
fetch() {
    local name=$1 url=$2 hash=$3
    if [ ! -f "$ifc_cache/$name" ]; then curl --fail --location --retry 2 "$url" -o "$ifc_cache/$name"; fi
    echo "$hash  $ifc_cache/$name" | sha256sum --check --status
}
fetch boost.tar.xz https://github.com/boostorg/boost/releases/download/boost-1.86.0/boost-1.86.0-b2-nodocs.tar.xz a4d99d032ab74c9c5e76eddcecc4489134282245fffa7e079c5804b92b45f51d
fetch eigen.tar.gz https://gitlab.com/libeigen/eigen/-/archive/3.3.9/eigen-3.3.9.tar.gz 7985975b787340124786f092b3a07d594b2e9cd53bbfe5f3d9b1daee7d55f56f
fetch json.hpp https://raw.githubusercontent.com/nlohmann/json/v3.6.1/single_include/nlohmann/json.hpp d2eeb25d2e95bffeb08ebb7704cdffd2e8fca7113eba9a0b38d60a5c391ea09a
if [ ! -f "$occt_prefix/inc/BRepOffsetAPI_ThruSections.hxx" ]; then
    fetch OCCT-7.9.3.tar.gz https://codeload.github.com/Open-Cascade-SAS/OCCT/tar.gz/refs/tags/V7_9_3 5ecf094ec6b12d5413dfb851d8c3590c354058aee556e32e408bdfbf8c357d57
    if [ ! -d "$ifc_cache/OCCT-7_9_3" ]; then tar -xzf "$ifc_cache/OCCT-7.9.3.tar.gz" -C "$ifc_cache"; fi
    cmake -S "$ifc_cache/OCCT-7_9_3" -B "$ifc_cache/occt-build" \
        -C "$project_root/native/ifc/occt-options.cmake" \
        -DCMAKE_TOOLCHAIN_FILE="$project_root/native/step/mingw-toolchain.cmake" \
        -DCMAKE_BUILD_TYPE=Release -DCMAKE_INSTALL_PREFIX="$occt_prefix"
    cmake --build "$ifc_cache/occt-build" --parallel "$jobs"
    cmake --install "$ifc_cache/occt-build"
fi
if [ ! -d "$ifc_cache/boost-1.86.0" ]; then tar -xJf "$ifc_cache/boost.tar.xz" -C "$ifc_cache"; fi
if [ ! -d "$ifc_cache/eigen-3.3.9" ]; then tar -xzf "$ifc_cache/eigen.tar.gz" -C "$ifc_cache"; fi
mkdir -p "$ifc_cache/json-include/nlohmann"
cp "$ifc_cache/json.hpp" "$ifc_cache/json-include/nlohmann/json.hpp"
source_archive="$project_root/docs/third-party/ifcconvert-source-0.8.5.tar.gz"
echo "d4e95a9cee6a5d4f7dd118cb3365b2fb056a6e586085fe9ea087472de1b9ac45  $source_archive" | sha256sum --check --status
source_dir="$ifc_cache/IfcOpenShell-1c5b825d8ef05ab9d14a15dac12e9eae2f5a37c2"
if [ ! -d "$source_dir" ]; then tar -xzf "$source_archive" -C "$ifc_cache"; fi
cmake -DIFC_SOURCE_DIR="$source_dir" -P "$project_root/native/ifc/patch-ifc.cmake"
(
    cd "$ifc_cache/boost-1.86.0"
    if [ ! -x b2 ]; then ./bootstrap.sh; fi
    printf 'using gcc : mingw : x86_64-w64-mingw32-g++-posix ;\n' > "$ifc_cache/boost-user-config.jam"
    ./b2 --user-config="$ifc_cache/boost-user-config.jam" toolset=gcc-mingw target-os=windows address-model=64 \
        threading=multi threadapi=win32 link=static runtime-link=shared variant=release --layout=system \
        --with-system --with-program_options --with-regex --with-thread --with-date_time --with-iostreams \
        --with-filesystem -sNO_BZIP2=1 -sNO_ZLIB=1 -sNO_LZMA=1 -sNO_ZSTD=1 -sNO_ICU=1 \
        --prefix="$ifc_cache/boost-install" install -j"$jobs"
)
cmake -S "$source_dir/cmake" -B "$ifc_cache/build" \
    -DCMAKE_TOOLCHAIN_FILE="$project_root/native/step/mingw-toolchain.cmake" \
    -DCMAKE_BUILD_TYPE=Release -DCMAKE_CXX_FLAGS=-Wa,-mbig-obj -DCMAKE_EXE_LINKER_FLAGS= -DSCHEMA_VERSIONS=4 \
    -DBUILD_IFCPYTHON=OFF -DBUILD_CONVERT=ON -DBUILD_EXAMPLES=OFF -DBUILD_GEOMSERVER=OFF \
    -DWITH_CGAL=OFF -DCOLLADA_SUPPORT=OFF -DHDF5_SUPPORT=OFF -DIFCXML_SUPPORT=OFF \
    -DGLTF_SUPPORT=ON -DWITH_PROJ=OFF -DWITH_ROCKSDB=OFF \
    -DOpenCASCADE_DIR="$occt_prefix/cmake" \
    -DBoost_DIR="$ifc_cache/boost-install/lib/cmake/Boost-1.86.0" \
    -DEIGEN_DIR="$ifc_cache/eigen-3.3.9" -DJSON_INCLUDE_DIR="$ifc_cache/json-include" \
    -Djson_header_path="$ifc_cache/json-include"
cmake --build "$ifc_cache/build" --target IfcConvert --parallel "$jobs"
converter=$(find "$ifc_cache/build" -type f -name IfcConvert.exe -print -quit)
test -n "$converter"
cp "$converter" "$output_dir/IfcConvert.exe"
x86_64-w64-mingw32-g++-posix "$project_root/native/ifc/check-float.cpp" \
    "$ifc_cache/build/ifcparse/libIfcParse.a" -L"$ifc_cache/boost-install/lib" \
    -lboost_iostreams -lboost_thread -lboost_system -lboost_date_time -lboost_chrono \
    -lboost_atomic -lboost_regex -lbcrypt -o "$output_dir/check-float.exe"
# Include the same OCCT/GCC libraries already used by the CAD backend. The
# executable imports only these runtime families and Windows system libraries.
find "$occt_prefix" -type f -iname '*.dll' -exec cp '{}' "$output_dir/" \;
for name in libgcc_s_seh-1.dll libstdc++-6.dll libwinpthread-1.dll; do
    runtime=$(x86_64-w64-mingw32-g++-posix -print-file-name="$name")
    test -f "$runtime"
    cp "$runtime" "$output_dir/$name"
done
cp "$ifc_cache/eigen.tar.gz" "$output_dir/eigen-source-3.3.9.tar.gz"
cp "$ifc_cache/json.hpp" "$output_dir/json-3.6.1.hpp"
python3 - "$output_dir" "$project_root" <<'PY'
import hashlib, json, pathlib, subprocess, sys
out, root = map(pathlib.Path, sys.argv[1:])
files = {}
for path in sorted(out.iterdir()):
    if path.suffix.lower() in {'.exe', '.dll'}:
        pe = subprocess.check_output(['x86_64-w64-mingw32-objdump', '-p', str(path)], text=True)
        if any(name in pe.lower() for name in ['dll name: msvcp', 'dll name: vcruntime']):
            raise SystemExit(f'Microsoft runtime dependency found in {path.name}')
    if path.name != 'check-float.exe' and path.suffix.lower() in {'.exe', '.dll', '.gz', '.hpp'}:
        files[path.name] = hashlib.sha256(path.read_bytes()).hexdigest()
inputs = {name: hashlib.sha256((root/name).read_bytes()).hexdigest() for name in [
    'scripts/build-ifc-mingw.sh', 'native/ifc/patch-ifc.cmake', 'native/ifc/check-float.cpp',
    'native/ifc/occt-options.cmake', 'native/step/occt-options.cmake',
    'native/step/mingw-toolchain.cmake', 'docs/third-party/ifcconvert-source-0.8.5.tar.gz']}
manifest = dict(source='1c5b825d8ef05ab9d14a15dac12e9eae2f5a37c2', schema='IFC4',
                compiler=subprocess.check_output(['x86_64-w64-mingw32-g++-posix', '--version'], text=True).splitlines()[0],
                occt='7.9.3', boost='1.86.0', eigen='3.3.9', nlohmann_json='3.6.1', files=files, inputs=inputs)
(out/'build-info.json').write_text(json.dumps(manifest, indent=2)+'\n')
PY
echo "IFC4 MinGW helper built in $output_dir"
