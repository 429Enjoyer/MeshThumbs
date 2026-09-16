# Preserve the private-storage diagnostic while using portable wide-string
# macro expansion (MSVC accepts L#c; GCC needs two-stage token pasting).
set(header "${OPENNURBS_SOURCE_DIR}/opennurbs_internal_defines.h")
file(READ "${header}" content)
if(content MATCHES "L#c")
  string(REPLACE "#include <unordered_map>" "#include <unordered_map>\n#define ON_WIDE_PRIVATE_I(x) L ## x\n#define ON_WIDE_PRIVATE(x) ON_WIDE_PRIVATE_I(x)" content "${content}")
  string(REPLACE "L#c" "ON_WIDE_PRIVATE(#c)" content "${content}")
  file(WRITE "${header}" "${content}")
elseif(NOT content MATCHES "ON_WIDE_PRIVATE")
  message(FATAL_ERROR "Unexpected openNURBS version for compatibility patch")
endif()

# The Windows UUID API is available to MinGW as well as MSVC.
set(uuid_source "${OPENNURBS_SOURCE_DIR}/opennurbs_uuid.cpp")
file(READ "${uuid_source}" content)
string(REPLACE "#if defined(ON_COMPILER_MSC)\n  // Header: Declared in Rpcdce.h."
  "#if defined(ON_RUNTIME_WIN) // MeshThumbs: include MinGW\n  // Header: Declared in Rpcdce.h." content "${content}")
file(WRITE "${uuid_source}" "${content}")
