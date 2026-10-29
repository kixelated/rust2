// The runtime every generated Kotlin file includes in its `Native` object, so no name it
// takes can clash with an exported type's: the codec values cross JNI in, the handles
// objects own, and the waker futures park on.

/** Encodes values as the library's JNI glue decodes them: little-endian, field by field. */
class Writer {
	private var b: java.nio.ByteBuffer = java.nio.ByteBuffer.allocate(64).order(java.nio.ByteOrder.LITTLE_ENDIAN)

	private fun room(n: Int) {
		if (b.remaining() < n) {
			val next = java.nio.ByteBuffer.allocate(kotlin.comparisons.maxOf(b.capacity() * 2, b.position() + n)).order(java.nio.ByteOrder.LITTLE_ENDIAN)
			next.put(b.array(), 0, b.position())
			b = next
		}
	}

	fun bool(v: Boolean) = i8(if (v) 1 else 0)

	fun i8(v: Byte) {
		room(1)
		b.put(v)
	}

	fun i16(v: Short) {
		room(2)
		b.putShort(v)
	}

	fun i32(v: Int) {
		room(4)
		b.putInt(v)
	}

	fun i64(v: Long) {
		room(8)
		b.putLong(v)
	}

	fun f32(v: Float) {
		room(4)
		b.putFloat(v)
	}

	fun f64(v: Double) {
		room(8)
		b.putDouble(v)
	}

	/** Bytes as their length, or -1 for none, then the bytes. */
	fun bytes(v: ByteArray?) {
		if (v == null) {
			i32(-1)
			return
		}
		room(4 + v.size)
		b.putInt(v.size)
		b.put(v)
	}

	fun string(v: String?) = bytes(v?.encodeToByteArray())

	/** An address as Rust's `SocketAddr` holds it, which an unresolved one has no form of. */
	fun socketAddr(v: java.net.InetSocketAddress) {
		val ip = kotlin.requireNotNull(v.address) { "an unresolved address has no Rust SocketAddr" }
		room(16)
		b.put(ip.address)
		repeat(16 - ip.address.size) { b.put(0) }
		i16(v.port.toShort())
		bool(ip is java.net.Inet6Address)
		// Java keeps no flow label.
		i32(0)
		i32((ip as? java.net.Inet6Address)?.scopeId ?: 0)
	}

	/** An IPv4 address, for Rust's `SocketAddrV4`. */
	fun socketAddrV4(v: java.net.InetSocketAddress) {
		kotlin.require(v.address !is java.net.Inet6Address) { "an IPv6 address has no Rust SocketAddrV4" }
		socketAddr(v)
	}

	/** An IPv6 address, for Rust's `SocketAddrV6`. */
	fun socketAddrV6(v: java.net.InetSocketAddress) {
		kotlin.require(v.address !is java.net.Inet4Address) { "an IPv4 address has no Rust SocketAddrV6" }
		socketAddr(v)
	}

	fun duration(v: kotlin.time.Duration) {
		kotlin.require(!v.isNegative()) { "a negative duration has no Rust Duration" }
		v.toComponents { secs, nanos ->
			i64(secs)
			i32(nanos)
		}
	}

	private var held: java.util.ArrayList<Handle>? = null

	/** An object's handle, held until [release], or 0 for none. */
	fun handle(v: Handle?) {
		if (v == null) {
			i64(0L)
			return
		}
		i64(v.hold())
		(held ?: java.util.ArrayList<Handle>().also { held = it }).add(v)
	}

	fun finish(): ByteArray = b.array().copyOf(b.position())

	/** Release the handles encoded, once the call that reads them returns. */
	fun release() {
		held?.forEach { it.release() }
	}

	/** Run [block] with this writer, releasing its handles once it returns. */
	inline fun <R> use(block: (Writer) -> R): R {
		try {
			return block(this)
		} finally {
			release()
		}
	}
}

/** Decodes what the library's JNI glue encodes. */
class Reader(bytes: ByteArray) {
	private val b: java.nio.ByteBuffer = java.nio.ByteBuffer.wrap(bytes).order(java.nio.ByteOrder.LITTLE_ENDIAN)

	fun bool(): Boolean = b.get() != 0.toByte()

	fun i8(): Byte = b.get()

	fun i16(): Short = b.getShort()

	fun i32(): Int = b.getInt()

	fun i64(): Long = b.getLong()

	fun f32(): Float = b.getFloat()

	fun f64(): Double = b.getDouble()

	fun bytes(): ByteArray? {
		val n = b.getInt()
		if (n < 0) return null
		val v = ByteArray(n)
		b.get(v)
		return v
	}

	fun string(): String? {
		val n = b.getInt()
		if (n < 0) return null
		val v = String(b.array(), b.arrayOffset() + b.position(), n, kotlin.text.Charsets.UTF_8)
		b.position(b.position() + n)
		return v
	}

	/** A path, which on Unix may hold bytes no String holds: those throw. */
	fun path(): String? = bytes()?.decodeToString(throwOnInvalidSequence = true)

	fun socketAddr(): java.net.InetSocketAddress {
		val ip = ByteArray(16)
		b.get(ip)
		val port = b.getShort().toInt() and 0xffff
		val v6 = bool()
		val flowinfo = b.getInt()
		val scope = b.getInt()
		kotlin.check(flowinfo == 0) { "an IPv6 flow label has no Java form" }
		// Built as Inet6Address directly, as InetAddress would read an IPv4-mapped one as IPv4.
		val address = when {
			!v6 -> java.net.InetAddress.getByAddress(ip.copyOf(4))
			scope == 0 -> java.net.Inet6Address.getByAddress(null, ip, null as java.net.NetworkInterface?)
			else -> java.net.Inet6Address.getByAddress(null, ip, scope)
		}
		return java.net.InetSocketAddress(address, port)
	}

	fun duration(): kotlin.time.Duration {
		val secs = b.getLong()
		val nanos = b.getInt()
		// Past Long.MAX_VALUE seconds, as no Kotlin Duration but its infinite one holds.
		if (secs < 0) return kotlin.time.Duration.INFINITE
		return secs.toDuration(kotlin.time.DurationUnit.SECONDS) + nanos.toDuration(kotlin.time.DurationUnit.NANOSECONDS)
	}
}

/**
 * A handle an object owns, or is lent for a call: each call holds it, so [close] frees it
 * only once no call does, and a call after [close] throws.
 */
class Handle(internal val raw: Long, private val free: ((Long) -> Unit)?, private val type: String) {
	// One for the owner until closed, and one per call holding it.
	private val holds = java.util.concurrent.atomic.AtomicLong(1)
	private val closed = java.util.concurrent.atomic.AtomicBoolean(false)

	fun hold(): Long {
		while (true) {
			val n = holds.get()
			kotlin.check(n != 0L && !closed.get()) { "$type is closed" }
			if (holds.compareAndSet(n, n + 1)) return raw
		}
	}

	fun release() {
		if (holds.decrementAndGet() == 0L) free?.invoke(raw)
	}

	fun close() {
		if (closed.compareAndSet(false, true)) release()
	}

	/** Run [block] with the raw handle, held until it returns. */
	inline fun <R> hold(block: (Long) -> R): R {
		val h = hold()
		try {
			return block(h)
		} finally {
			release()
		}
	}
}

/** Run [block] with the raw handle of [handle], or 0 for none, held until it returns. */
inline fun <R> hold(handle: Handle?, block: (Long) -> R): R = when (handle) {
	null -> block(0L)
	else -> handle.hold(block)
}

/** A waker a future parks on, which latches a wake that lands before it parks. */
class Signal : Waker {
	private val state = java.util.concurrent.atomic.AtomicReference<Any?>(null)

	override fun wake() {
		val parked = state.getAndSet(WOKEN)
		if (parked !is kotlinx.coroutines.CancellableContinuation<*>) return
		@Suppress("UNCHECKED_CAST")
		val c = parked as kotlinx.coroutines.CancellableContinuation<Unit>
		// Never resumed in place, since the next poll would run inside Rust's wake, which may
		// hold a lock that poll takes: a dispatcher that would resume here is made to defer,
		// and one that cannot, as `Unconfined`, resumes on `Default`.
		val dispatcher = c.context[kotlin.coroutines.ContinuationInterceptor] as? kotlinx.coroutines.CoroutineDispatcher
		when {
			dispatcher != null && dispatcher.isDispatchNeeded(c.context) -> c.resume(Unit)
			dispatcher != null && dispatcher !== kotlinx.coroutines.Dispatchers.Unconfined ->
				dispatcher.dispatch(c.context, java.lang.Runnable { c.resume(Unit) })
			else -> kotlinx.coroutines.Dispatchers.Default.dispatch(c.context, java.lang.Runnable { c.resume(Unit) })
		}
	}

	/** Forget any wake before the next poll, which sees whatever it was for. */
	fun reset() = state.set(null)

	/** Suspend until a wake since [reset], at once if one already landed. */
	suspend fun park() = kotlinx.coroutines.suspendCancellableCoroutine<Unit> { c ->
		if (!state.compareAndSet(null, c)) c.resume(Unit)
	}

	private companion object {
		val WOKEN = Any()
	}
}

/**
 * Await the future [raw]: poll it with one waker until it finishes, then lift its result.
 * A cancel frees it, which cancels it in Rust and loses nothing.
 */
suspend fun <T> await(
	raw: Long,
	poll: (Long, Long, Boolean) -> ByteArray?,
	free: (Long) -> Unit,
	lift: (Reader) -> T,
): T {
	val signal = Signal()
	val node = __waker_new(signal)
	try {
		var given = false
		while (true) {
			signal.reset()
			val done = poll(raw, node, given)
			given = true
			if (done != null) return lift(Reader(done))
			signal.park()
		}
	} finally {
		free(raw)
		__waker_free(node)
	}
}
