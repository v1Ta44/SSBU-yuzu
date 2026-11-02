/* Real named-pipe bridge, protocol selector and SDK adapters; no game/GPU. */
#define NETWORK_PROTOCOL_NO_MAIN
#include "network_protocol.c"
int console_bridge_start(void);
int console_bridge_request_restart(void);
static void request(HANDLE pipe,const char *s,char *out,DWORD capacity) {
    DWORD n; CHECK(WriteFile(pipe,s,(DWORD)strlen(s),&n,NULL));
    DWORD used=0;
    do { CHECK(ReadFile(pipe,out+used,capacity-used-1,&n,NULL));used+=n;CHECK(used<capacity-1); }while(!memchr(out,'\n',used));
    out[used]=0;
}
int main(int argc,char **argv) {
    CHECK(extra_import_lookup("missing_plugin_only_sdk")==NULL);
    if(argc>1&&!strcmp(argv[1],"--child")) {
        CHECK(console_bridge_start());
        CHECK(console_bridge_request_restart());
        Sleep(8000);return 7;
    }
    char self[MAX_PATH];CHECK(GetModuleFileNameA(NULL,self,sizeof self));
    const char *modes[]={"yuzu","lobby","off"};const char *names[]={"yuzu","upstream","off"};
    for(int i=0;i<3;i++)for(int round=0;round<2;round++) {
        char name[128];snprintf(name,sizeof name,"\\\\.\\pipe\\ssbu-bridge-test-%lu-%d-%d",GetCurrentProcessId(),i,round);
        HANDLE pipe=CreateNamedPipeA(name,PIPE_ACCESS_DUPLEX,PIPE_TYPE_BYTE|PIPE_WAIT,1,16384,16384,0,NULL);CHECK(pipe!=INVALID_HANDLE_VALUE);
        CHECK(SetEnvironmentVariableA("SSBU_CONSOLE_PIPE",name));CHECK(SetEnvironmentVariableA("SSBU_ONLINE",modes[i]));
        char command[MAX_PATH+32];snprintf(command,sizeof command,"\"%s\" --child",self);
        STARTUPINFOA si={sizeof si};PROCESS_INFORMATION child;
        CHECK(CreateProcessA(NULL,command,NULL,NULL,FALSE,CREATE_NO_WINDOW,NULL,NULL,&si,&child));CloseHandle(child.hThread);
        CHECK(ConnectNamedPipe(pipe,NULL)||GetLastError()==ERROR_PIPE_CONNECTED);
        char response[32768],expected[64];snprintf(expected,sizeof expected,"\"connection\":\"%s\"",names[i]);
        request(pipe,"poll\n",response,sizeof response);CHECK(strstr(response,expected)&&strstr(response,"\"started\":false"));
        if(i!=0) {request(pipe,"join\t3132372e302e302e31\t24872\t506c61796572\t\t\n",response,sizeof response);CHECK(strstr(response,"\"code\":2"));}
        request(pipe,"start\n",response,sizeof response);CHECK(strstr(response,"\"code\":0"));
        int signalled=0;
        for(int k=0;k<100&&!signalled;k++) {request(pipe,"poll\n",response,sizeof response);signalled=strstr(response,"\"restart_requested\":true")!=NULL;Sleep(5);}
        CHECK(signalled&&strstr(response,expected)&&strstr(response,"\"started\":true"));
        request(pipe,"shutdown\n",response,sizeof response);CHECK(strstr(response,"\"code\":0"));
        CHECK(WaitForSingleObject(child.hProcess,5000)==WAIT_OBJECT_0);DWORD code;CHECK(GetExitCodeProcess(child.hProcess,&code)&&code==0);
        CloseHandle(child.hProcess);DisconnectNamedPipe(pipe);CloseHandle(pipe);
    }
    puts("PASS real Console pipe: three connection modes, admission gate, restart request, normal shutdown and fresh sessions");return 0;
}
