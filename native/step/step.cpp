// MeshThumbs STEP tessellator. MIT; Open CASCADE remains LGPL-2.1 with its
// exception.
#include <BRepBndLib.hxx>
#include <BRepLib_ToolTriangulatedShape.hxx>
#include <BRepMesh_IncrementalMesh.hxx>
#include <BRep_Tool.hxx>
#include <Bnd_Box.hxx>
#include <IFSelect_ReturnStatus.hxx>
#include <IGESControl_Reader.hxx>
#include <Message.hxx>
#include <Message_Messenger.hxx>
#include <Poly_Triangulation.hxx>
#include <STEPControl_Reader.hxx>
#include <Standard_Failure.hxx>
#include <TopExp_Explorer.hxx>
#include <TopoDS.hxx>
#include <TopoDS_Face.hxx>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <mutex>
#include <stdexcept>

#ifdef _WIN32
#define EXPORT extern "C" __declspec(dllexport)
#else
#define EXPORT extern "C" __attribute__((visibility("default")))
#endif

// Six doubles per vertex, three vertices per triangle. Nothing allocated by
// OCCT crosses the DLL boundary. Returning zero from the callback cancels.
using TriangleSink = int (*)(void *, const double *);
static std::mutex readerMutex;

EXPORT std::uint32_t meshthumbs_step_abi() { return 1; }

static int load_cad(const std::uint16_t *path, std::uint64_t triangleLimit,
                    TriangleSink sink, void *context, char *error,
                    std::uint32_t errorCapacity, bool iges) {
  try {
    if (!path || !sink || !triangleLimit)
      throw std::runtime_error("invalid STEP render request");
    // STEP's legacy translator has process-wide settings. Serialize public
    // renderer calls; Explorer already runs each request in a separate process.
    std::lock_guard<std::mutex> guard(readerMutex);
    std::u16string inputPath(reinterpret_cast<const char16_t *>(path));
    std::ifstream input(std::filesystem::path(inputPath), std::ios::binary);
    if (!input)
      throw std::runtime_error("cannot open STEP file");
    input.seekg(0, std::ios::end);
    auto bytes = input.tellg();
    if (bytes <= 0 || bytes > 300LL * 1024 * 1024)
      throw std::runtime_error("STEP file size is invalid");
    input.seekg(0);
    // Keep parser chatter out of the CLI/Explorer worker transport.
    Message::DefaultMessenger()->ChangePrinters().Clear();
    TopoDS_Shape shape;
    if (iges) {
      IGESControl_Reader reader;
      const auto utf8 = std::filesystem::path(inputPath).u8string();
      if (reader.ReadFile(utf8.c_str()) != IFSelect_RetDone)
        throw std::runtime_error("invalid or unsupported IGES document");
      if (reader.NbRootsForTransfer() <= 0 ||
          reader.NbRootsForTransfer() > 100000)
        throw std::runtime_error(
            "IGES root count is outside the supported limit");
      if (!reader.TransferRoots())
        throw std::runtime_error("IGES contains no transferable geometry");
      shape = reader.OneShape();
    } else {
      STEPControl_Reader reader;
      if (reader.ReadStream("meshthumbs.step", input) != IFSelect_RetDone)
        throw std::runtime_error("invalid or unsupported STEP document");
      if (reader.NbRootsForTransfer() <= 0 ||
          reader.NbRootsForTransfer() > 100000)
        throw std::runtime_error(
            "STEP root count is outside the supported limit");
      if (!reader.TransferRoots())
        throw std::runtime_error("STEP contains no transferable geometry");
      shape = reader.OneShape();
    }
    if (shape.IsNull())
      throw std::runtime_error("STEP contains no shapes");

    Bnd_Box bounds;
    BRepBndLib::Add(shape, bounds, false);
    if (bounds.IsVoid() || bounds.IsOpen())
      throw std::runtime_error("STEP has invalid bounds");
    double xmin, ymin, zmin, xmax, ymax, zmax;
    bounds.Get(xmin, ymin, zmin, xmax, ymax, zmax);
    const double cx = xmin / 2 + xmax / 2, cy = ymin / 2 + ymax / 2,
                 cz = zmin / 2 + zmax / 2;
    const double span = std::max({xmax - xmin, ymax - ymin, zmax - zmin});
    if (!std::isfinite(span) || span <= 0 || !std::isfinite(cx + cy + cz))
      throw std::runtime_error("STEP has non-finite or degenerate bounds");
    // A 0.1% model-span chord tolerance is sufficient at thumbnail sizes.
    // Meshing stays single-threaded to avoid unbounded work across Explorer
    // requests.
    BRepMesh_IncrementalMesh mesh(shape, std::max(span * 0.001, 1e-7), false,
                                  0.35, false);
    if (!mesh.IsDone())
      throw std::runtime_error("STEP tessellation failed");

    std::uint64_t total = 0, faceCount = 0;
    for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
      if (++faceCount > 100000)
        throw std::runtime_error("STEP face count exceeds 100000");
      TopLoc_Location location;
      auto triangulation =
          BRep_Tool::Triangulation(TopoDS::Face(it.Current()), location);
      if (triangulation.IsNull() || triangulation->NbTriangles() <= 0)
        throw std::runtime_error(
            "STEP has a face that could not be tessellated");
      total += static_cast<std::uint64_t>(triangulation->NbTriangles());
      if (total > triangleLimit)
        throw std::runtime_error("STEP exceeds the triangle limit");
    }
    if (!total)
      throw std::runtime_error("STEP contains no renderable faces");

    for (TopExp_Explorer it(shape, TopAbs_FACE); it.More(); it.Next()) {
      const TopoDS_Face face = TopoDS::Face(it.Current());
      TopLoc_Location location;
      auto triangulation = BRep_Tool::Triangulation(face, location);
      BRepLib_ToolTriangulatedShape::ComputeNormals(face, triangulation);
      const gp_Trsf transform = location.Transformation();
      const bool reversed = face.Orientation() == TopAbs_REVERSED;
      for (int i = 1; i <= triangulation->NbTriangles(); ++i) {
        int indices[3];
        triangulation->Triangle(i).Get(indices[0], indices[1], indices[2]);
        if (reversed != transform.IsNegative())
          std::swap(indices[1], indices[2]);
        double vertices[18];
        for (int v = 0; v < 3; ++v) {
          auto point = triangulation->Node(indices[v]).Transformed(transform);
          auto normal =
              triangulation->Normal(indices[v]).Transformed(transform);
          if (reversed)
            normal.Reverse();
          // Recenter in double precision before crossing into the f32 renderer.
          // CAD convention is Z-up; renderer convention is Y-up.
          double values[] = {(point.X() - cx) / span,
                             (point.Z() - cz) / span,
                             -(point.Y() - cy) / span,
                             normal.X(),
                             normal.Z(),
                             -normal.Y()};
          for (int j = 0; j < 6; ++j) {
            if (!std::isfinite(values[j]))
              throw std::runtime_error("STEP has non-finite mesh data");
            vertices[v * 6 + j] = values[j];
          }
        }
        if (!sink(context, vertices))
          throw std::runtime_error("STEP triangle transfer cancelled");
      }
    }
    return 0;
  } catch (const Standard_Failure &failure) {
    if (error && errorCapacity) {
      const char *message = failure.GetMessageString();
      std::strncpy(error, message ? message : "Open CASCADE failure",
                   errorCapacity - 1);
      error[errorCapacity - 1] = '\0';
    }
  } catch (const std::exception &failure) {
    if (error && errorCapacity) {
      std::strncpy(error, failure.what(), errorCapacity - 1);
      error[errorCapacity - 1] = '\0';
    }
  } catch (...) {
    if (error && errorCapacity) {
      std::strncpy(error, "unknown STEP reader failure", errorCapacity - 1);
      error[errorCapacity - 1] = '\0';
    }
  }
  return 1;
}

EXPORT int meshthumbs_step_load(const std::uint16_t *path, std::uint64_t limit,
                                TriangleSink sink, void *context, char *error,
                                std::uint32_t capacity) {
  return load_cad(path, limit, sink, context, error, capacity, false);
}
EXPORT int meshthumbs_iges_load(const std::uint16_t *path, std::uint64_t limit,
                                TriangleSink sink, void *context, char *error,
                                std::uint32_t capacity) {
  return load_cad(path, limit, sink, context, error, capacity, true);
}
