/* Exercise the deployed --hidden flag without --integration-test, which
 * deliberately overrides startup flags. All fixture windows stay off screen. */
#include <windows.h>
#include <stdio.h>
#include <string>
#include "delay_ui.h"
#define CHECK(x) do { if(!(x)){fprintf(stderr,"FAIL line %d %s\n",__LINE__,#x);return 1;} } while(0)
static DWORD child_pid;
struct ChildLifetime {
    HANDLE process=NULL;
    ~ChildLifetime(){if(process){if(WaitForSingleObject(process,0)!=WAIT_OBJECT_0)TerminateProcess(process,1);CloseHandle(process);}}
};
static BOOL CALLBACK matching(HWND w,LPARAM out) {
    DWORD pid=0;GetWindowThreadProcessId(w,&pid);wchar_t cls[96];
    if(pid==child_pid && GetClassNameW(w,cls,96) && !wcscmp(cls,ND_UI_CLASS)){*(HWND *)out=w;return FALSE;}
    return TRUE;
}
static HWND panel() { HWND w=NULL;EnumWindows(matching,(LPARAM)&w);return w; }
static void pump() { MSG m;while(PeekMessageW(&m,NULL,0,0,PM_REMOVE)){TranslateMessage(&m);DispatchMessageW(&m);}Sleep(10); }
static bool topmost(HWND w) { return (GetWindowLongPtrW(w,GWL_EXSTYLE)&WS_EX_TOPMOST)!=0; }
int wmain(int argc,wchar_t **argv) {
    CHECK(argc==4);ChildLifetime child;
    WNDCLASSW cls={};cls.lpfnWndProc=DefWindowProcW;cls.hInstance=GetModuleHandleW(NULL);cls.lpszClassName=L"DelayWindowModesFixture";
    CHECK(RegisterClassW(&cls));
    HWND owner=CreateWindowExW(WS_EX_TOOLWINDOW|WS_EX_NOACTIVATE,cls.lpszClassName,L"Off-screen game fixture",WS_OVERLAPPEDWINDOW,-30000,-30000,1280,900,NULL,NULL,cls.hInstance,NULL);CHECK(owner);
    ShowWindow(owner,SW_SHOWNOACTIVATE);
    wchar_t ids[100];swprintf(ids,100,L" --owner %llu --pid %lu",(unsigned long long)(uintptr_t)owner,GetCurrentProcessId());
    std::wstring command=L"\"";command+=argv[1];command+=L"\" --preview --hidden";command+=ids;
    command+=L" --assets \"";command+=argv[2];command+=L"\" --data \"";command+=argv[3];command+=L"\"";
    STARTUPINFOW si={};si.cb=sizeof si;si.dwFlags=STARTF_USESHOWWINDOW;si.wShowWindow=SW_HIDE;PROCESS_INFORMATION pi={};
    CHECK(CreateProcessW(argv[1],&command[0],NULL,NULL,FALSE,CREATE_NO_WINDOW,NULL,NULL,&si,&pi));CloseHandle(pi.hThread);child.process=pi.hProcess;child_pid=pi.dwProcessId;
    HWND overlay=NULL;ULONGLONG deadline=GetTickCount64()+12000;
    while((!(overlay=panel()) || !GetPropW(overlay,L"NdPanelReady")) && GetTickCount64()<deadline)pump();
    CHECK(overlay && GetPropW(overlay,L"NdPanelReady"));
    CHECK(!IsWindowVisible(overlay) && !GetPropW(overlay,L"NdWebViewVisible"));
    // The same toggle handler is used after F8; no global keyboard injection.
    for(int cycle=0;cycle<3;cycle++){
        SendMessageW(overlay,WM_APP+242,0,0);
        CHECK(IsWindowVisible(overlay) && GetPropW(overlay,L"NdWebViewVisible"));
        CHECK(!topmost(overlay));
        SendMessageW(overlay,WM_CLOSE,0,0);
        CHECK(!IsWindowVisible(overlay) && !GetPropW(overlay,L"NdWebViewVisible"));
    }
    puts("PASS real --hidden launch opens and closes native/browser windows through the F8 toggle handler");
    // F11 uses HWND_TOP, not TOPMOST: exercise that exact layer distinction.
    HWND foreground=GetForegroundWindow();
    SetWindowLongPtrW(owner,GWL_STYLE,WS_POPUP|WS_VISIBLE);
    SetWindowPos(owner,HWND_TOP,-30000,-30000,1920,1080,SWP_NOACTIVATE|SWP_FRAMECHANGED|SWP_NOOWNERZORDER);
    CHECK(!topmost(owner));SendMessageW(overlay,WM_APP+242,0,0);
    CHECK(IsWindowVisible(overlay) && GetPropW(overlay,L"NdWebViewVisible") && topmost(overlay));
    CHECK(!topmost(owner) && GetForegroundWindow()==foreground);
    SendMessageW(overlay,WM_CLOSE,0,0);
    CHECK(!IsWindowVisible(overlay) && !topmost(overlay) && !topmost(owner));
    SendMessageW(overlay,WM_APP+242,0,0);
    CHECK(IsWindowVisible(overlay) && topmost(overlay) && !topmost(owner));
    // Return to windowed while the panel stays expanded, as with a second F11.
    SetWindowLongPtrW(owner,GWL_STYLE,WS_OVERLAPPEDWINDOW|WS_VISIBLE);
    SetWindowPos(owner,HWND_NOTOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE|SWP_FRAMECHANGED|SWP_NOOWNERZORDER);
    // Preview intentionally has no live game timer. Reopen to run the same
    // visibility handler; live F11 transitions are checked by the IPC fixture.
    SendMessageW(overlay,WM_CLOSE,0,0);SendMessageW(overlay,WM_APP+242,0,0);
    CHECK(IsWindowVisible(overlay) && !topmost(overlay) && !topmost(owner));
    SendMessageW(overlay,WM_CLOSE,0,0);
    puts("PASS F11 non-topmost borderless: temporary panel topmost, collapse clears it, windowed return restores ordinary layer without focus changes");
    // Menu fullscreen keeps the owner itself topmost. Hiding a panel must not
    // demote its owner, since Windows can cascade owned-window band changes.
    SetWindowLongPtrW(owner,GWL_STYLE,WS_POPUP|WS_VISIBLE);
    SetWindowPos(owner,HWND_TOPMOST,-30000,-30000,1920,1080,SWP_NOACTIVATE|SWP_FRAMECHANGED|SWP_NOOWNERZORDER);
    CHECK(topmost(owner));SendMessageW(overlay,WM_APP+242,0,0);
    CHECK(IsWindowVisible(overlay) && GetPropW(overlay,L"NdWebViewVisible") && topmost(overlay) && topmost(owner));
    SendMessageW(overlay,WM_CLOSE,0,0);
    CHECK(!IsWindowVisible(overlay) && !GetPropW(overlay,L"NdWebViewVisible") && topmost(owner));
    SetWindowLongPtrW(owner,GWL_STYLE,WS_OVERLAPPEDWINDOW|WS_VISIBLE);
    SetWindowPos(owner,HWND_NOTOPMOST,0,0,0,0,SWP_NOMOVE|SWP_NOSIZE|SWP_NOACTIVATE|SWP_FRAMECHANGED|SWP_NOOWNERZORDER);
    SendMessageW(overlay,WM_APP+242,0,0);
    CHECK(IsWindowVisible(overlay) && GetPropW(overlay,L"NdWebViewVisible") && !topmost(overlay) && !topmost(owner));
    puts("PASS menu fullscreen: shown panel topmost, hiding preserves owner policy, windowed return clears panel band");
    // End only this fixture's preview process, without touching user's game.
    PostMessageW(overlay,WM_DESTROY,0,0);
    deadline=GetTickCount64()+5000;while(WaitForSingleObject(pi.hProcess,0)!=WAIT_OBJECT_0 && GetTickCount64()<deadline)pump();
    CHECK(WaitForSingleObject(pi.hProcess,0)==WAIT_OBJECT_0);DestroyWindow(owner);return 0;
}
