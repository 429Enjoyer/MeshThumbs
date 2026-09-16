// MIT. openNURBS and the dynamically linked Open CASCADE keep their own terms.
#include "rhino_tessellation.h"
#include <BRepBuilderAPI_MakeFace.hxx>
#include <BRepBuilderAPI_MakePolygon.hxx>
#include <BRepCheck_Analyzer.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <Poly_Triangulation.hxx>
#include <TopoDS_Face.hxx>
#include <TopoDS_Wire.hxx>
#include <gp_Pln.hxx>
#include <memory>

namespace {
using Ring = std::vector<ON_3dPoint>;
constexpr size_t MaxPoints = 4096, MaxLoops = 64;

void require(bool condition, const char *message) {
  if (!condition) throw std::runtime_error(message);
}

// A positive-weight Bezier lies in its control hull. Subdivision therefore
// bounds chord error without missing a high-curvature span between samples.
void flatten(const ON_BezierCurve &curve, double tolerance, Ring &points,
             unsigned depth, size_t &visits) {
  require(++visits <= 65536 && depth <= 20, "3DM curve subdivision limit exceeded");
  ON_3dPoint a, b;
  require(curve.GetCV(0, a) && curve.GetCV(curve.CVCount()-1, b) && a.IsValid() && b.IsValid(), "invalid 3DM curve endpoints");
  ON_Line chord(a, b);
  bool flat = true;
  for (int i = 0; i < curve.CVCount(); ++i) {
    ON_3dPoint p;
    require(curve.GetCV(i, p) && p.IsValid() && std::isfinite(curve.Weight(i)) && curve.Weight(i) > 0,
            "3DM profile requires finite positive rational weights");
    double t = 0.;
    chord.ClosestPointTo(p, &t);
    auto distance = p.DistanceTo(chord.PointAt(std::clamp(t, 0., 1.)));
    require(std::isfinite(distance), "3DM profile coordinate range overflow");
    flat = flat && distance <= tolerance;
  }
  if (!flat) {
    ON_BezierCurve left, right;
    require(curve.Split(.5, left, right), "cannot subdivide 3DM curve");
    flatten(left, tolerance, points, depth+1, visits);
    flatten(right, tolerance, points, depth+1, visits);
    return;
  }
  if (points.empty()) points.push_back(a);
  require(points.back().DistanceTo(a) <= tolerance, "disconnected 3DM curve spans");
  if (points.back() != b) points.push_back(b);
  require(points.size() <= MaxPoints + 1, "3DM profile point limit exceeded");
}

Ring sample(const ON_Curve &curve, bool closed) {
  require(curve.Degree() >= 1 && curve.Degree() <= 32 && curve.SpanCount() <= 4096,
          "3DM curve complexity limit exceeded");
  require(!closed || curve.IsClosed(), "3DM boundary is not closed");
  ON_NurbsCurve nurbs;
  require(curve.GetNurbForm(nurbs) != 0 && nurbs.IsValid() && nurbs.CVCount() <= 16384,
          "unsupported 3DM profile curve");
  const double scale = nurbs.BoundingBox().Diagonal().Length();
  require(std::isfinite(scale) && scale > 0., "degenerate 3DM profile");
  Ring points;
  size_t visits = 0;
  for (int span = 0; span <= nurbs.CVCount() - nurbs.Order(); ++span) {
    if (nurbs.Knot(span+nurbs.Order()-2) == nurbs.Knot(span+nurbs.Order()-1)) continue;
    ON_BezierCurve bezier;
    require(nurbs.ConvertSpanToBezier(span, bezier), "cannot read 3DM curve span");
    flatten(bezier, scale * .0005, points, 0, visits);
  }
  if (closed) {
    require(points.size() >= 4 && points.front().DistanceTo(points.back()) <= scale * 1e-8,
            "disconnected 3DM closed boundary");
    points.pop_back();
  }
  require(points.size() >= (closed ? 3u : 2u), "degenerate 3DM boundary");
  return points;
}

void emit(Scene &scene, Ring points, const ON_Xform &world, ON_Color color) {
  const double determinant = world.Determinant();
  require(std::isfinite(determinant) && determinant != 0., "singular 3DM instance transform");
  Face face;
  for (auto p : points) {
    p.Transform(world);
    face.push_back({p.x, p.z, -p.y, 0, 0, 0, color.Red()/255., color.Green()/255., color.Blue()/255., 1});
  }
  if (determinant < 0.) std::reverse(face.begin(), face.end());
  scene.add(std::move(face));
}

void planar(Scene &scene, std::vector<Ring> rings, ON_3dVector normal,
            const ON_Xform &world, ON_Color color) {
  require(!rings.empty() && rings.size() <= MaxLoops && normal.Unitize(), "invalid 3DM planar face");
  const ON_3dPoint origin = rings.front().front();
  double scale = 0.;
  size_t total = 0;
  for (const auto &ring : rings) {
    total += ring.size();
    for (auto p : ring) scale = std::max(scale, p.DistanceTo(origin));
  }
  require(total <= 32768 && std::isfinite(scale) && scale > 0., "3DM face boundary limit exceeded");
  // OCCT tolerances apply to normalized local coordinates, not model units or
  // a georeferenced origin. Restore original coordinates after triangulation.
  for (auto &ring : rings) for (auto &p : ring) {
    const auto local = (p-origin)/scale;
    require(std::abs(local*normal) <= 1e-7, "nonplanar 3DM boundary");
    p = ON_3dPoint(local);
  }
  const gp_Pln plane(gp_Pnt(0,0,0), gp_Dir(normal.x,normal.y,normal.z));
  auto wire = [&](Ring ring, bool outer) {
    ON_3dVector area(0,0,0);
    for (size_t i=0; i<ring.size(); ++i) area += ON_CrossProduct(ON_3dVector(ring[i]), ON_3dVector(ring[(i+1)%ring.size()]));
    require(std::abs(area*normal) > 1e-14, "zero-area 3DM boundary");
    if ((area*normal > 0.) != outer) std::reverse(ring.begin(), ring.end());
    BRepBuilderAPI_MakePolygon polygon;
    for (auto p : ring) polygon.Add(gp_Pnt(p.x,p.y,p.z));
    polygon.Close();
    require(polygon.IsDone(), "cannot construct 3DM planar boundary");
    return polygon.Wire();
  };
  BRepBuilderAPI_MakeFace builder(plane, wire(rings[0], true), true);
  for (size_t i=1; i<rings.size(); ++i) builder.Add(wire(rings[i], false));
  require(builder.IsDone() && BRepCheck_Analyzer(builder.Face()).IsValid(), "invalid 3DM planar trimming loops");
  const auto face = builder.Face();
  BRepMesh_IncrementalMesh mesher(face, .001, false, .1, false);
  require(mesher.IsDone(), "cannot mesh 3DM planar face");
  TopLoc_Location location;
  const auto triangles = BRep_Tool::Triangulation(face, location);
  require(!triangles.IsNull() && triangles->NbTriangles() > 0, "3DM planar face has no triangles");
  require(static_cast<uint64_t>(triangles->NbTriangles()) <= scene.limit - scene.triangles,
          "3DM generated mesh exceeds triangle limit");
  for (int i=1; i<=triangles->NbTriangles(); ++i) {
    int a,b,c; triangles->Triangle(i).Get(a,b,c);
    Ring points;
    for (int index : {a,b,c}) {
      auto p = triangles->Node(index).Transformed(location.Transformation());
      points.push_back(origin + ON_3dVector(p.X(),p.Y(),p.Z()) * scale);
    }
    // Independently enforce the requested surface orientation, including the
    // reversed bottom of an extrusion and reversed ON_BrepFace surfaces.
    if (ON_CrossProduct(points[1]-points[0],points[2]-points[0])*normal < 0.) std::swap(points[1],points[2]);
    emit(scene, std::move(points), world, color);
  }
}
}

void rhino_planar_face(Scene &scene, const ON_BrepFace &face,
                       const ON_Xform &world, ON_Color color) {
  ON_Plane plane;
  require(face.IsPlanar(&plane), "3DM uncached curved Brep faces are not supported");
  const auto *brep = face.Brep();
  require(brep && face.m_li.Count() > 0 && face.m_li.Count() <= static_cast<int>(MaxLoops), "invalid 3DM face loops");
  std::vector<Ring> rings(1);
  bool outer = false;
  for (int i=0; i<face.m_li.Count(); ++i) {
    const auto index = face.m_li[i];
    require(index >= 0 && index < brep->m_L.Count(), "invalid 3DM loop index");
    const auto &loop = brep->m_L[index];
    std::unique_ptr<ON_Curve> curve(brep->Loop3dCurve(loop));
    require(curve != nullptr, "cannot read 3DM trim boundary");
    if (loop.m_type == ON_BrepLoop::outer) {
      require(!outer, "multiple outer 3DM face loops");
      rings[0] = sample(*curve, true); outer = true;
    } else {
      require(loop.m_type == ON_BrepLoop::inner, "unsupported 3DM trim loop type");
      rings.push_back(sample(*curve, true));
    }
  }
  require(outer, "3DM face has no outer boundary");
  planar(scene, std::move(rings), face.m_bRev ? -plane.Normal() : plane.Normal(), world, color);
}

void rhino_extrusion(Scene &scene, const ON_Extrusion &extrusion,
                     const ON_Xform &world, ON_Color color) {
  require(extrusion.IsValid() && extrusion.ProfileCount() > 0 && extrusion.ProfileCount() <= static_cast<int>(MaxLoops), "invalid or excessive 3DM extrusion profiles");
  ON_Xform start, end;
  require(extrusion.GetProfileTransformation(0.,start) && extrusion.GetProfileTransformation(1.,end), "invalid 3DM extrusion placement");
  std::vector<Ring> bottom, top;
  size_t total = 0;
  for (int i=0; i<extrusion.ProfileCount(); ++i) {
    const auto *curve = extrusion.Profile(i);
    require(curve != nullptr, "missing 3DM extrusion profile");
    const bool closed = curve->IsClosed();
    auto points = sample(*curve, closed);
    total += points.size();
    require(total <= 32768, "3DM extrusion point limit exceeded");
    Ring a, b;
    for (auto p : points) { auto q=p; p.Transform(start); q.Transform(end); a.push_back(p); b.push_back(q); }
    for (size_t j=0; j<(closed ? points.size() : points.size()-1); ++j) {
      size_t k=(j+1)%points.size();
      emit(scene, {a[j],a[k],b[k],b[j]}, world, color);
    }
    bottom.push_back(std::move(a)); top.push_back(std::move(b));
  }
  auto normal = [](const ON_Xform &transform) {
    ON_3dVector x(1,0,0),y(0,1,0); x.Transform(transform); y.Transform(transform);
    return ON_CrossProduct(x,y);
  };
  const int caps = extrusion.IsCapped();
  if (caps & 1) planar(scene, std::move(bottom), -normal(start), world, color);
  if (caps & 2) planar(scene, std::move(top), normal(end), world, color);
}
