/* Host paths in nn::fs / Skyline are UTF-8. Convert only at the Windows API
 * boundary; an ANSI codepage cannot round-trip the names returned to plugins.
 * Save journaling and guest FSA dispatch are deliberately outside this layer. */
#ifndef SSBU_FS_HOST_H
#define SSBU_FS_HOST_H
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
#ifdef _WIN32
static int fsu_wide(const char *path, wchar_t *out, int cap) {
    if (!path) { SetLastError(ERROR_INVALID_PARAMETER); return 0; }
    return MultiByteToWideChar(CP_UTF8, MB_ERR_INVALID_CHARS, path, -1, out, cap) != 0;
}
static int fsu_utf8(const wchar_t *path, char *out, int cap) {
    return WideCharToMultiByte(CP_UTF8, WC_ERR_INVALID_CHARS, path, -1, out, cap, NULL, NULL) != 0;
}
static int fsu_env(const wchar_t *name, char *out, int cap) {
    const wchar_t *v = _wgetenv(name);
    if (!v || !*v) { out[0] = 0; return 0; }
    if (!fsu_utf8(v, out, cap)) { out[0] = 0; return -1; }
    return 1;
}
static DWORD fsu_attributes(const char *path) {
    wchar_t w[MAX_PATH * 3];
    return fsu_wide(path, w, MAX_PATH * 3) ? GetFileAttributesW(w) : INVALID_FILE_ATTRIBUTES;
}
static BOOL fsu_mkdir(const char *path, LPSECURITY_ATTRIBUTES security) {
    wchar_t w[MAX_PATH * 3];
    return fsu_wide(path, w, MAX_PATH * 3) && CreateDirectoryW(w, security);
}
static BOOL fsu_rmdir(const char *path) {
    wchar_t w[MAX_PATH * 3];
    return fsu_wide(path, w, MAX_PATH * 3) && RemoveDirectoryW(w);
}
static BOOL fsu_delete(const char *path) {
    wchar_t w[MAX_PATH * 3];
    return fsu_wide(path, w, MAX_PATH * 3) && DeleteFileW(w);
}
static BOOL fsu_set_attributes(const char *path, DWORD flags) {
    wchar_t w[MAX_PATH * 3];
    return fsu_wide(path, w, MAX_PATH * 3) && SetFileAttributesW(w, flags);
}
static HANDLE fsu_open(const char *path, DWORD access, DWORD share, LPSECURITY_ATTRIBUTES security,
                       DWORD disposition, DWORD flags, HANDLE template_file) {
    wchar_t w[MAX_PATH * 3];
    return fsu_wide(path, w, MAX_PATH * 3) ? CreateFileW(w, access, share, security, disposition, flags, template_file) : INVALID_HANDLE_VALUE;
}
static BOOL fsu_copy(const char *from, const char *to, BOOL fail_if_exists) {
    wchar_t a[MAX_PATH * 3], b[MAX_PATH * 3];
    return fsu_wide(from, a, MAX_PATH * 3) && fsu_wide(to, b, MAX_PATH * 3) && CopyFileW(a, b, fail_if_exists);
}
static BOOL fsu_move(const char *from, const char *to, DWORD flags) {
    wchar_t a[MAX_PATH * 3], b[MAX_PATH * 3];
    return fsu_wide(from, a, MAX_PATH * 3) && fsu_wide(to, b, MAX_PATH * 3) && MoveFileExW(a, b, flags);
}
static FILE *fsu_fopen(const char *path, const char *mode) {
    wchar_t w[MAX_PATH * 3], m[32];
    if (!fsu_wide(path, w, MAX_PATH * 3) || !fsu_wide(mode, m, 32)) return NULL;
    return _wfopen(w, m);
}
typedef struct { DWORD dwFileAttributes, nFileSizeHigh, nFileSizeLow; char cFileName[MAX_PATH * 3]; } FsuFindData;
static int fsu_find_data(const WIN32_FIND_DATAW *w, FsuFindData *out) {
    out->dwFileAttributes = w->dwFileAttributes;
    out->nFileSizeHigh = w->nFileSizeHigh; out->nFileSizeLow = w->nFileSizeLow;
    return fsu_utf8(w->cFileName, out->cFileName, sizeof out->cFileName);
}
static BOOL fsu_find_next(HANDLE h, FsuFindData *out) {
    WIN32_FIND_DATAW w;
    while (FindNextFileW(h, &w)) if (fsu_find_data(&w, out)) return TRUE;
    return FALSE;
}
static HANDLE fsu_find_first(const char *pattern, FsuFindData *out) {
    wchar_t p[MAX_PATH * 3]; WIN32_FIND_DATAW w;
    if (!fsu_wide(pattern, p, MAX_PATH * 3)) return INVALID_HANDLE_VALUE;
    HANDLE h = FindFirstFileW(p, &w);
    if (h != INVALID_HANDLE_VALUE && !fsu_find_data(&w, out) && !fsu_find_next(h, out)) { FindClose(h); return INVALID_HANDLE_VALUE; }
    return h;
}
static DWORD fsu_temp(DWORD cap, char *out) {
    wchar_t w[MAX_PATH + 1];
    DWORD n = GetTempPathW(MAX_PATH + 1, w);
    if (!n || n > MAX_PATH || !fsu_utf8(w, out, (int)cap)) return 0;
    return (DWORD)strlen(out);
}
static DWORD fsu_module_path(HMODULE module, char *out, DWORD cap) {
    wchar_t w[MAX_PATH + 1];
    DWORD n = GetModuleFileNameW(module, w, MAX_PATH + 1);
    if (!n || n >= MAX_PATH + 1 || !fsu_utf8(w, out, (int)cap)) { if (cap) out[0] = 0; return 0; }
    return (DWORD)strlen(out);
}
#else
/* POSIX paths already use UTF-8; keep the existing platform adapter. */
#define fsu_attributes GetFileAttributesA
#define fsu_mkdir CreateDirectoryA
#define fsu_rmdir RemoveDirectoryA
#define fsu_delete DeleteFileA
#define fsu_set_attributes SetFileAttributesA
#define fsu_open CreateFileA
#define fsu_copy CopyFileA
#define fsu_move MoveFileExA
#define fsu_fopen fopen
#define FsuFindData WIN32_FIND_DATAA
#define fsu_find_next FindNextFileA
#define fsu_find_first FindFirstFileA
#define fsu_temp GetTempPathA
#define fsu_module_path GetModuleFileNameA
#endif
#endif
