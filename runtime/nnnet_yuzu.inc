/* Included after nnnet_c4a.inc. Keep offline handlers intact; only virtual FDs
 * and explicitly enabled room networking take these paths. */
#include "yuzu_net_internal.h"
#include "network_abi.h"
#define YPTR(a) ((void *)(uintptr_t)(a))
static int yfd(uint64_t fd) { return fd>=YN_VFD_BASE && fd<=0x3fffffff; }
static void yret(Ctx *c,int r) {
    if(r<0){yn_log(r==-11?3:2,"sdk.socket","fd=%d errno=%d caller=0x%llx",(int)A0,-r,(unsigned long long)c->x[30]);so_fail(c,-r);}else RETI(r);
}
#define YLDN(call) do { uint32_t result=(call);yn_log(2,"sdk.ldn","%s result=0x%x caller=0x%llx",#call,result,(unsigned long long)c->x[30]);RET(result); } while(0)
static void y_socket(Ctx *c) {
    if(!yn_enabled()){socket_Socket(c);return;}
    so_need_init(c,"nn::socket::Socket");
    if((A0&255)!=2){so_fail(c,97);return;}
    yret(c,yn_socket_create((int)A1,(int)A2));
}
static void y_finalize(Ctx *c) { yn_socket_close_all();socket_Finalize(c); }
static void y_close(Ctx *c) { if(yfd(A0))yret(c,yn_socket_close((int)A0));else socket_Close(c); }
static void y_bind(Ctx *c) {
    if(!yfd(A0)){socket_Bind(c);return;}
    struct sockaddr_in a;if(!A1 || A2<8){so_fail(c,22);return;}
    if(!so_addr_in(A1,&a)){so_fail(c,97);return;}yret(c,yn_socket_bind((int)A0,&a));
}
static void y_bind_typed(Ctx *c) { if(!yfd(A0))socket_BindSockAddr(c);else y_bind(c); }
static void y_connect(Ctx *c) {
    if(!yfd(A0)){socket_Connect(c);return;}
    struct sockaddr_in a;if(!A1 || A2<8){so_fail(c,22);return;}
    if(!so_addr_in(A1,&a)){so_fail(c,97);return;}yret(c,yn_socket_connect((int)A0,&a));
}
static void y_listen(Ctx *c) { if(yfd(A0))so_fail(c,95);else socket_Listen(c); }
static void y_accept(Ctx *c) { if(yfd(A0))so_fail(c,95);else socket_Accept(c); }
static void y_shutdown(Ctx *c) { if(yfd(A0))yret(c,yn_socket_shutdown((int)A0,(int)A1));else socket_Shutdown(c); }
static void y_name(Ctx *c,int peer) {
    if(!yfd(A0)){so_name(c,peer);return;}
    if(!A1 || !A2 || !M32(A2)){so_fail(c,22);return;}
    struct sockaddr_in a;int r=yn_socket_name((int)A0,&a,peer);
    if(!r)M32(A2)=so_addr_out(A1,M32(A2),&a);yret(c,r);
}
static void y_sockname(Ctx *c){y_name(c,0);}
static void y_peername(Ctx *c){y_name(c,1);}
static void y_recv_impl(Ctx *c,int from) {
    if(!yfd(A0)){if(from)socket_RecvFrom(c);else socket_Recv(c);return;}
    if(A2>0x7fffffff){so_fail(c,14);return;}
    struct sockaddr_in a={0};int r=yn_socket_recv((int)A0,YPTR(A1),(size_t)A2,(int)A3,&a);
    if(r>=0 && from && A4 && A5)M32(A5)=so_addr_out(A4,M32(A5),&a);yret(c,r);
}
static void y_recv(Ctx *c){y_recv_impl(c,0);}
static void y_recvfrom(Ctx *c){y_recv_impl(c,1);}
static void y_send_impl(Ctx *c,int to) {
    if(!yfd(A0)){if(to)socket_SendTo(c);else socket_Send(c);return;}
    struct sockaddr_in a,*dst=NULL;
    if(to && A4 && A5){if(A5<8){so_fail(c,22);return;}if(!so_addr_in(A4,&a)){so_fail(c,97);return;}dst=&a;}
    yret(c,yn_socket_send((int)A0,YPTR(A1),(size_t)A2,(int)A3,dst));
}
static void y_send(Ctx *c){y_send_impl(c,0);}
static void y_sendto(Ctx *c){y_send_impl(c,1);}
static void y_fcntl(Ctx *c){if(yfd(A0))yret(c,yn_socket_fcntl((int)A0,(int)A1,(int)A2));else socket_Fcntl(c);}
static void y_getopt(Ctx *c){
    if(!yfd(A0)){socket_GetSockOpt(c);return;}
    if(!A4){so_fail(c,22);return;}uint32_t n=M32(A4);
    int r=yn_socket_option((int)A0,0,(int)A1,(int)A2,YPTR(A3),&n);if(!r)M32(A4)=n;yret(c,r);
}
static void y_setopt(Ctx *c){
    if(!yfd(A0)){socket_SetSockOpt(c);return;}
    uint32_t n=(uint32_t)A4;yret(c,yn_socket_option((int)A0,1,(int)A1,(int)A2,YPTR(A3),&n));
}
static void y_poll(Ctx *c) {
    uint64_t p=A0,n=A1;int timeout=(int)A2;
    if(!p || n>SO_MAX_FD || timeout<-1){if(n>SO_MAX_FD)so_fail(c,22);else socket_Poll(c);return;}
    int any=0;for(size_t i=0;i<n;i++)if(yfd((uint32_t)M32(p+i*8)))any=1;
    if(!any){socket_Poll(c);return;}
    struct {int fd;uint16_t events,revents;} native[SO_MAX_FD];unsigned index[SO_MAX_FD];
    ULONGLONG end=timeout>=0?GetTickCount64()+(unsigned)timeout:0;
    for(;;){
        uint64_t seq=yn_activity();unsigned count=0;int ready=0;
        for(size_t i=0;i<n;i++){
            int fd=(int)M32(p+i*8);M16(p+i*8+6)=0;if(fd<0)continue;
            if(yfd((uint32_t)fd)){int v=yn_socket_poll(fd,M16(p+i*8+4));M16(p+i*8+6)=(uint16_t)v;if(v)ready++;}
            else {index[count]=(unsigned)i;native[count].fd=fd;native[count].events=M16(p+i*8+4);native[count].revents=0;count++;}
        }
        if(count){A0=P(native);A1=count;A2=0;socket_Poll(c);int r=(int)A0;A0=p;A1=n;A2=(uint64_t)(int64_t)timeout;
            if(r<0){RETI(-1);return;}ready+=r;
            for(unsigned j=0;j<count;j++)M16(p+index[j]*8+6)=native[j].revents;
        }
        ULONGLONG now=GetTickCount64();if(ready || (timeout>=0 && now>=end)){RETI(ready);return;}
        DWORD ms=timeout<0?INFINITE:(DWORD)(end-now);if(count && ms>10)ms=10;
        yn_wait_activity(seq,ms);
    }
}
static void yl_init(Ctx *c){YLDN(yn_ldn_initialize());}
static void yl_final(Ctx *c){YLDN(yn_ldn_finalize());}
static void yl_state(Ctx *c){RET(yn_ldn_state());}
static void yl_event(Ctx *c){
    if(!A0){RET(YN_LDN_ERROR(96));return;}
    yn_lock();
    if(!ssbu_yn.ldn_state){yn_unlock();RET(YN_LDN_ERROR(32));return;}
    if(ssbu_yn.ldn_event!=A0)os_init_event(A0,0);
    uint32_t r=yn_ldn_event(A0);yn_unlock();RET(r);
}
static void yl_open_ap(Ctx *c){YLDN(yn_ldn_open(0,0));}
static void yl_close_ap(Ctx *c){YLDN(yn_ldn_open(0,1));}
static void yl_open_sta(Ctx *c){YLDN(yn_ldn_open(1,0));}
static void yl_close_sta(Ctx *c){YLDN(yn_ldn_open(1,1));}
static void yl_create(Ctx *c){YLDN(yn_ldn_create(YPTR(A0),YPTR(A1),YPTR(A2),NULL));}
static void yl_create_private(Ctx *c){YLDN(yn_ldn_create(YPTR(A0),YPTR(A1),YPTR(A3),YPTR(A2)));}
static void yl_connect(Ctx *c){YLDN(yn_ldn_connect(YPTR(A0),YPTR(A2),(int)A3));}
static void yl_connect_private(Ctx *c){
    if(!A0 || !A2 || !A3){RET(YN_LDN_ERROR(96));return;}
    uint8_t filter[0x60]={0},*found=malloc(64*YN_LDN_SIZE);if(!found){RET(YN_LDN_ERROR(31));return;}
    memcpy(filter,YPTR(A0),16);memcpy(filter+16,YPTR(A2+16),16);yn_put32(filter+0x5c,0x23);
    int count=0;uint32_t r=yn_ldn_scan(found,&count,64,filter);
    if(!r)r=count?yn_ldn_connect(found,YPTR(A3),(int)A4):YN_LDN_ERROR(97);
    free(found);RET(r);
}
static void yl_destroy(Ctx *c){YLDN(yn_ldn_disconnect(1));}
static void yl_disconnect(Ctx *c){YLDN(yn_ldn_disconnect(0));}
static void yl_info(Ctx *c){YLDN(yn_ldn_info(YPTR(A0)));}
static void yl_config(Ctx *c){YLDN(yn_ldn_config(YPTR(A0)));}
static void yl_ip(Ctx *c){YLDN(yn_ldn_ip(YPTR(A0),YPTR(A1)));}
static void yl_security(Ctx *c){YLDN(yn_ldn_security(YPTR(A0)));}
static void yl_advertise(Ctx *c){YLDN(yn_ldn_advertise(YPTR(A0),(size_t)A1));}
static void yl_scan(Ctx *c){YLDN(yn_ldn_scan(YPTR(A0),YPTR(A1),(int)A2,YPTR(A3)));}
static void yl_policy(Ctx *c){YLDN(yn_ldn_policy((unsigned)A0));}
static void yl_reject(Ctx *c){YLDN(yn_ldn_reject((uint32_t)A0));}
static void yl_reason(Ctx *c){RET(yn_ldn_reason());}
static void yl_controller(Ctx *c){RET(yn_ldn_state()?0:YN_LDN_ERROR(32));}
/* Protocol bindings are installed by nnldn_register, the single network entry. */

ImportFn yuzu_protocol_handler(NetworkAbi operation) {
    static const ImportFn handlers[NET_ABI_COUNT] = {
        [NET_ABI_SOCKET_FINALIZE] = y_finalize,
        [NET_ABI_SOCKET_SOCKET] = y_socket,
        [NET_ABI_SOCKET_CLOSE] = y_close,
        [NET_ABI_SOCKET_BIND] = y_bind,
        [NET_ABI_SOCKET_BIND_TYPED] = y_bind_typed,
        [NET_ABI_SOCKET_CONNECT] = y_connect,
        [NET_ABI_SOCKET_LISTEN] = y_listen,
        [NET_ABI_SOCKET_SHUTDOWN] = y_shutdown,
        [NET_ABI_SOCKET_ACCEPT] = y_accept,
        [NET_ABI_SOCKET_SOCKNAME] = y_sockname,
        [NET_ABI_SOCKET_PEERNAME] = y_peername,
        [NET_ABI_SOCKET_RECV] = y_recv,
        [NET_ABI_SOCKET_RECVFROM] = y_recvfrom,
        [NET_ABI_SOCKET_SEND] = y_send,
        [NET_ABI_SOCKET_SENDTO] = y_sendto,
        [NET_ABI_SOCKET_FCNTL] = y_fcntl,
        [NET_ABI_SOCKET_POLL] = y_poll,
        [NET_ABI_SOCKET_GETOPT] = y_getopt,
        [NET_ABI_SOCKET_SETOPT] = y_setopt,
        [NET_ABI_LDN_INIT] = yl_init,
        [NET_ABI_LDN_STATE] = yl_state,
        [NET_ABI_LDN_FINAL] = yl_final,
        [NET_ABI_LDN_EVENT] = yl_event,
        [NET_ABI_LDN_CONTROLLER] = yl_controller,
        [NET_ABI_LDN_OPEN_STA] = yl_open_sta,
        [NET_ABI_LDN_CLOSE_STA] = yl_close_sta,
        [NET_ABI_LDN_OPEN_AP] = yl_open_ap,
        [NET_ABI_LDN_CLOSE_AP] = yl_close_ap,
        [NET_ABI_LDN_DISCONNECT] = yl_disconnect,
        [NET_ABI_LDN_DESTROY] = yl_destroy,
        [NET_ABI_LDN_INFO] = yl_info,
        [NET_ABI_LDN_CONFIG] = yl_config,
        [NET_ABI_LDN_IP] = yl_ip,
        [NET_ABI_LDN_SECURITY] = yl_security,
        [NET_ABI_LDN_ADVERTISE] = yl_advertise,
        [NET_ABI_LDN_POLICY] = yl_policy,
        [NET_ABI_LDN_REJECT] = yl_reject,
        [NET_ABI_LDN_SCAN] = yl_scan,
        [NET_ABI_LDN_CREATE] = yl_create,
        [NET_ABI_LDN_CREATE_PRIVATE] = yl_create_private,
        [NET_ABI_LDN_CONNECT] = yl_connect,
        [NET_ABI_LDN_CONNECT_PRIVATE] = yl_connect_private,
        [NET_ABI_LDN_REASON] = yl_reason,
    };
    return (unsigned)operation < NET_ABI_COUNT ? handlers[operation] : NULL;
}
