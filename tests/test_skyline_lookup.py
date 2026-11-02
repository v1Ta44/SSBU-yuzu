"""Exercise Skyline symbol lookup precedence, including registered SDK host exports."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]

FIXTURE = r'''#include <stdint.h>
#include <stdio.h>
#include <string.h>
typedef struct { uint64_t x[32]; } Ctx;
typedef void (*ImportFn)(Ctx *);
typedef struct { uint64_t value; } Export;
static const struct { int defined; uint64_t value; } dyn_syms[] = {{1,0x1234},{0,0},{0,0}};
#define GUEST_BASE 0x7100000000ULL
#define IMPORT_BASE 0x7300000000ULL
#define IMPORT_SLOT 256ULL
#define RO_OK 0
static void wr64(uint64_t p,uint64_t v) { *(uint64_t *)(uintptr_t)p=v; }
static void handler(Ctx *c) {(void)c;}
static const Export *exp_find(const char *name) { static const Export e={0x7280123456ULL}; return !strcmp(name,"module_export") || !strcmp(name,"main_export") ? &e : NULL; }
static int main_find(const char *name) {return (!strcmp(name,"main_export") || !strcmp(name,"own_export"))?0:!strcmp(name,"sdk_import")?1:!strcmp(name,"unimplemented_import")?2:-1;}
static ImportFn import_lookup(const char *name) {return !strcmp(name,"sdk_import")?handler:NULL;}
static ImportFn extra_import_lookup(const char *name) {return !strcmp(name,"plugin_only_sdk")?handler:NULL;}
static int closures;
static uint64_t hostfn_address(const char *name,ImportFn f) {(void)name;(void)f;closures++;return 0x7400000800ULL;}
'''
CHECKS = r'''
static int check(const char *name,uint64_t expected) {
    uint64_t output=0xdeadbeef;Ctx c={0};c.x[0]=(uint64_t)(uintptr_t)&output;c.x[1]=(uint64_t)(uintptr_t)name;
    ro_LookupSymbol(&c);
    if(output!=expected || c.x[0]!=(expected?RO_OK:0x1416u)) {fprintf(stderr,"lookup failed: %s, address %llx, result %llx\n",name,(unsigned long long)output,(unsigned long long)c.x[0]);return 1;}
    return 0;
}
int main(void) {
    int bad=0;
    bad+=check("module_export",0x7280123456ULL);
    bad+=check("main_export",0x7280123456ULL); /* loaded modules retain precedence */
    bad+=check("own_export",GUEST_BASE+0x1234);
    bad+=check("sdk_import",IMPORT_BASE+IMPORT_SLOT);
    if(closures)bad++; /* main's existing hookable entry must be reused */
    bad+=check("plugin_only_sdk",0x7400000800ULL);
    if(closures!=1)bad++;
    bad+=check("unimplemented_import",0);
    bad+=check("missing",0);
    printf("Skyline lookup: loaded-module precedence, SDK import identity, plugin-only host export, missing-symbol failure: %s\n",bad?"FAILED":"ok");
    return bad?1:0;
}
'''

class SkylineLookupTests(unittest.TestCase):
    def test_sdk_host_exports_and_missing_symbols(self):
        compiler = os.environ.get("RECOMP_CLANG") or shutil.which("clang")
        if not compiler:
            self.skipTest("clang is unavailable")
        source = (ROOT / "runtime/skyline.c").read_text(encoding="utf-8")
        begin = source.index("static void ro_LookupSymbol(Ctx *c)")
        end = source.index("static void ro_LookupModuleSymbol", begin)
        with tempfile.TemporaryDirectory(prefix="skyline-lookup-") as directory:
            fixture = Path(directory) / "lookup.c"
            executable = Path(directory) / "lookup.exe"
            fixture.write_text(FIXTURE + source[begin:end] + CHECKS, encoding="utf-8")
            result = subprocess.run([compiler, "-O1", str(fixture), "-o", str(executable)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            result = subprocess.run([str(executable)], capture_output=True, text=True, timeout=10)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            print(result.stdout.strip())

if __name__ == "__main__":
    unittest.main()
