// bench/export.cpp's frame writes and reads through the generated Kotlin, so the two differ
// by what JNI and the Kotlin wrapper add. Each round runs twice, the first to warm the JIT.

import kotlinx.coroutines.runBlocking
import moq.*

const val GROUPS = 10000
const val FRAMES = 100

// Nanoseconds per frame since `start`.
fun perFrame(start: Long): Double = (System.nanoTime() - start).toDouble() / (GROUPS * FRAMES)

fun round(): String {
	val broadcast = BroadcastInfo.new().produce()
	val track = Timescale.MICRO.use { micro ->
		broadcast.createTrack("bench", TrackInfo.default().withTimescale(micro))
	}

	// A frame of 100 zero bytes, as bench/export.c writes.
	val frame = ByteArray(100)
	var micros = 0uL

	// Each frame stamped through a fresh timestamp handle, as a caller holding a u64 must.
	var start = System.nanoTime()
	repeat(GROUPS) {
		track.appendGroup().use { group ->
			repeat(FRAMES) {
				Timestamp.fromMicros(micros++).use { at -> group.writeFrame(at, frame) }
			}
			group.finish()
		}
	}
	val stamped = perFrame(start)

	// The same, reusing one timestamp handle: what the handle costs.
	val at = Timestamp.fromMicros(micros)
	start = System.nanoTime()
	repeat(GROUPS) {
		track.appendGroup().use { group ->
			repeat(FRAMES) { group.writeFrame(at, frame) }
			group.finish()
		}
	}
	val reused = perFrame(start)

	// The floor: a hold of a handle and a bool back.
	start = System.nanoTime()
	var closed = 0
	repeat(GROUPS * FRAMES) {
		if (track.isClosed()) closed++
	}
	val call = perFrame(start)

	// Reads: each frame written, then read through a suspend fun whose future is ready at
	// its first poll, so it never suspends.
	val read = runBlocking {
		val subscriber = track.consume().use { it.subscribe(null) }
		val start = System.nanoTime()
		repeat(GROUPS) {
			track.appendGroup().use { group ->
				subscriber.recvGroup()!!.use { reader ->
					repeat(FRAMES) {
						group.writeFrame(at, frame)
						reader.readFrame()!!.close()
					}
				}
				group.finish()
			}
		}
		perFrame(start).also { subscriber.close() }
	}
	at.close()
	track.close()
	broadcast.close()

	val bang = if (closed != 0) "!" else ""
	return "export (Kotlin): %.0f ns per stamped 100 B frame, %.0f reusing the timestamp, %.1f ns per handle call, %.0f ns per frame written and read%s"
		.format(stamped, reused, call, read, bang)
}

fun main() {
	round()
	println(round())
}
