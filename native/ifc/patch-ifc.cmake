# Compatibility edits to LGPL-3.0-or-later IfcOpenShell, shipped with its sources.
if(NOT EXISTS "${IFC_SOURCE_DIR}/src/ifcgeom")
  message(FATAL_ERROR "Set IFC_SOURCE_DIR to the pinned IfcOpenShell source")
endif()
# OCCT 7.9 removed this obsolete typedef; the underlying sewing class is the same.
foreach(name base_utils layerset wire_utils)
  set(path "${IFC_SOURCE_DIR}/src/ifcgeom/kernels/opencascade/${name}.cpp")
  file(READ "${path}" text)
  string(REPLACE "BRepOffsetAPI_Sewing" "BRepBuilderAPI_Sewing" patched "${text}")
  if(NOT patched STREQUAL text)
    file(WRITE "${path}" "${patched}")
  endif()
endforeach()
# MinGW's fallback real-number parser calls tellg() after reaching EOF, where
# it returns -1. That made complete valid decimal tokens look invalid.
set(path "${IFC_SOURCE_DIR}/src/ifcparse/IfcParse.cpp")
file(READ "${path}" text)
set(old [=[    double d;
    std::stringstream ss;
    ss.imbue(std::locale::classic());
    ss << start;
    ss >> d;
    size_t nread = ss.tellg();
    *end = const_cast<char*>(start) + nread;
    return d;]=])
set(new [=[    double d = 0.;
    std::istringstream ss(start);
    ss.imbue(std::locale::classic());
    if (!(ss >> d)) {
        *end = const_cast<char*>(start);
        return 0.;
    }
    const auto nread = ss.eof() ? strlen(start) : static_cast<size_t>(ss.tellg());
    *end = const_cast<char*>(start) + nread;
    return d;]=])
string(REPLACE "${old}" "${new}" patched "${text}")
if(NOT patched STREQUAL text)
  file(WRITE "${path}" "${patched}")
elseif(NOT text MATCHES "std::istringstream ss\\(start\\)")
  message(FATAL_ERROR "Pinned MinGW float-parser block changed")
endif()
set(path "${IFC_SOURCE_DIR}/src/ifcparse/buildinfo.cpp")
file(READ "${path}" text)
string(REPLACE "const char *IFCOPENSHELL_VERSION = \"0.8.0\";"
  "const char *IFCOPENSHELL_VERSION = \"0.8.5-meshthumbs-mingw\";" patched "${text}")
if(NOT patched STREQUAL text)
  file(WRITE "${path}" "${patched}")
endif()
