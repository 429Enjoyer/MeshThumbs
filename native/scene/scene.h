#pragma once
#include <algorithm>
#include <array>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <filesystem>
#include <stdexcept>
#include <utility>
#include <vector>
#define EXPORT extern "C" __declspec(dllexport)
using PolygonSink = int (*)(void *, const double *, std::uint32_t);
using Vertex = std::array<double, 10>; // position, normal, RGBA
using Face = std::vector<Vertex>;
struct Scene {
  std::vector<Face> faces;
  std::uint64_t triangles = 0, bytes = 0, limit;
  explicit Scene(std::uint64_t budget) : limit(budget) {}
  void add(Face face) {
    if (face.size() < 3 || face.size() > 4096)
      throw std::runtime_error("unsupported polygon size (3..4096)");
    triangles += face.size() - 2;
    bytes += face.size() * sizeof(Vertex) + sizeof(Face);
    if (triangles > limit)
      throw std::runtime_error("scene exceeds triangle limit");
    if (bytes > 300ULL * 1024 * 1024)
      throw std::runtime_error("expanded scene exceeds 300 MiB");
    for (const auto &v : face)
      for (auto x : v)
        if (!std::isfinite(x))
          throw std::runtime_error("non-finite scene data");
    faces.push_back(std::move(face));
  }
  void emit(PolygonSink sink, void *context) {
    if (faces.empty())
      throw std::runtime_error("file has no supported visible mesh geometry");
    double lo[3] = {INFINITY, INFINITY, INFINITY},
           hi[3] = {-INFINITY, -INFINITY, -INFINITY};
    for (auto &f : faces)
      for (auto &v : f)
        for (int i = 0; i < 3; ++i) {
          lo[i] = std::min(lo[i], v[i]);
          hi[i] = std::max(hi[i], v[i]);
        }
    double span = std::max({hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]});
    if (!std::isfinite(span) || span <= 0)
      throw std::runtime_error("degenerate scene bounds");
    for (auto &f : faces) {
      for (auto &v : f)
        for (int i = 0; i < 3; ++i)
          v[i] = (v[i] - (hi[i] / 2 + lo[i] / 2)) / span;
      if (!sink(context, f.front().data(),
                static_cast<std::uint32_t>(f.size())))
        throw std::runtime_error("polygon transfer cancelled");
    }
  }
};
inline std::filesystem::path input_path(const std::uint16_t *input) {
  if (!input)
    throw std::runtime_error("missing model path");
  std::u16string path;
  for (; *input; ++input)
    path.push_back(static_cast<char16_t>(*input));
  auto result = std::filesystem::path(path);
  auto size = std::filesystem::file_size(result);
  if (size == 0 || size > 300ULL * 1024 * 1024)
    throw std::runtime_error("model size is outside 1..300 MiB");
  return result;
}
inline int failure(char *error, std::uint32_t capacity, const char *message) {
  if (error && capacity) {
    std::strncpy(error, message, capacity - 1);
    error[capacity - 1] = 0;
  }
  return 1;
}
