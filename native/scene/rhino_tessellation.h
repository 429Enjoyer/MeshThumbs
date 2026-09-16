#pragma once
#include "scene.h"
#include <opennurbs.h>

// Original MeshThumbs adapters. No Rhino meshing engine or Rhino installation.
void rhino_planar_face(Scene &, const ON_BrepFace &, const ON_Xform &, ON_Color);
void rhino_extrusion(Scene &, const ON_Extrusion &, const ON_Xform &, ON_Color);
