#pragma once
// The public openNURBS headers assume Microsoft's Windows build environment.
// Supply its Unicode SDK declarations and the POSIX directory helper on MinGW.
#ifndef UNICODE
#define UNICODE
#endif
#ifndef _UNICODE
#define _UNICODE
#endif
#include <cstdarg>
#include <cstdio>
#include <ctime>
#include <dirent.h>
#include <locale.h>
#include <rpc.h>
// Windows SDK declarations must precede known-folder GUID declarations.
// clang-format off
#include <windows.h>
#include <shlobj.h>
#include <knownfolders.h>
// clang-format on
inline tm *localtime_r(const time_t *input, tm *result) {
  return localtime_s(result, input) == 0 ? result : nullptr;
}
inline int vsnprintf_l(char *out, size_t size, _locale_t locale,
                       const char *format, va_list args) {
  return _vsnprintf_l(out, size, format, locale, args);
}
// MSVCRT's MinGW import library has no secure va_list scan entry points.
// Use the Windows UCRT secure scanners, with a locale allocated by that same
// CRT (never pass an MSVCRT locale across the CRT boundary).
struct OpenNurbsUcrt {
  HMODULE module =
      LoadLibraryExW(L"ucrtbase.dll", nullptr, LOAD_LIBRARY_SEARCH_SYSTEM32);
  using Create = void *(__cdecl *)(int, const char *);
  using Free = void(__cdecl *)(void *);
  void *locale = module ? reinterpret_cast<Create>(GetProcAddress(
                              module, "_create_locale"))(LC_ALL, "C")
                        : nullptr;
  ~OpenNurbsUcrt() {
    if (locale)
      reinterpret_cast<Free>(GetProcAddress(module, "_free_locale"))(locale);
    if (module)
      FreeLibrary(module);
  }
};
inline int _vsscanf_s_l(const char *input, const char *format, _locale_t,
                        va_list args) {
  static OpenNurbsUcrt crt;
  using Scan = int(__cdecl *)(unsigned long long, const char *, size_t,
                              const char *, void *, va_list);
  auto scan = crt.module ? reinterpret_cast<Scan>(GetProcAddress(
                               crt.module, "__stdio_common_vsscanf"))
                         : nullptr;
  return scan && crt.locale
             ? scan(1, input, static_cast<size_t>(-1), format, crt.locale, args)
             : -1;
}
inline int _vswscanf_s_l(const wchar_t *input, const wchar_t *format, _locale_t,
                         va_list args) {
  static OpenNurbsUcrt crt;
  using Scan = int(__cdecl *)(unsigned long long, const wchar_t *, size_t,
                              const wchar_t *, void *, va_list);
  auto scan = crt.module ? reinterpret_cast<Scan>(GetProcAddress(
                               crt.module, "__stdio_common_vswscanf"))
                         : nullptr;
  return scan && crt.locale
             ? scan(1, input, static_cast<size_t>(-1), format, crt.locale, args)
             : -1;
}
inline void qsort_r(void *base, size_t count, size_t width, void *context,
                    int (*compare)(void *, const void *, const void *)) {
  qsort_s(base, count, width, compare, context);
}
template <class... Args>
int sprintf_l(char *out, _locale_t locale, const char *format, Args... args) {
  return _sprintf_l(out, format, locale, args...);
}
template <class... Args>
int snprintf_l(char *out, size_t size, _locale_t locale, const char *format,
               Args... args) {
  return _snprintf_l(out, size, format, locale, args...);
}
template <class... Args>
int sscanf_l(const char *input, _locale_t locale, const char *format,
             Args... args) {
  return _sscanf_l(input, format, locale, args...);
}
#ifndef NAME_MAX
#define NAME_MAX 255
#endif
inline int readdir_r(DIR *directory, struct dirent *entry,
                     struct dirent **result) {
  auto current = readdir(directory);
  *result = current ? entry : nullptr;
  if (current)
    *entry = *current;
  return 0;
}
