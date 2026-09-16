#include "scene.h"
#include <Alembic/AbcCoreOgawa/All.h>
#include <Alembic/AbcGeom/All.h>
#include <fstream>
#include <functional>
#include <set>
namespace A = Alembic::Abc;
namespace G = Alembic::AbcGeom;

static void polygons(Scene &out, const A::P3fArraySamplePtr &positions,
                     const A::Int32ArraySamplePtr &counts,
                     const A::Int32ArraySamplePtr &indices,
                     const A::M44d &matrix, const G::IN3fGeomParam &normals,
                     const A::ISampleSelector &sample,
                     const A::Int32ArraySamplePtr &holes = {}) {
  if (!positions || !counts || !indices)
    throw std::runtime_error("incomplete Alembic mesh");
  if (counts->size() > 5000000 || positions->size() > 15000000 ||
      indices->size() > 20000000)
    throw std::runtime_error("Alembic mesh array limit exceeded");
  std::set<int> omitted;
  if (holes)
    for (size_t hi = 0; hi < holes->size(); ++hi) {
      auto h = (*holes)[hi];
      if (h < 0 || static_cast<size_t>(h) >= counts->size())
        throw std::runtime_error("bad SubD hole index");
      omitted.insert(h);
    }
  A::N3fArraySamplePtr ns;
  G::GeometryScope scope = G::kUnknownScope;
  if (normals.valid()) {
    ns = normals.getExpandedValue(sample).getVals();
    scope = normals.getScope();
  }
  const auto normalMatrix = matrix.inverse().transposed();
  const bool mirror = matrix.determinant() < 0;
  size_t corner = 0;
  for (size_t f = 0; f < counts->size(); ++f) {
    int count = (*counts)[f];
    if (count < 3 || count > 4096 || corner + count > indices->size())
      throw std::runtime_error("invalid Alembic face size");
    Face face;
    face.reserve(count);
    for (int c = 0; c < count; ++c) {
      int index = (*indices)[corner + c];
      if (index < 0 || static_cast<size_t>(index) >= positions->size())
        throw std::runtime_error("Alembic vertex index out of range");
      A::V3d p((*positions)[index]), world, n(0);
      matrix.multVecMatrix(p, world);
      if (ns && ns->size()) {
        size_t i;
        switch (scope) {
        case G::kConstantScope:
          i = 0;
          break;
        case G::kUniformScope:
          i = f;
          break;
        case G::kVertexScope:
        case G::kVaryingScope:
          i = index;
          break;
        case G::kFacevaryingScope:
          i = corner + c;
          break;
        default:
          throw std::runtime_error("unsupported Alembic normal interpolation");
        }
        if (i >= ns->size())
          throw std::runtime_error("Alembic normal index out of range");
        normalMatrix.multDirMatrix(A::V3d((*ns)[i]), n);
        if (n.length2() > 0)
          n.normalize();
      }
      face.push_back({world.x, world.y, world.z, n.x, n.y, n.z, 196. / 255,
                      205. / 255, 214. / 255, 1});
    }
    corner += count;
    if (omitted.count(static_cast<int>(f)))
      continue;
    // Alembic uses RenderMan's clockwise winding; the renderer uses CCW.
    if (!mirror)
      std::reverse(face.begin(), face.end());
    out.add(std::move(face));
  }
  if (corner != indices->size())
    throw std::runtime_error("unused Alembic face indices");
}

EXPORT std::uint32_t meshthumbs_scene_abi() { return 1; }
EXPORT int meshthumbs_abc_load(const std::uint16_t *path, std::uint64_t budget,
                               PolygonSink sink, void *context, char *error,
                               std::uint32_t capacity) {
  try {
    if (!sink || !budget)
      throw std::runtime_error("invalid Alembic request");
    std::ifstream stream(input_path(path), std::ios::binary);
    char magic[5] = {};
    stream.read(magic, 5);
    stream.seekg(0);
    if (std::memcmp(magic, "Ogawa", 5) != 0)
      throw std::runtime_error(
          "only Alembic Ogawa archives are supported (not HDF5)");
    Alembic::AbcCoreOgawa::ReadArchive reader(
        std::vector<std::istream *>{&stream});
    A::IArchive archive(reader("meshthumbs.abc"),
                        A::ErrorHandler::kThrowPolicy);
    if (!archive.valid())
      throw std::runtime_error("invalid Alembic archive");
    double time = INFINITY;
    if (archive.getNumTimeSamplings() > 100000)
      throw std::runtime_error("Alembic time-sampling limit exceeded");
    for (std::uint32_t i = 1; i < archive.getNumTimeSamplings(); ++i)
      time = std::min(time, archive.getTimeSampling(i)->getSampleTime(0));
    if (!std::isfinite(time))
      time = 0;
    A::ISampleSelector sample(time, A::ISampleSelector::kFloorIndex);
    Scene scene(budget);
    size_t visited = 0;
    std::function<void(A::IObject, const A::M44d &, int)> visit;
    visit = [&](A::IObject object, const A::M44d &parent, int depth) {
      if (++visited > 100000 || depth > 64)
        throw std::runtime_error("Alembic hierarchy limit exceeded");
      if (G::GetVisibility(object, sample) == G::kVisibilityHidden)
        return;
      A::M44d world = parent;
      if (G::IXform::matches(object.getHeader())) {
        auto value =
            G::IXform(object, A::kWrapExisting).getSchema().getValue(sample);
        world = value.getInheritsXforms() ? value.getMatrix() * parent
                                          : value.getMatrix();
      }
      if (!std::isfinite(world.determinant()) || world.determinant() == 0)
        throw std::runtime_error("singular Alembic transform");
      if (G::IPolyMesh::matches(object.getHeader())) {
        auto schema = G::IPolyMesh(object, A::kWrapExisting).getSchema();
        auto value = schema.getValue(sample);
        polygons(scene, value.getPositions(), value.getFaceCounts(),
                 value.getFaceIndices(), world, schema.getNormalsParam(),
                 sample);
      } else if (G::ISubD::matches(object.getHeader())) {
        auto value =
            G::ISubD(object, A::kWrapExisting).getSchema().getValue(sample);
        polygons(scene, value.getPositions(), value.getFaceCounts(),
                 value.getFaceIndices(), world, G::IN3fGeomParam(), sample,
                 value.getHoles());
      }
      if (object.getNumChildren() > 100000)
        throw std::runtime_error("Alembic child count exceeded");
      for (size_t i = 0; i < object.getNumChildren(); ++i)
        visit(object.getChild(i), world, depth + 1);
    };
    visit(archive.getTop(), A::M44d(), 0);
    scene.emit(sink, context);
    return 0;
  } catch (const std::exception &e) {
    return failure(error, capacity, e.what());
  } catch (...) {
    return failure(error, capacity, "unknown Alembic reader failure");
  }
}
