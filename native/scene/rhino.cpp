#include "scene.h"
#include <functional>
#include <memory>
#include <mutex>
#include <opennurbs.h>
#include "rhino_tessellation.h"
static std::mutex rhinoMutex;

static void mesh(Scene &scene, const ON_Mesh &mesh, const ON_Xform &world,
                 ON_Color color) {
  if (mesh.VertexCount() > 15000000 || mesh.FaceCount() > 5000000)
    throw std::runtime_error("3DM mesh limit exceeded");
  const auto determinant = world.Determinant();
  if (!std::isfinite(determinant) || determinant == 0)
    throw std::runtime_error("singular 3DM instance transform");
  ON_Xform normal = world.Inverse();
  normal.Transpose();
  for (int f = 0; f < mesh.FaceCount(); ++f) {
    const auto &source = mesh.m_F[f];
    int count = source.IsTriangle() ? 3 : 4;
    Face face;
    for (int c = 0; c < count; ++c) {
      int index = source.vi[c];
      if (index < 0 || index >= mesh.VertexCount())
        throw std::runtime_error("3DM vertex index out of range");
      ON_3dPoint p = mesh.Vertex(index);
      p.Transform(world);
      ON_3dVector n(0, 0, 0);
      if (mesh.HasVertexNormals()) {
        n = ON_3dVector(mesh.m_N[index]);
        n.Transform(normal);
        n.Unitize();
      }
      auto rgb = mesh.HasVertexColors() ? mesh.m_C[index] : color;
      face.push_back({p.x, p.z, -p.y, n.x, n.z, -n.y, rgb.Red() / 255.,
                      rgb.Green() / 255., rgb.Blue() / 255., 1});
    }
    if (determinant < 0)
      std::reverse(face.begin(), face.end());
    scene.add(std::move(face));
  }
}
EXPORT int meshthumbs_3dm_load(const std::uint16_t *path, std::uint64_t budget,
                               PolygonSink sink, void *context, char *error,
                               std::uint32_t capacity) {
  try {
    if (!sink || !budget)
      throw std::runtime_error("invalid 3DM request");
    std::lock_guard<std::mutex> lock(rhinoMutex);
    ON::Begin();
    auto file = input_path(path);
    ONX_Model model;
    std::unique_ptr<FILE, decltype(&fclose)> input(
        _wfopen(file.wstring().c_str(), L"rb"), fclose);
    if (!input)
      throw std::runtime_error("cannot open 3DM file");
    ON_BinaryFile archive(ON::archive_mode::read3dm, input.get());
    if (!model.Read(archive, nullptr))
      throw std::runtime_error("invalid 3DM model");
    Scene scene(budget);
    size_t visited = 0;
    std::function<bool(const ON_Layer *, int)> visibleLayer =
        [&](const ON_Layer *layer, int depth) {
          if (depth > 64)
            throw std::runtime_error("3DM layer hierarchy limit exceeded");
          if (!layer)
            return true;
          if (!layer->IsVisible())
            return false;
          if (ON_UuidIsNil(layer->ParentLayerId()))
            return true;
          return visibleLayer(
              ON_Layer::FromModelComponentRef(
                  model.LayerFromId(layer->ParentLayerId()), nullptr),
              depth + 1);
        };
    std::function<void(const ON_ModelGeometryComponent *, const ON_Xform &,
                       int)>
        visit;
    visit = [&](const ON_ModelGeometryComponent *item, const ON_Xform &world,
                int depth) {
      if (++visited > 100000 || depth > 64)
        throw std::runtime_error("3DM instance expansion limit exceeded");
      if (!item || !item->Geometry(nullptr))
        throw std::runtime_error("missing 3DM instance geometry");
      const auto *attr = item->Attributes(nullptr);
      if (attr && !attr->IsVisible())
        return;
      if (attr &&
          !visibleLayer(ON_Layer::FromModelComponentRef(
                            model.LayerFromIndex(attr->m_layer_index), nullptr),
                        0))
        return;
      const auto *geometry = item->Geometry(nullptr);
      if (auto instance = ON_InstanceRef::Cast(geometry)) {
        const auto *def = ON_InstanceDefinition::FromModelComponentRef(
            model.ComponentFromId(ON_ModelComponent::Type::InstanceDefinition,
                                  instance->m_instance_definition_uuid),
            nullptr);
        if (!def)
          throw std::runtime_error("missing or external 3DM block definition");
        const auto &ids = def->InstanceGeometryIdList();
        if (ids.Count() == 0)
          throw std::runtime_error("empty or linked 3DM block");
        for (int i = 0; i < ids.Count(); ++i)
          visit(&model.ModelGeometryComponentFromId(ids[i]),
                world * instance->m_xform, depth + 1);
        return;
      }
      ON_Color color(196, 205, 214);
      if (attr) {
        auto candidate = model.WireframeColorFromAttributes(*attr);
        if (candidate != ON_Color::Black ||
            attr->ColorSource() == ON::color_from_object)
          color = candidate;
      }
      if (auto m = ON_Mesh::Cast(geometry))
        mesh(scene, *m, world, color);
      else if (auto brep = ON_Brep::Cast(geometry)) {
        if (brep->m_F.Count() > 100000)
          throw std::runtime_error("3DM face limit exceeded");
        for (int i = 0; i < brep->m_F.Count(); ++i) {
          const auto &face = brep->m_F[i];
          if (auto cached = face.Mesh(ON::render_mesh); cached && cached->FaceCount() > 0 && cached->VertexCount() >= 3) mesh(scene, *cached, world, color);
          else rhino_planar_face(scene, face, world, color);
        }
      } else if (auto extrusion = ON_Extrusion::Cast(geometry)) {
        auto m = extrusion->Mesh(ON::render_mesh);
        if (m && m->FaceCount() > 0 && m->VertexCount() >= 3) mesh(scene, *m, world, color);
        else rhino_extrusion(scene, *extrusion, world, color);
      } else if (auto surface = ON_Surface::Cast(geometry)) {
        if (!surface->IsPlanar())
          throw std::runtime_error("3DM uncached curved surfaces are not supported");
        std::unique_ptr<ON_Brep> brep(surface->BrepForm());
        if (!brep || brep->m_F.Count() > 100000)
          throw std::runtime_error("invalid 3DM planar surface");
        for (int i = 0; i < brep->m_F.Count(); ++i)
          rhino_planar_face(scene, brep->m_F[i], world, color);
      } else if (ON_SubD::Cast(geometry))
        throw std::runtime_error(
            "3DM surface/SubD has no supported saved mesh");
      // Curves, points, annotations, lights and cameras have no surface.
    };
    ONX_ModelComponentIterator it(model,
                                  ON_ModelComponent::Type::ModelGeometry);
    for (auto c = it.FirstComponent(); c; c = it.NextComponent()) {
      auto item = ON_ModelGeometryComponent::Cast(c);
      if (item && item->IsInstanceDefinitionGeometry())
        continue;
      visit(item, ON_Xform::IdentityTransformation, 0);
    }
    scene.emit(sink, context);
    return 0;
  } catch (const std::exception &e) {
    return failure(error, capacity, e.what());
  } catch (...) {
    return failure(error, capacity, "unknown 3DM reader failure");
  }
}
