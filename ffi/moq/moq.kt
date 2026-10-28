// moq.c's script through the generated Kotlin: a broadcast's track, published and
// subscribed to in one process, with every read a suspend fun, and an origin, with
// moq-pattern, moq-clock, moq-tokio, hang, and moq-mux exported into the same library,
// and the catalog lent to Kotlin for a call.
//
// It needs no runtime: each read that must wait resumes on a dispatcher the script
// drains, after the write that wakes it.

import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.Executor
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.asCoroutineDispatcher
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import moq.*

var failures = 0

fun expect(ok: Boolean, what: String) {
	if (!ok) {
		System.err.println("failed: $what")
		failures++
	}
}

// The jobs coroutines resume with, run when the script drains them.
class Queue : Executor {
	private val jobs = ConcurrentLinkedQueue<Runnable>()

	override fun execute(job: Runnable) {
		jobs.add(job)
	}

	val size: Int
		get() = jobs.size

	fun drain() {
		while (true) {
			(jobs.poll() ?: return).run()
		}
	}
}

fun main() {
	val q = Queue()
	val scope = CoroutineScope(q.asCoroutineDispatcher())

	val broadcast = BroadcastInfo.new().produce()
	val track = broadcast.createTrack("video", null)
	expect(track.name() == "video", "a track has its name")

	// A second track of the same name is an error, printed as Rust prints it.
	try {
		broadcast.createTrack("video", null)
		expect(false, "a duplicate track is an error")
	} catch (e: ErrorException) {
		expect(e.message == "duplicate", "a duplicate track's error is Rust's")
	}

	val subscriber = runBlocking { track.consume().subscribe(null) }

	// A group's arrival wakes the subscriber's pending receive.
	var group: GroupConsumer? = null
	scope.launch { group = subscriber.recvGroup() }
	q.drain()
	expect(group == null, "a receive waits for a group")
	val writer = track.appendGroup()
	expect(q.size == 1, "a group's arrival wakes the receive")
	q.drain()
	val first = group!!
	expect(first.index() == 0uL, "the first group is 0")

	// So does a frame's.
	var frame: FrameFrame? = null
	scope.launch { frame = first.readFrame() }
	q.drain()
	expect(frame == null, "a read waits for a frame")
	writer.writeFrame(null, "hello")
	expect(q.size == 1, "a frame's arrival wakes the read")
	q.drain()
	val hello = frame!!
	expect(hello.payload.decodeToString() == "hello", "a frame has its payload")
	expect(hello.timestamp == null, "an untimed frame has no timestamp")
	// When it arrived: moq-clock's `Instant`, from moq-clock's own export.
	expect(now().checkedDurationSince(hello.arrived()) != null, "a frame arrived before now")

	// A timestamp is a `Copy` value behind a handle: passing it copies it out. The track is
	// untimed, so a timestamped frame is refused.
	val at = Timestamp.fromMillis(1500u)
	expect(at.asSecs() == 1uL, "1500 ms is 1 s")
	try {
		writer.writeFrame(at, "late")
		expect(false, "a timestamped frame on an untimed track is an error")
	} catch (e: ErrorException) {
		expect(e.message == "frame timestamp doesn't fit its track", "the late frame's error is Rust's")
	}
	expect(at.asSecs() == 1uL, "a copied timestamp stays usable")

	// The last frame, then the group's end, which reads as null.
	writer.writeFrame(null, "world")
	writer.finish()
	runBlocking {
		expect(first.readFrame()?.payload?.decodeToString() == "world", "the last frame reads")
		expect(first.readFrame() == null, "a finished group reads null")
	}

	// An origin and the driver that runs it, a pair. Its root, which it lends, is returned
	// as a new handle to a clone.
	val (origin, driver) = OriginProducer.new(OriginConfig.default())
	expect(origin.root().asStr() == "", "an origin's root is empty")

	// Scoped beneath a root to patterns, which moq-pattern's own export builds.
	Patterns.new().use { patterns ->
		expect(patterns.insert(Pattern.subtree("alice")), "a pattern inserts")
		origin.scope("room", patterns).use { scoped ->
			expect(scoped.root().asStr() == "room", "a scope has its root")
			expect(scoped.absolute("alice").asStr() == "room/alice", "a scope's paths are absolute")
			val allowed = scoped.allowed()
			expect(allowed.matches("alice/cam"), "a scope allows its patterns")
			expect(!allowed.matches("bob"), "a scope allows nothing else")
			scoped.consume().use { consumer ->
				expect(consumer.absolute("bob").asStr() == "room/bob", "a scope's consumer shares its root")
			}
		}
	}

	// A broadcast the origin creates is stamped with its path, which its info lends. Each
	// handle to it is closed, since an open one keeps it, and so the origin, alive.
	origin.createBroadcast("room/alice").use { publication ->
		val path = publication.broadcast().use { it.info().path }
		expect(path.asStr() == "room/alice", "a broadcast has its path")

		// Paths own their segments, so Kotlin builds them, and relative ones, from a string.
		val cam = Path.new("/cam/")
		expect(cam.asStr() == "cam", "a path trims its slashes")
		val joined = path.join(cam)
		expect(joined.asStr() == "room/alice/cam", "paths join")
		expect(joined.stripPrefix(path) == cam, "a path strips its prefix")
		expect(path.stripPrefix(joined) == null, "a path strips no longer prefix")
		expect(joined.resolve(Relative.new("../bob")).asStr() == "room/bob", "a relative path resolves")
	}

	// The driver, polled as a language's event loop would: at the time, and with a waiter
	// made from the waker and held across polls, since what a poll registers lives only as
	// long as its waiter. It runs until every owner of the origin is gone.
	var wakes = 0
	val waiter = Waiter.new(Waker { wakes++ })
	driver.poll(now(), waiter)
	// A client publishes the origin through moq-net's generic `with_publisher`, which a
	// consumer moves into.
	Client.new().withPublisher(origin.consume()).close()
	val before = wakes
	origin.close()
	// The last owner's close wakes the driver, which then finishes, cleanly.
	expect(wakes > before, "the last owner's close wakes the driver")
	try {
		driver.poll(now(), waiter)
		expect(false, "a driver whose origin is gone finishes")
	} catch (e: ErrorException) {
		expect(e.message == "closed", "a finished driver's error is Rust's")
	}

	// moq-mux reads an OpusHead into hang's catalog config: 44.1 kHz in, stereo, which the
	// catalog lists at the decoder's 48 kHz, keeping the head as its description.
	val head = "OpusHead".encodeToByteArray() + byteArrayOf(1, 2, 0x38, 0x01, 0x44, 0xac.toByte(), 0, 0, 0, 0, 0)
	val audio = codecOpusAudioConfig(head)
	expect(audio.sampleRate == 48000u, "Opus decodes at 48 kHz")
	expect(audio.channelCount == 2u, "the head is stereo")
	expect(audio.codec == CatalogAudioCodec.Opus, "the codec is Opus")
	expect(audio.description?.contentEquals(head) == true, "the head is the description")
	// A truncated head is moq-mux's error.
	try {
		codecOpusAudioConfig(head.copyOf(8))
		expect(false, "a truncated head is an error")
	} catch (e: MuxErrorException) {
		expect(e.message.isNotEmpty(), "a truncated head's error has a message")
	}

	// A binary track the catalog lists, published through moq-mux's generic
	// `binary_snapshot` at the config Kotlin builds.
	val publisher = BroadcastInfo.new().produce()
	val extra = CatalogConfig.default().withCatalog(HangCatalog.default())
	val catalog = CatalogProducer.new(publisher, extra)
	val snapshot = catalog.binarySnapshot(publisher.createTrack("state", null), BinaryConfig.default())
	expect(snapshot.name() == "state", "a snapshot has its track's name")
	snapshot.update("latest")
	var listed = 0
	catalog.mutate { lent ->
		if (lent.toJson().contains("state")) listed++
	}
	expect(listed == 1, "the catalog lists the snapshot's track")

	// moq-tokio's terminal connect errors, which moq-net's `Error` shares no names with.
	expect(ConnectError.UNAUTHORIZED.isAuth(), "unauthorized is an auth error")
	expect(ConnectError.UNAUTHORIZED.toString() == "unauthorized", "a connect error prints as Rust's")

	if (failures == 0) println("ok")
	kotlin.system.exitProcess(if (failures == 0) 0 else 1)
}
