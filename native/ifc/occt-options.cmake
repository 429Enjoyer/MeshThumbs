include("${CMAKE_CURRENT_LIST_DIR}/../step/occt-options.cmake")
# IFC swept surfaces and offsets also need modeling toolkits not used by STEP import.
set(BUILD_TOOLKITS "TKDESTEP;TKDEIGES;TKMesh;TKOffset;TKFillet;TKBin;TKXMesh" CACHE STRING "" FORCE)
