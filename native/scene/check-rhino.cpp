// Original MIT geometry regression checks; downloaded showcase assets remain separate.
#include "rhino_tessellation.h"
#include <iostream>
#include <memory>

static void check(bool ok, const char *message) {
  if (!ok) throw std::runtime_error(message);
}
static ON_PolylineCurve *rectangle(double x0, double y0, double x1, double y1) {
  ON_Polyline p;
  for (auto point : {ON_3dPoint(x0,y0,0), ON_3dPoint(x1,y0,0), ON_3dPoint(x1,y1,0), ON_3dPoint(x0,y1,0), ON_3dPoint(x0,y0,0)}) p.Append(point);
  return new ON_PolylineCurve(p);
}
static ON_Extrusion prism(bool hole = false) {
  ON_Extrusion e;
  check(e.SetPathAndUp(ON_3dPoint(0,0,0),ON_3dPoint(0,0,2),ON_3dVector(0,1,0)), "path");
  check(e.SetOuterProfile(rectangle(-2,-2,2,2), true), "outer");
  if (hole) check(e.AddInnerProfile(rectangle(-1,-1,1,1)), "hole");
  return e;
}
static std::pair<double,double> measure(const Scene &scene) {
  double area = 0., volume = 0.;
  for (const auto &face : scene.faces) {
    auto p = [](const Vertex &v) { return ON_3dVector(v[0],v[1],v[2]); };
    for (size_t i=1; i+1<face.size(); ++i) {
      auto a=p(face[0]), b=p(face[i]), c=p(face[i+1]);
      area += ON_CrossProduct(b-a,c-a).Length()/2.;
      volume += a*ON_CrossProduct(b,c)/6.;
    }
  }
  return {area,volume};
}
static void write(const ON_Geometry &geometry, const std::string &path) {
  ONX_Model model;
  ON_3dmObjectAttributes attributes; attributes.SetColorSource(ON::color_from_object); attributes.m_color=ON_Color(48,148,196);
  model.AddModelGeometryComponent(&geometry,&attributes);
  check(model.Write(path.c_str(), 80), "fixture write failed");
}
int main(int argc, char **argv) {
  try {
    ON::Begin();
    if (argc == 4 && std::string(argv[1]) == "--strip-cache") {
      ONX_Model model;
      check(model.Read(argv[2]), "source read failed");
      ONX_ModelComponentIterator it(model,ON_ModelComponent::Type::ModelGeometry);
      int changed = 0;
      for(auto c=it.FirstComponent(); c; c=it.NextComponent()) {
        auto item=ON_ModelGeometryComponent::Cast(c);
        auto g=item->ExclusiveGeometry();
        check(g != nullptr, "shared geometry cannot be edited");
        if(auto e=ON_Extrusion::Cast(g)) { e->DestroyMesh(ON::any_mesh); ++changed; }
      }
      check(changed>0 && model.Write(argv[3],80), "cache removal failed");
      std::cout << "Removed extrusion mesh caches; analytic geometry retained\n";
      return 0;
    }
    if (argc > 1 && std::string(argv[1]) == "--audit") {
      for (int i=2; i<argc; ++i) {
        ONX_Model model;
        check(model.Read(argv[i]), "audit read failed");
        std::cout << argv[i] << '\n';
        ONX_ModelComponentIterator it(model,ON_ModelComponent::Type::ModelGeometry);
        for (auto c=it.FirstComponent(); c; c=it.NextComponent()) {
          auto item=ON_ModelGeometryComponent::Cast(c); auto g=item->Geometry(nullptr);
          std::cout << "  " << g->ClassId()->ClassName();
          if (auto e=ON_Extrusion::Cast(g)) std::cout << " profiles=" << e->ProfileCount() << " cached=" << bool(e->Mesh(ON::render_mesh));
          if (auto b=ON_Brep::Cast(g)) for (int f=0; f<b->m_F.Count(); ++f) std::cout << " face" << f << "(plane=" << b->m_F[f].IsPlanar() << ",cached=" << bool(b->m_F[f].Mesh(ON::render_mesh)) << ")";
          std::cout << '\n';
        }
      }
      return 0;
    }
    const ON_Color color(48,148,196);
    auto solid=prism(), hollow=prism(true);
    for (bool hole : {false,true}) {
      Scene scene(10000);
      rhino_extrusion(scene,hole ? hollow : solid,ON_Xform::IdentityTransformation,color);
      auto [area,volume]=measure(scene);
      check(std::abs(area-(hole ? 72. : 64.))<1e-6, "extrusion area / cap hole");
      check(std::abs(volume-(hole ? 24. : 32.))<1e-6, "extrusion volume / winding");
      for (auto &f:scene.faces) for(auto &v:f) check(std::abs(v[6]-48./255.)<1e-12, "face color");
    }
    ON_Xform mirrored=ON_Xform::IdentityTransformation;
    mirrored[0][0]=-2; mirrored[1][1]=3; mirrored[2][2]=4; mirrored[0][3]=11;
    Scene reflected(10000); rhino_extrusion(reflected,solid,mirrored,color);
    check(std::abs(measure(reflected).second-32.*24.)<1e-6, "mirrored/scaled extrusion orientation");
    auto mitered=solid;
    check(mitered.SetMiterPlaneNormal(ON_3dVector(.25,0,1),1), "miter plane");
    Scene miter(10000); rhino_extrusion(miter,mitered,ON_Xform::IdentityTransformation,color);
    check(std::abs(measure(miter).second-32.)<1e-6, "mitered end cap / volume");
    ON_Extrusion open;
    check(open.SetPathAndUp(ON_3dPoint(0,0,0),ON_3dPoint(0,0,2),ON_3dVector(0,1,0)), "open path");
    ON_Polyline line; line.Append(ON_3dPoint(0,0,0)); line.Append(ON_3dPoint(2,0,0)); line.Append(ON_3dPoint(2,1,0));
    check(open.SetOuterProfile(new ON_PolylineCurve(line),false), "open profile");
    Scene curtain(10000); rhino_extrusion(curtain,open,ON_Xform::IdentityTransformation,color);
    check(std::abs(measure(curtain).first-6.)<1e-6 && curtain.faces.size()==2, "open extrusion has no invented caps");
    ON_Extrusion round;
    check(round.SetPathAndUp(ON_3dPoint(0,0,0),ON_3dPoint(0,0,2),ON_3dVector(0,1,0)), "round path");
    check(round.SetOuterProfile(new ON_ArcCurve(ON_Circle(ON_Plane::World_xy,2.)),true), "round profile");
    check(round.AddInnerProfile(new ON_ArcCurve(ON_Circle(ON_Plane::World_xy,1.))), "round hole");
    Scene tube(10000); rhino_extrusion(tube,round,ON_Xform::IdentityTransformation,color);
    check(std::abs(measure(tube).second-6.*ON_PI)<.05, "adaptive curved profile / holes");
    std::unique_ptr<ON_Brep> brep(hollow.BrepForm());
    check(bool(brep), "Brep conversion");
    Scene flat(10000);
    for(int i=0;i<brep->m_F.Count();++i) rhino_planar_face(flat,brep->m_F[i],ON_Xform::IdentityTransformation,color);
    check(std::abs(measure(flat).second-24.)<1e-6 && std::abs(measure(flat).first-72.)<1e-6, "uncached planar Brep with holes");
    Scene reversed(10000); brep->Flip();
    for(int i=0;i<brep->m_F.Count();++i) rhino_planar_face(reversed,brep->m_F[i],ON_Xform::IdentityTransformation,color);
    check(std::abs(measure(reversed).second+24.)<1e-6, "reversed Brep faces");
    ON_PlaneSurface surface(ON_Plane::World_xy);
    check(surface.SetExtents(0,ON_Interval(-2,2),true) && surface.SetExtents(1,ON_Interval(-3,3),true), "plane extents");
    std::unique_ptr<ON_Brep> surfaceBrep(surface.BrepForm());
    check(surfaceBrep && surfaceBrep->m_F.Count()==1, "planar surface Brep");
    Scene sheet(10000); rhino_planar_face(sheet,surfaceBrep->m_F[0],ON_Xform::IdentityTransformation,color);
    check(std::abs(measure(sheet).first-24.)<1e-6, "standalone planar surface area");
    bool limited=false;
    try { Scene tiny(1); rhino_extrusion(tiny,solid,ON_Xform::IdentityTransformation,color); }
    catch(const std::exception &) { limited=true; }
    check(limited,"triangle budget");
    if(argc > 1) {
      write(solid,std::string(argv[1])+"/extrusion.3dm");
      write(hollow,std::string(argv[1])+"/hollow.3dm");
      write(round,std::string(argv[1])+"/tube.3dm");
      write(mitered,std::string(argv[1])+"/miter.3dm");
      write(open,std::string(argv[1])+"/open.3dm");
      write(surface,std::string(argv[1])+"/surface.3dm");
      brep->Flip(); write(*brep,std::string(argv[1])+"/planar-brep.3dm");
    }
    std::cout << "3DM geometry checks passed: caps, holes, curved profiles, transforms, colors, winding and budget\n";
    return 0;
  } catch(const std::exception &e) { std::cerr << e.what() << '\n'; return 1; }
}
