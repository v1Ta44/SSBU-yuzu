/* Actual nnfs/nnfsdir handlers, with only guest execution and unrelated host services stubbed.
 * All writable mounts are supplied by the isolated Python fixture. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <windows.h>
#include "runtime.h"
#include "hostrt.h"
#include "gfx_options.h"
#include "fs_host.h"
static ImportDef imports[160]; static int count, checks, failed;
int g_hitch_ms;
void hitch_note(const char *s) {(void)s;}
void r1_wrote(uint64_t a,uint64_t n){(void)a;(void)n;}
void r1_read_retry_note(void){}
void trace_printf(const char *fmt,...){(void)fmt;}
void register_imports(const ImportDef *defs,int n){
    for(int i=0;i<n;i++){int k=0;for(;k<count;k++)if(!strcmp(imports[k].name,defs[i].name))break;imports[k]=defs[i];if(k==count)count++;}
}
void *import_data(const char *name){static int dummy;return &dummy;}
uint64_t main_export(const char *name){return 0;}
void guest_trap(const char *what,uint64_t addr){fprintf(stderr,"unexpected guest call: %s\n",what);exit(90);}
void call_guest(Ctx *c,uint64_t addr){guest_trap("guest execution not expected in host filesystem test",addr);}
void call_indirect(Ctx *c,uint64_t addr){call_guest(c,addr);}
const GfxOptions *gfx_options(void){static GfxOptions o;return &o;}
static uint64_t op(Ctx*c,const char*name,uint64_t a,uint64_t b,uint64_t d,uint64_t e,uint64_t f){
    c->x[0]=a;c->x[1]=b;c->x[2]=d;c->x[3]=e;c->x[4]=f;
    for(int i=0;i<count;i++)if(!strcmp(imports[i].name,name)){imports[i].fn(c);return c->x[0];}
    fprintf(stderr,"missing handler %s\n",name);exit(91);
}
#define P(p) ((uint64_t)(uintptr_t)(p))
#define FS(n) "_ZN2nn2fs" n
#define CHECK(v,msg) do{checks++;if(!(v)){failed++;fprintf(stderr,"FAIL: %s\n",msg);}}while(0)
void register_fs_imports(void);void register_nnfsdir_imports(void);
uint32_t fs_mount_direct(const char*,const char*);
int nnfs_selftest(Ctx*);int nnfsdir_selftest(Ctx*);
static int read_is(Ctx*c,const char*path,const char*bytes,int n){
    uint64_t h=0,got=0;char buf[32]={0};
    if(op(c,FS("8OpenFileEPNS0_10FileHandleEPKci"),P(&h),P(path),1,0,0))return 0;
    uint64_t r=op(c,FS("8ReadFileEPmNS0_10FileHandleElPvm"),P(&got),h,0,P(buf),n);
    op(c,FS("9CloseFileENS0_10FileHandleE"),h,0,0,0,0);
    return !r&&got==(uint64_t)n&&!memcmp(buf,bytes,n);
}
static void write_bytes(Ctx*c,const char*path,const char*bytes,int n){
    uint64_t h=0;
    CHECK(!op(c,FS("10CreateFileEPKcl"),P(path),n,0,0,0),"create Unicode file");
    CHECK(!op(c,FS("8OpenFileEPNS0_10FileHandleEPKci"),P(&h),P(path),3,0,0),"open Unicode file for write");
    uint64_t options=1;
    CHECK(!op(c,FS("9WriteFileENS0_10FileHandleElPKvmRKNS0_11WriteOptionE"),h,0,P(bytes),n,P(&options)),"write Unicode file");
    op(c,FS("9CloseFileENS0_10FileHandleE"),h,0,0,0,0);
}
int main(void){
    char root[MAX_PATH],user[MAX_PATH];
    if(fsu_env(L"SSBU_FS_TEST_ROOT",root,sizeof root)!=1||fsu_env(L"SSBU_USERDATA",user,sizeof user)!=1)return 92;
    Ctx *c=calloc(1,sizeof *c);register_fs_imports();register_nnfsdir_imports();
    CHECK(!nnfs_selftest(c),"upstream mount table and journal recovery");
    CHECK(!nnfsdir_selftest(c),"upstream directory modes and consumed iterator");
    CHECK(!fs_mount_direct("sd",root),"mount Unicode SD root");
    const char *names[]={"English",u8"中文包",u8"日本語_🎮"};
    const char payload[]={0,1,2,(char)0xff,'X','Y'};
    for(int i=0;i<3;i++){
        char dir[MAX_PATH],sub[MAX_PATH],file[MAX_PATH],renamed[MAX_PATH];
        snprintf(dir,sizeof dir,"sd:/%s",names[i]);
        snprintf(sub,sizeof sub,"%s/%s",dir,u8"子目录");
        snprintf(file,sizeof file,"%s/%s",sub,u8"原文件.bin");
        CHECK(!op(c,FS("15CreateDirectoryEPKc"),P(dir),0,0,0,0),"create package directory");
        CHECK(!op(c,FS("15CreateDirectoryEPKc"),P(sub),0,0,0,0),"create nested Unicode directory");
        int type=-1;
        CHECK(!op(c,FS("12GetEntryTypeEPNS0_18DirectoryEntryTypeEPKc"),P(&type),P(sub),0,0,0)&&type==0,"nested directory exists");
        write_bytes(c,file,payload,sizeof payload);
        CHECK(read_is(c,file,payload,sizeof payload),"read original bytes through SDK handle");
        uint64_t h=0;int64_t n=-1,got=-1;unsigned char entries[0x310*2]={0};
        CHECK(!op(c,FS("13OpenDirectoryEPNS0_15DirectoryHandleEPKci"),P(&h),P(dir),3,0,0),"enter Unicode package");
        CHECK(!op(c,FS("22GetDirectoryEntryCountEPlNS0_15DirectoryHandleE"),P(&n),h,0,0,0)&&n==1,"count Unicode child");
        CHECK(!op(c,FS("13ReadDirectoryEPlPNS0_14DirectoryEntryENS0_15DirectoryHandleEl"),P(&got),P(entries),h,2,0)&&got==1&&!strcmp((char*)entries,u8"子目录"),"return exact UTF-8 name");
        op(c,FS("14CloseDirectoryENS0_15DirectoryHandleE"),h,0,0,0,0);
        snprintf(renamed,sizeof renamed,"%s/%s",sub,u8"改名.bin");
        CHECK(!op(c,FS("10RenameFileEPKcS2_"),P(file),P(renamed),0,0,0),"rename Unicode file");
        CHECK(read_is(c,renamed,payload,sizeof payload),"read renamed bytes");
        CHECK(!op(c,FS("10DeleteFileEPKc"),P(renamed),0,0,0,0),"delete only fixture file");
        CHECK(!op(c,FS("15DeleteDirectoryEPKc"),P(sub),0,0,0,0),"delete empty Unicode fixture folder");
        snprintf(renamed,sizeof renamed,"sd:/%s",i==0?u8"改名英文":i==1?u8"中文改名":u8"日本語_改名🎮");
        CHECK(!op(c,FS("15RenameDirectoryEPKcS2_"),P(dir),P(renamed),0,0,0),"rename Unicode directory");
        CHECK(!op(c,FS("15DeleteDirectoryEPKc"),P(renamed),0,0,0,0),"remove renamed empty fixture folder");
    }
    uint64_t uid[2]={1,0};
    CHECK(!op(c,FS("14EnsureSaveDataERKNS_7account3UidE"),P(uid),0,0,0,0),"create save under Unicode userdata root");
    CHECK(!op(c,FS("13MountSaveDataEPKcRKNS_7account3UidE"),P("utf8save"),P(uid),0,0,0),"mount journal under Unicode root");
    const char *save=u8"utf8save:/中文记录.bin";
    write_bytes(c,save,payload,sizeof payload);
    char host[MAX_PATH];snprintf(host,sizeof host,"%s/save/%s",user,u8"中文记录.bin");
    CHECK(fsu_attributes(host)==INVALID_FILE_ATTRIBUTES,"uncommitted Unicode write is private");
    CHECK(!op(c,FS("6CommitEPKc"),P("utf8save"),0,0,0,0),"commit Unicode journal");
    CHECK(fsu_attributes(host)!=INVALID_FILE_ATTRIBUTES,"commit creates exact Unicode filename");
    CHECK(!op(c,FS("10DeleteFileEPKc"),P(save),0,0,0,0),"journal deletion");
    op(c,FS("7UnmountEPKc"),P("utf8save"),0,0,0,0);
    CHECK(fsu_attributes(host)!=INVALID_FILE_ATTRIBUTES,"unmount rolls back uncommitted deletion");
    CHECK(!op(c,FS("13MountSaveDataEPKcRKNS_7account3UidE"),P("utf8save"),P(uid),0,0,0),"remount Unicode save");
    CHECK(read_is(c,save,payload,sizeof payload),"Unicode committed bytes survive remount");
    CHECK(!op(c,FS("10DeleteFileEPKc"),P(save),0,0,0,0),"delete committed Unicode fixture file");
    CHECK(!op(c,FS("6CommitEPKc"),P("utf8save"),0,0,0,0),"commit Unicode deletion");
    CHECK(fsu_attributes(host)==INVALID_FILE_ATTRIBUTES,"committed deletion applied");
    op(c,FS("7UnmountEPKc"),P("utf8save"),0,0,0,0);
    wchar_t invalid[MAX_PATH];CHECK(!fsu_wide("invalid\xff",invalid,MAX_PATH),"reject invalid UTF-8");
    printf("UTF-8 filesystem: %d checks, %d failed\n",checks,failed);free(c);return failed!=0;
}
