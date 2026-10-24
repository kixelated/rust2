import uniffi.uniffi_bindgen_tests.*

val testInterface = TestInterface(20u)
assert(testInterface.getValue() == 20u)
assert(cloneInterface(testInterface).getValue() == 20u)

// Test optional interfaces
assert(roundtripOptionalInterface(null) == null)
assert(roundtripOptionalInterface(TestInterface(20u))?.getValue() == 20u)

// Test secondary constructors
assert(TestInterface.secondaryConstructor(20u).getValue() == 40u)

fun assertDestroyedArgumentRefused(call: (TestInterface) -> Unit) {
    val destroyed = TestInterface(20u)
    destroyed.destroy()
    try {
        call(destroyed)
        throw RuntimeException("Expected destroyed argument to be refused")
    } catch (e: IllegalStateException) {
        assert(e.message?.contains("already been destroyed") == true)
    }
}

// Object lowering is shared by constructors, methods, and free functions.
assertDestroyedArgumentRefused { TestInterface.fromInterface(it) }
assertDestroyedArgumentRefused { testInterface.cloneOther(it) }
assertDestroyedArgumentRefused { cloneInterface(it) }

// Test records that store interfaces
val two = TwoTestInterfaces(first=TestInterface(1u), second=TestInterface(2u))
val swapped = swapTestInterfaces(two)
assert(swapped.first.getValue() == 2u)
assert(swapped.second.getValue() == 1u)

// Test enums that store interfaces
val en = TestInterfaceEnum.One(TestInterface(1u))
assert(en.i.getValue() == 1u)

// Test argument name mapping
// the following calls will fail if the argument name differs
testInterface.methodWithMultiWordArg(theArgument="test")

// Other bindgens test that we free references, but that's much harder for Kotlin.
// For now, we don't test this at all
