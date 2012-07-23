// Runtime support for pipes.

import unsafe::{forget, reinterpret_cast, transmute};
import either::{either, left, right};
import option::unwrap;
import arc::methods;

// Things used by code generated by the pipe compiler.
export entangle, get_buffer, drop_buffer;
export send_packet_buffered, recv_packet_buffered;
export mk_packet;

// export these so we can find them in the buffer_resource
// destructor. This is probably another metadata bug.
export atomic_add_acq, atomic_sub_rel;

// User-level things
export send_packet, recv_packet, send, recv, try_recv, peek;
export select, select2, selecti, select2i, selectable;
export spawn_service, spawn_service_recv;
export stream, port, chan, shared_chan, port_set, channel;

macro_rules! move {
    { $x:expr } => { unsafe { let y <- *ptr::addr_of($x); y } }
}

// This is to help make sure we only move out of enums in safe
// places. Once there is unary move, it can be removed.
fn move<T>(-x: T) -> T { x }

/**

Some thoughts about fixed buffers.

The idea is if a protocol is bounded, we will synthesize a record that
has a field for each state. Each of these states contains a packet for
the messages that are legal to be sent in that state. Then, instead of
allocating, the send code just finds a pointer to the right field and
uses that instead.

Unforunately, this makes things kind of tricky. We need to be able to
find the buffer, which means we need to pass it around. This could
either be associated with the (send|recv)_packet classes, or with the
packet itself. We will also need some form of reference counting so we
can track who has the responsibility of freeing the buffer.

We want to preserve the ability to do things like optimistic buffer
re-use, and skipping over to a new buffer when necessary. What I mean
is, suppose we had the typical stream protocol. It'd make sense to
amortize allocation costs by allocating a buffer with say 16
messages. When the sender gets to the end of the buffer, it could
check if the receiver is done with the packet in slot 0. If so, it can
just reuse that one, checking if the receiver is done with the next
one in each case. If it is ever not done, it just allocates a new
buffer and skips over to that.

Also, since protocols are in libcore, we have to do this in a way that
maintains backwards compatibility.

buffer header and buffer. Cast as c_void when necessary.

===

Okay, here are some new ideas.

It'd be nice to keep the bounded/unbounded case as uniform as
possible. It leads to less code duplication, and less things that can
go sublty wrong. For the bounded case, we could either have a struct
with a bunch of unique pointers to pre-allocated packets, or we could
lay them out inline. Inline layout is better, if for no other reason
than that we don't have to allocate each packet
individually. Currently we pass unique packets around as unsafe
pointers, but they are actually unique pointers. We should instead use
real unsafe pointers. This makes freeing data and running destructors
trickier though. Thus, we should allocate all packets in parter of a
higher level buffer structure. Packets can maintain a pointer to their
buffer, and this is the part that gets freed.

It might be helpful to have some idea of a semi-unique pointer (like
being partially pregnant, also like an ARC).

*/

enum state {
    empty,
    full,
    blocked,
    terminated
}

class buffer_header {
    // Tracks whether this buffer needs to be freed. We can probably
    // get away with restricting it to 0 or 1, if we're careful.
    let mut ref_count: int;

    new() { self.ref_count = 0; }

    // We may want a drop, and to be careful about stringing this
    // thing along.
}

// This is for protocols to associate extra data to thread around.
type buffer<T: send> = {
    header: buffer_header,
    data: T,
};

class packet_header {
    let mut state: state;
    let mut blocked_task: option<*rust_task>;

    // This is a reinterpret_cast of a ~buffer, that can also be cast
    // to a buffer_header if need be.
    let mut buffer: *libc::c_void;

    new() {
        self.state = empty;
        self.blocked_task = none;
        self.buffer = ptr::null();
    }

    // Returns the old state.
    unsafe fn mark_blocked(this: *rust_task) -> state {
        self.blocked_task = some(this);
        swap_state_acq(self.state, blocked)
    }

    unsafe fn unblock() {
        alt swap_state_acq(self.state, empty) {
          empty | blocked { }
          terminated { self.state = terminated; }
          full { self.state = full; }
        }
    }

    // unsafe because this can do weird things to the space/time
    // continuum. It ends making multiple unique pointers to the same
    // thing. You'll proobably want to forget them when you're done.
    unsafe fn buf_header() -> ~buffer_header {
        assert self.buffer.is_not_null();
        reinterpret_cast(self.buffer)
    }

    fn set_buffer<T: send>(b: ~buffer<T>) unsafe {
        self.buffer = reinterpret_cast(b);
    }
}

type packet<T: send> = {
    header: packet_header,
    mut payload: option<T>,
};

fn mk_packet<T: send>() -> packet<T> {
    {
        header: packet_header(),
        mut payload: none
    }
}

fn unibuffer<T: send>() -> ~buffer<packet<T>> {
    let b = ~{
        header: buffer_header(),
        data: {
            header: packet_header(),
            mut payload: none,
        }
    };

    unsafe {
        b.data.header.buffer = reinterpret_cast(b);
    }

    b
}

fn packet<T: send>() -> *packet<T> {
    let b = unibuffer();
    let p = ptr::addr_of(b.data);
    // We'll take over memory management from here.
    unsafe { forget(b) }
    p
}

#[abi = "rust-intrinsic"]
extern mod rusti {
    fn atomic_xchng(&dst: int, src: int) -> int;
    fn atomic_xchng_acq(&dst: int, src: int) -> int;
    fn atomic_xchng_rel(&dst: int, src: int) -> int;

    fn atomic_add_acq(&dst: int, src: int) -> int;
    fn atomic_sub_rel(&dst: int, src: int) -> int;
}

// If I call the rusti versions directly from a polymorphic function,
// I get link errors. This is a bug that needs investigated more.
fn atomic_xchng_rel(&dst: int, src: int) -> int {
    rusti::atomic_xchng_rel(dst, src)
}

fn atomic_add_acq(&dst: int, src: int) -> int {
    rusti::atomic_add_acq(dst, src)
}

fn atomic_sub_rel(&dst: int, src: int) -> int {
    rusti::atomic_sub_rel(dst, src)
}

type rust_task = libc::c_void;

extern mod rustrt {
    #[rust_stack]
    fn rust_get_task() -> *rust_task;

    #[rust_stack]
    fn task_clear_event_reject(task: *rust_task);

    fn task_wait_event(this: *rust_task, killed: &mut *libc::c_void) -> bool;
    pure fn task_signal_event(target: *rust_task, event: *libc::c_void);
}

fn wait_event(this: *rust_task) -> *libc::c_void {
    let mut event = ptr::null();

    let killed = rustrt::task_wait_event(this, &mut event);
    if killed && !task::failing() {
        fail ~"killed"
    }
    event
}

fn swap_state_acq(&dst: state, src: state) -> state {
    unsafe {
        reinterpret_cast(rusti::atomic_xchng_acq(
            *(ptr::mut_addr_of(dst) as *mut int),
            src as int))
    }
}

fn swap_state_rel(&dst: state, src: state) -> state {
    unsafe {
        reinterpret_cast(rusti::atomic_xchng_rel(
            *(ptr::mut_addr_of(dst) as *mut int),
            src as int))
    }
}

unsafe fn get_buffer<T: send>(p: *packet_header) -> ~buffer<T> {
    transmute((*p).buf_header())
}

class buffer_resource<T: send> {
    let buffer: ~buffer<T>;
    new(+b: ~buffer<T>) {
        let p = ptr::addr_of(*b);
        #error("take %?", p);
        atomic_add_acq(b.header.ref_count, 1);
        self.buffer = b;
    }

    drop unsafe {
        let b = move!{self.buffer};
        let p = ptr::addr_of(*b);
        #error("drop %?", p);
        let old_count = atomic_sub_rel(b.header.ref_count, 1);
        //let old_count = atomic_xchng_rel(b.header.ref_count, 0);
        if old_count == 1 {
            // The new count is 0.

            // go go gadget drop glue
        }
        else {
            forget(b)
        }
    }
}

fn send<T: send, Tbuffer: send>(-p: send_packet_buffered<T, Tbuffer>,
                                -payload: T) {
    let header = p.header();
    let p_ = p.unwrap();
    let p = unsafe { &*p_ };
    assert ptr::addr_of(p.header) == header;
    assert p.payload == none;
    p.payload <- some(payload);
    let old_state = swap_state_rel(p.header.state, full);
    alt old_state {
      empty {
        // Yay, fastpath.

        // The receiver will eventually clean this up.
        //unsafe { forget(p); }
      }
      full { fail ~"duplicate send" }
      blocked {
        #debug("waking up task for %?", p_);
        alt p.header.blocked_task {
          some(task) {
            rustrt::task_signal_event(
                task, ptr::addr_of(p.header) as *libc::c_void);
          }
          none { fail ~"blocked packet has no task" }
        }

        // The receiver will eventually clean this up.
        //unsafe { forget(p); }
      }
      terminated {
        // The receiver will never receive this. Rely on drop_glue
        // to clean everything up.
      }
    }
}

fn recv<T: send, Tbuffer: send>(-p: recv_packet_buffered<T, Tbuffer>) -> T {
    option::unwrap(try_recv(p))
}

fn try_recv<T: send, Tbuffer: send>(-p: recv_packet_buffered<T, Tbuffer>)
    -> option<T>
{
    let p_ = p.unwrap();
    let p = unsafe { &*p_ };
    let this = rustrt::rust_get_task();
    rustrt::task_clear_event_reject(this);
    p.header.blocked_task = some(this);
    let mut first = true;
    loop {
        rustrt::task_clear_event_reject(this);
        let old_state = swap_state_acq(p.header.state,
                                       blocked);
        alt old_state {
          empty {
            #debug("no data available on %?, going to sleep.", p_);
            wait_event(this);
            #debug("woke up, p.state = %?", copy p.header.state);
          }
          blocked {
            if first {
                fail ~"blocking on already blocked packet"
            }
          }
          full {
            let mut payload = none;
            payload <-> p.payload;
            p.header.state = terminated;
            ret some(option::unwrap(payload))
          }
          terminated {
            assert old_state == terminated;
            ret none;
          }
        }
        first = false;
    }
}

/// Returns true if messages are available.
pure fn peek<T: send>(p: recv_packet<T>) -> bool {
    alt unsafe {(*p.header()).state} {
      empty { false }
      blocked { fail ~"peeking on blocked packet" }
      full | terminated { true }
    }
}

fn sender_terminate<T: send>(p: *packet<T>) {
    let p = unsafe { &*p };
    alt swap_state_rel(p.header.state, terminated) {
      empty {
        // The receiver will eventually clean up.
        //unsafe { forget(p) }
      }
      blocked {
        // wake up the target
        let target = p.header.blocked_task.get();
        rustrt::task_signal_event(target,
                                  ptr::addr_of(p.header) as *libc::c_void);

        // The receiver will eventually clean up.
        //unsafe { forget(p) }
      }
      full {
        // This is impossible
        fail ~"you dun goofed"
      }
      terminated {
        // I have to clean up, use drop_glue
      }
    }
}

fn receiver_terminate<T: send>(p: *packet<T>) {
    let p = unsafe { &*p };
    alt swap_state_rel(p.header.state, terminated) {
      empty {
        // the sender will clean up
        //unsafe { forget(p) }
      }
      blocked {
        // this shouldn't happen.
        fail ~"terminating a blocked packet"
      }
      terminated | full {
        // I have to clean up, use drop_glue
      }
    }
}

#[doc = "Returns when one of the packet headers reports data is
available."]
fn wait_many(pkts: &[*packet_header]) -> uint {
    let this = rustrt::rust_get_task();

    rustrt::task_clear_event_reject(this);
    let mut data_avail = false;
    let mut ready_packet = pkts.len();
    for pkts.eachi |i, p| unsafe {
        let p = unsafe { &*p };
        let old = p.mark_blocked(this);
        alt old {
          full | terminated {
            data_avail = true;
            ready_packet = i;
            (*p).state = old;
            break;
          }
          blocked { fail ~"blocking on blocked packet" }
          empty { }
        }
    }

    while !data_avail {
        #debug("sleeping on %? packets", pkts.len());
        let event = wait_event(this) as *packet_header;
        let pos = vec::position(pkts, |p| p == event);

        alt pos {
          some(i) {
            ready_packet = i;
            data_avail = true;
          }
          none {
            #debug("ignoring spurious event, %?", event);
          }
        }
    }

    #debug("%?", pkts[ready_packet]);

    for pkts.each |p| { unsafe{ (*p).unblock()} }

    #debug("%?, %?", ready_packet, pkts[ready_packet]);

    unsafe {
        assert (*pkts[ready_packet]).state == full
            || (*pkts[ready_packet]).state == terminated;
    }

    ready_packet
}

fn select2<A: send, B: send>(
    +a: recv_packet<A>,
    +b: recv_packet<B>)
    -> either<(option<A>, recv_packet<B>), (recv_packet<A>, option<B>)>
{
    let i = wait_many([a.header(), b.header()]/_);

    unsafe {
        alt i {
          0 { left((try_recv(a), b)) }
          1 { right((a, try_recv(b))) }
          _ { fail ~"select2 return an invalid packet" }
        }
    }
}

trait selectable {
    pure fn header() -> *packet_header;
}

fn selecti<T: selectable>(endpoints: &[T]) -> uint {
    wait_many(endpoints.map(|p| p.header()))
}

fn select2i<A: selectable, B: selectable>(a: A, b: B) -> either<(), ()> {
    alt wait_many([a.header(), b.header()]/_) {
      0 { left(()) }
      1 { right(()) }
      _ { fail ~"wait returned unexpected index" }
    }
}

#[doc = "Waits on a set of endpoints. Returns a message, its index,
 and a list of the remaining endpoints."]
fn select<T: send>(+endpoints: ~[recv_packet<T>])
    -> (uint, option<T>, ~[recv_packet<T>])
{
    let ready = wait_many(endpoints.map(|p| p.header()));
    let mut remaining = ~[];
    let mut result = none;
    do vec::consume(endpoints) |i, p| {
        if i == ready {
            result = try_recv(p);
        }
        else {
            vec::push(remaining, p);
        }
    }

    (ready, result, remaining)
}

type send_packet<T: send> = send_packet_buffered<T, packet<T>>;

fn send_packet<T: send>(p: *packet<T>) -> send_packet<T> {
    send_packet_buffered(p)
}

class send_packet_buffered<T: send, Tbuffer: send> {
    let mut p: option<*packet<T>>;
    let mut buffer: option<buffer_resource<Tbuffer>>;
    new(p: *packet<T>) {
        //#debug("take send %?", p);
        self.p = some(p);
        unsafe {
            self.buffer = some(
                buffer_resource(
                    get_buffer(ptr::addr_of((*p).header))));
        };
    }
    drop {
        //if self.p != none {
        //    #debug("drop send %?", option::get(self.p));
        //}
        if self.p != none {
            let mut p = none;
            p <-> self.p;
            sender_terminate(option::unwrap(p))
        }
        unsafe { #error("send_drop: %?",
                        if self.buffer == none {
                            "none"
                        } else { "some" }); }
    }
    fn unwrap() -> *packet<T> {
        let mut p = none;
        p <-> self.p;
        option::unwrap(p)
    }

    pure fn header() -> *packet_header {
        alt self.p {
          some(packet) {
            unsafe {
                let packet = &*packet;
                let header = ptr::addr_of(packet.header);
                //forget(packet);
                header
            }
          }
          none { fail ~"packet already consumed" }
        }
    }

    fn reuse_buffer() -> buffer_resource<Tbuffer> {
        #error("send reuse_buffer");
        let mut tmp = none;
        tmp <-> self.buffer;
        option::unwrap(tmp)
    }
}

type recv_packet<T: send> = recv_packet_buffered<T, packet<T>>;

fn recv_packet<T: send>(p: *packet<T>) -> recv_packet<T> {
    recv_packet_buffered(p)
}

class recv_packet_buffered<T: send, Tbuffer: send> : selectable {
    let mut p: option<*packet<T>>;
    let mut buffer: option<buffer_resource<Tbuffer>>;
    new(p: *packet<T>) {
        //#debug("take recv %?", p);
        self.p = some(p);
        unsafe {
            self.buffer = some(
                buffer_resource(
                    get_buffer(ptr::addr_of((*p).header))));
        };
    }
    drop {
        //if self.p != none {
        //    #debug("drop recv %?", option::get(self.p));
        //}
        if self.p != none {
            let mut p = none;
            p <-> self.p;
            receiver_terminate(option::unwrap(p))
        }
        unsafe { #error("recv_drop: %?",
                        if self.buffer == none {
                            "none"
                        } else { "some" }); }
    }
    fn unwrap() -> *packet<T> {
        let mut p = none;
        p <-> self.p;
        option::unwrap(p)
    }

    pure fn header() -> *packet_header {
        alt self.p {
          some(packet) {
            unsafe {
                let packet = &*packet;
                let header = ptr::addr_of(packet.header);
                //forget(packet);
                header
            }
          }
          none { fail ~"packet already consumed" }
        }
    }

    fn reuse_buffer() -> buffer_resource<Tbuffer> {
        #error("recv reuse_buffer");
        let mut tmp = none;
        tmp <-> self.buffer;
        option::unwrap(tmp)
    }
}

fn entangle<T: send>() -> (send_packet<T>, recv_packet<T>) {
    let p = packet();
    (send_packet(p), recv_packet(p))
}

fn spawn_service<T: send>(
    init: extern fn() -> (send_packet<T>, recv_packet<T>),
    +service: fn~(+recv_packet<T>))
    -> send_packet<T>
{
    let (client, server) = init();

    // This is some nasty gymnastics required to safely move the pipe
    // into a new task.
    let server = ~mut some(server);
    do task::spawn |move service| {
        let mut server_ = none;
        server_ <-> *server;
        service(option::unwrap(server_))
    }

    client
}

fn spawn_service_recv<T: send>(
    init: extern fn() -> (recv_packet<T>, send_packet<T>),
    +service: fn~(+send_packet<T>))
    -> recv_packet<T>
{
    let (client, server) = init();

    // This is some nasty gymnastics required to safely move the pipe
    // into a new task.
    let server = ~mut some(server);
    do task::spawn |move service| {
        let mut server_ = none;
        server_ <-> *server;
        service(option::unwrap(server_))
    }

    client
}

// Streams - Make pipes a little easier in general.

proto! streamp {
    open:send<T: send> {
        data(T) -> open<T>
    }
}

// It'd be nice to call this send, but it'd conflict with the built in
// send kind.
trait channel<T: send> {
    fn send(+x: T);
}

trait recv<T: send> {
    fn recv() -> T;
    fn try_recv() -> option<T>;
    // This should perhaps be a new trait
    pure fn peek() -> bool;
}

type chan_<T:send> = { mut endp: option<streamp::client::open<T>> };

enum chan<T:send> {
    chan_(chan_<T>)
}

type port_<T:send> = { mut endp: option<streamp::server::open<T>> };

enum port<T:send> {
    port_(port_<T>)
}

fn stream<T:send>() -> (chan<T>, port<T>) {
    let (c, s) = streamp::init();

    (chan_({ mut endp: some(c) }), port_({ mut endp: some(s) }))
}

impl chan<T: send> of channel<T> for chan<T> {
    fn send(+x: T) {
        let mut endp = none;
        endp <-> self.endp;
        self.endp = some(
            streamp::client::data(unwrap(endp), x))
    }
}

impl port<T: send> of recv<T> for port<T> {
    fn recv() -> T {
        let mut endp = none;
        endp <-> self.endp;
        let streamp::data(x, endp) = pipes::recv(unwrap(endp));
        self.endp = some(endp);
        x
    }

    fn try_recv() -> option<T> {
        let mut endp = none;
        endp <-> self.endp;
        alt move(pipes::try_recv(unwrap(endp))) {
          some(streamp::data(x, endp)) {
            self.endp = some(move!{endp});
            some(move!{x})
          }
          none { none }
        }
    }

    pure fn peek() -> bool unchecked {
        let mut endp = none;
        endp <-> self.endp;
        let peek = alt endp {
          some(endp) {
            pipes::peek(endp)
          }
          none { fail ~"peeking empty stream" }
        };
        self.endp <-> endp;
        peek
    }
}

// Treat a whole bunch of ports as one.
class port_set<T: send> : recv<T> {
    let mut ports: ~[pipes::port<T>];

    new() { self.ports = ~[]; }

    fn add(+port: pipes::port<T>) {
        vec::push(self.ports, port)
    }

    fn try_recv() -> option<T> {
        let mut result = none;
        while result == none && self.ports.len() > 0 {
            let i = wait_many(self.ports.map(|p| p.header()));
            // dereferencing an unsafe pointer nonsense to appease the
            // borrowchecker.
            alt move(unsafe {(*ptr::addr_of(self.ports[i])).try_recv()}) {
              some(m) {
                  result = some(move!{m});
              }
              none {
                // Remove this port.
                let mut ports = ~[];
                self.ports <-> ports;
                vec::consume(ports,
                             |j, x| if i != j { vec::push(self.ports, x) });
              }
            }
        }
        result
    }

    fn recv() -> T {
        option::unwrap(self.try_recv())
    }

    pure fn peek() -> bool {
        // It'd be nice to use self.port.each, but that version isn't
        // pure.
        for vec::each(self.ports) |p| {
            if p.peek() { ret true }
        }
        false
    }
}

impl<T: send> of selectable for port<T> {
    pure fn header() -> *packet_header unchecked {
        alt self.endp {
          some(endp) {
            endp.header()
          }
          none { fail ~"peeking empty stream" }
        }
    }
}


type shared_chan<T: send> = arc::exclusive<chan<T>>;

impl chan<T: send> of channel<T> for shared_chan<T> {
    fn send(+x: T) {
        let mut xx = some(x);
        do self.with |_c, chan| {
            let mut x = none;
            x <-> xx;
            chan.send(option::unwrap(x))
        }
    }
}

fn shared_chan<T:send>(+c: chan<T>) -> shared_chan<T> {
    arc::exclusive(c)
}
