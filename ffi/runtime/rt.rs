//! The runtime every export shares: buffers, handles, wakers, and futures across the
//! C ABI. rust2-ffi copies it beside the generated code.
//!
//! A broken contract across the ABI (a null or consumed handle, a handle borrowed in a
//! way Rust's rules forbid, invalid UTF-8, a future polled after it finished) aborts with
//! a message naming the C function: there is no Rust caller to return an error to.

use std::cell::UnsafeCell;
use std::ffi::c_void;
use std::mem::ManuallyDrop;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::task::{Context, Wake};

/// Abort with a message naming the C function whose contract was broken.
#[cold]
pub fn fail(what: &str, msg: &str) -> ! {
	eprintln!("{what}: {msg}");
	std::process::abort()
}

/// Bytes across the ABI. Passed in, a view Rust copies or borrows for the call and never
/// frees. Returned, an owned buffer the caller frees once. A null `ptr` is `None`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Buf {
	pub ptr: *const u8,
	pub len: usize,
	pub owner: *mut c_void,
}

impl Buf {
	pub const NONE: Self = Self {
		ptr: std::ptr::null(),
		len: 0,
		owner: std::ptr::null_mut(),
	};

	pub fn is_none(&self) -> bool {
		self.ptr.is_null()
	}

	/// The bytes a caller passed in, which it keeps.
	///
	/// # Safety
	/// `ptr` points at `len` readable bytes for `'a`.
	pub unsafe fn bytes<'a>(self, what: &str) -> &'a [u8] {
		match (self.ptr.is_null(), self.len) {
			(_, 0) => &[],
			(true, _) => fail(what, "a buffer has a length but no pointer"),
			// SAFETY: the caller's contract.
			(false, len) => unsafe { std::slice::from_raw_parts(self.ptr, len) },
		}
	}

	/// The string a caller passed in, which it keeps.
	///
	/// # Safety
	/// As [`Self::bytes`].
	pub unsafe fn str<'a>(self, what: &str) -> &'a str {
		// SAFETY: the caller's contract.
		match std::str::from_utf8(unsafe { self.bytes(what) }) {
			Ok(s) => s,
			Err(_) => fail(what, "a string is not UTF-8"),
		}
	}

	/// A view of bytes Rust keeps, for a callback to read during its call.
	pub fn view(bytes: &[u8]) -> Self {
		Self {
			ptr: bytes.as_ptr(),
			len: bytes.len(),
			owner: std::ptr::null_mut(),
		}
	}

	/// Bytes handed to the caller, which frees them.
	pub fn owned(bytes: bytes::Bytes) -> Self {
		let ptr = bytes.as_ptr();
		let len = bytes.len();
		let owner = Box::into_raw(Box::new(bytes)).cast();
		Self { ptr, len, owner }
	}

	pub fn string(string: String) -> Self {
		Self::owned(string.into())
	}

	pub fn vec(vec: Vec<u8>) -> Self {
		Self::owned(vec.into())
	}

	/// Free a buffer Rust returned; a view or `None` frees nothing.
	///
	/// # Safety
	/// `owner` is null or came from [`Self::owned`], and is freed once.
	pub unsafe fn free(self) {
		if !self.owner.is_null() {
			// SAFETY: the caller's contract.
			drop(unsafe { Box::from_raw(self.owner.cast::<bytes::Bytes>()) });
		}
	}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Duration {
	pub secs: u64,
	pub nanos: u32,
}

impl Duration {
	pub fn lift(self, what: &str) -> std::time::Duration {
		if self.nanos >= 1_000_000_000 {
			fail(what, "a duration has a billion nanoseconds or more");
		}
		std::time::Duration::new(self.secs, self.nanos)
	}

	pub fn lower(duration: std::time::Duration) -> Self {
		Self {
			secs: duration.as_secs(),
			nanos: duration.subsec_nanos(),
		}
	}
}

pub fn char(code: u32, what: &str) -> char {
	char::from_u32(code).unwrap_or_else(|| fail(what, "a char is not a Unicode scalar value"))
}

const EXCLUSIVE: usize = usize::MAX;
const TAKEN: usize = usize::MAX - 1;

/// A value behind a handle: shared through an `Arc`, and borrowed as Rust borrows it, with
/// the borrow checked at run time. `state` counts shared borrows, or is [`EXCLUSIVE`]
/// while one exclusive borrow lasts, or [`TAKEN`] once the value moved out.
pub struct Object<T> {
	state: AtomicUsize,
	value: UnsafeCell<ManuallyDrop<T>>,
}

// SAFETY: the value moves between threads only by `take` or an exclusive borrow, and is
// shared only by shared borrows, which the export takes only of `Sync` types.
unsafe impl<T: Send> Send for Object<T> {}
// SAFETY: as above.
unsafe impl<T: Send> Sync for Object<T> {}

impl<T> Object<T> {
	/// A new handle owning `value`.
	pub fn new(value: T) -> *const c_void {
		Arc::into_raw(Arc::new(Self {
			state: AtomicUsize::new(0),
			value: UnsafeCell::new(ManuallyDrop::new(value)),
		}))
		.cast()
	}

	/// # Safety
	/// `handle` is null or a live handle of this type.
	unsafe fn get<'a>(handle: *const c_void, what: &str) -> &'a Self {
		if handle.is_null() {
			fail(what, "a handle is null");
		}
		// SAFETY: the caller's contract.
		unsafe { &*handle.cast::<Self>() }
	}

	fn acquire(&self, what: &str) {
		let mut state = self.state.load(Ordering::Relaxed);
		loop {
			match state {
				TAKEN => fail(what, "a handle's value was moved out"),
				EXCLUSIVE => fail(what, "a handle is borrowed exclusively"),
				_ => {}
			}
			match self
				.state
				.compare_exchange_weak(state, state + 1, Ordering::Acquire, Ordering::Relaxed)
			{
				Ok(_) => return,
				Err(now) => state = now,
			}
		}
	}

	fn acquire_mut(&self, what: &str) {
		if let Err(state) = self
			.state
			.compare_exchange(0, EXCLUSIVE, Ordering::Acquire, Ordering::Relaxed)
		{
			match state {
				TAKEN => fail(what, "a handle's value was moved out"),
				_ => fail(what, "a handle is already borrowed"),
			}
		}
	}

	/// Borrow the value shared for a call.
	///
	/// # Safety
	/// `handle` is a live handle of this type, kept alive through the borrow.
	pub unsafe fn borrow<'a>(handle: *const c_void, what: &str) -> Shared<'a, T> {
		// SAFETY: the caller's contract.
		let object = unsafe { Self::get(handle, what) };
		object.acquire(what);
		Shared { object }
	}

	/// Borrow the value exclusively for a call.
	///
	/// # Safety
	/// As [`Self::borrow`].
	pub unsafe fn borrow_mut<'a>(handle: *const c_void, what: &str) -> Exclusive<'a, T> {
		// SAFETY: the caller's contract.
		let object = unsafe { Self::get(handle, what) };
		object.acquire_mut(what);
		Exclusive { object }
	}

	/// Borrow the value shared for as long as a future runs, keeping the handle alive.
	///
	/// # Safety
	/// `handle` is a live handle of this type.
	pub unsafe fn borrow_arc(handle: *const c_void, what: &str) -> SharedArc<T> {
		// SAFETY: the caller's contract.
		let object = unsafe { Self::arc(handle, what) };
		object.acquire(what);
		SharedArc { object }
	}

	/// Borrow the value exclusively for as long as a future runs.
	///
	/// # Safety
	/// As [`Self::borrow_arc`].
	pub unsafe fn borrow_mut_arc(handle: *const c_void, what: &str) -> ExclusiveArc<T> {
		// SAFETY: the caller's contract.
		let object = unsafe { Self::arc(handle, what) };
		object.acquire_mut(what);
		ExclusiveArc { object }
	}

	/// # Safety
	/// `handle` is a live handle of this type.
	unsafe fn arc(handle: *const c_void, what: &str) -> Arc<Self> {
		// SAFETY: the caller's contract.
		unsafe { Self::get(handle, what) };
		let ptr = handle.cast::<Self>();
		// SAFETY: a live handle holds a count, so one more keeps it alive.
		unsafe {
			Arc::increment_strong_count(ptr);
			Arc::from_raw(ptr)
		}
	}

	/// Move the value out, leaving the handle to be freed.
	///
	/// # Safety
	/// As [`Self::borrow`].
	pub unsafe fn take(handle: *const c_void, what: &str) -> T {
		// SAFETY: the caller's contract.
		let object = unsafe { Self::get(handle, what) };
		if let Err(state) = object
			.state
			.compare_exchange(0, TAKEN, Ordering::Acquire, Ordering::Relaxed)
		{
			match state {
				TAKEN => fail(what, "a handle's value was moved out"),
				_ => fail(what, "a handle is borrowed"),
			}
		}
		// SAFETY: TAKEN excludes every other access, and the value is never read again.
		unsafe { ManuallyDrop::take(&mut *object.value.get()) }
	}

	/// Release the caller's handle.
	///
	/// # Safety
	/// `handle` is null or a live handle of this type, released once.
	pub unsafe fn free(handle: *const c_void) {
		if !handle.is_null() {
			// SAFETY: the caller's contract.
			drop(unsafe { Arc::from_raw(handle.cast::<Self>()) });
		}
	}
}

impl<T> Drop for Object<T> {
	fn drop(&mut self) {
		if *self.state.get_mut() != TAKEN {
			// SAFETY: the last reference is gone and the value was never taken.
			unsafe { ManuallyDrop::drop(self.value.get_mut()) };
		}
	}
}

pub struct Shared<'a, T> {
	object: &'a Object<T>,
}

impl<T> Deref for Shared<'_, T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFETY: a shared borrow is held.
		unsafe { &*self.object.value.get() }
	}
}

impl<T> Drop for Shared<'_, T> {
	fn drop(&mut self) {
		self.object.state.fetch_sub(1, Ordering::Release);
	}
}

pub struct Exclusive<'a, T> {
	object: &'a Object<T>,
}

impl<T> Deref for Exclusive<'_, T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFETY: the exclusive borrow is held.
		unsafe { &*self.object.value.get() }
	}
}

impl<T> DerefMut for Exclusive<'_, T> {
	fn deref_mut(&mut self) -> &mut T {
		// SAFETY: the exclusive borrow is held.
		unsafe { &mut *self.object.value.get() }
	}
}

impl<T> Drop for Exclusive<'_, T> {
	fn drop(&mut self) {
		self.object.state.store(0, Ordering::Release);
	}
}

pub struct SharedArc<T> {
	object: Arc<Object<T>>,
}

impl<T> Deref for SharedArc<T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFETY: a shared borrow is held.
		unsafe { &*self.object.value.get() }
	}
}

impl<T> Drop for SharedArc<T> {
	fn drop(&mut self) {
		self.object.state.fetch_sub(1, Ordering::Release);
	}
}

pub struct ExclusiveArc<T> {
	object: Arc<Object<T>>,
}

impl<T> Deref for ExclusiveArc<T> {
	type Target = T;
	fn deref(&self) -> &T {
		// SAFETY: the exclusive borrow is held.
		unsafe { &*self.object.value.get() }
	}
}

impl<T> DerefMut for ExclusiveArc<T> {
	fn deref_mut(&mut self) -> &mut T {
		// SAFETY: the exclusive borrow is held.
		unsafe { &mut *self.object.value.get() }
	}
}

impl<T> Drop for ExclusiveArc<T> {
	fn drop(&mut self) {
		self.object.state.store(0, Ordering::Release);
	}
}

/// A foreign waker: `wake` may be called from any thread, any number of times, until
/// `drop` releases `data`. Either function may be null.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Waker {
	pub wake: Option<unsafe extern "C" fn(*mut c_void)>,
	pub drop: Option<unsafe extern "C" fn(*mut c_void)>,
	pub data: *mut c_void,
}

impl Waker {
	/// Whether `other` is this waker again, which a poll reuses rather than wrapping anew.
	fn same(&self, other: &Self) -> bool {
		self.wake.map(|f| f as usize) == other.wake.map(|f| f as usize) && self.data == other.data
	}
}

struct Foreign(Waker);

// SAFETY: the waker's contract: `wake` and `drop` may be called from any thread.
unsafe impl Send for Foreign {}
// SAFETY: as above.
unsafe impl Sync for Foreign {}

impl Wake for Foreign {
	fn wake(self: Arc<Self>) {
		self.wake_by_ref();
	}

	fn wake_by_ref(self: &Arc<Self>) {
		if let Some(wake) = self.0.wake {
			// SAFETY: the waker's contract.
			unsafe { wake(self.0.data) };
		}
	}
}

impl Drop for Foreign {
	fn drop(&mut self) {
		if let Some(drop) = self.0.drop {
			// SAFETY: the waker's contract; this is the last reference.
			unsafe { drop(self.0.data) };
		}
	}
}

/// Writes a finished future's result through the poll's out pointers, returning the poll.
pub type Done = Box<dyn FnOnce(*mut c_void, *mut c_void) -> i32 + Send>;

pub const PENDING: i32 = 0;
pub const READY: i32 = 1;
pub const ERROR: i32 = 2;

/// A future behind a handle, polled from C.
pub struct Future {
	future: Option<Pin<Box<dyn std::future::Future<Output = Done> + Send>>>,
	/// The last foreign waker, and the Rust waker around it.
	waker: Option<(Waker, std::task::Waker)>,
	polling: AtomicBool,
}

impl Future {
	pub fn new(future: impl std::future::Future<Output = Done> + Send + 'static) -> *mut Self {
		Box::into_raw(Box::new(Self {
			future: Some(Box::pin(future)),
			waker: None,
			polling: AtomicBool::new(false),
		}))
	}

	/// Poll it once with `waker`, writing the result through `out` and `err` when ready.
	///
	/// # Safety
	/// `future` is a live future handle, and `out` and `err` are null or point where the
	/// function's result types go.
	pub unsafe fn poll(future: *mut Self, waker: Waker, out: *mut c_void, err: *mut c_void, what: &str) -> i32 {
		if future.is_null() {
			fail(what, "a future is null");
		}
		// SAFETY: the caller's contract; only the flag is touched before it is held.
		if unsafe { &*future }.polling.swap(true, Ordering::Acquire) {
			fail(what, "a future is polled concurrently");
		}
		// SAFETY: the flag makes this the only access.
		let this = unsafe { &mut *future };
		let Some(inner) = this.future.as_mut() else {
			fail(what, "a future is polled after it finished");
		};
		if !this.waker.as_ref().is_some_and(|(last, _)| last.same(&waker)) {
			this.waker = Some((waker, std::task::Waker::from(Arc::new(Foreign(waker)))));
		}
		let (_, rust) = this.waker.as_ref().expect("set above");
		let mut cx = Context::from_waker(rust);
		let poll = inner.as_mut().poll(&mut cx);
		let result = match poll {
			std::task::Poll::Pending => PENDING,
			std::task::Poll::Ready(done) => {
				this.future = None;
				this.waker = None;
				done(out, err)
			}
		};
		this.polling.store(false, Ordering::Release);
		result
	}

	/// Drop the future, cancelling it if it has not finished.
	///
	/// # Safety
	/// `future` is null or a live future handle, freed once and not being polled.
	pub unsafe fn free(future: *mut Self) {
		if !future.is_null() {
			// SAFETY: the caller's contract.
			drop(unsafe { Box::from_raw(future) });
		}
	}
}

/// Write `value` through `out` if the caller wants it, or drop it.
///
/// # Safety
/// `out` is null or points where a `T` goes.
pub unsafe fn write<T>(out: *mut c_void, value: impl FnOnce() -> T) {
	if !out.is_null() {
		// SAFETY: the caller's contract.
		unsafe { out.cast::<T>().write(value()) };
	}
}
