/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package binding_tests

import (
	"context"
	"fmt"
	"runtime"
	"strconv"
	"sync"
	"testing"
	"time"

	. "github.com/NordSecurity/uniffi-bindgen-go/binding_tests/generated/futures"

	"github.com/stretchr/testify/assert"
)

func bg() context.Context {
	return context.Background()
}

func assertInstantExecution(t *testing.T, start time.Time) {
	t1 := time.Now()
	elapsed := t1.Sub(start)

	fmt.Printf("elapsed %s\n", elapsed)
	assert.True(t, elapsed < 10*time.Millisecond)
	assert.True(t, elapsed > 0)
}

func assertDelayedExecution(t *testing.T, start time.Time, delay time.Duration) {
	t1 := time.Now()
	elapsed := t1.Sub(start)

	fmt.Printf("elapsed %s\n", elapsed)
	assert.True(t, elapsed < delay+50*time.Millisecond)
	assert.True(t, elapsed > delay)
}

func TestFuturesAlwaysReady(t *testing.T) {
	// Test `alwaysReady`
	t0 := time.Now()
	result, err := AlwaysReady(bg())
	assert.NoError(t, err)
	assertInstantExecution(t, t0)
	assert.True(t, result)
}

func TestFuturesRecord(t *testing.T) {
	// Test record.

	result, err := NewMyRecord(bg(), "foo", 42)
	assert.NoError(t, err)
	assert.Equal(t, result.A, "foo")
	assert.Equal(t, result.B, uint32(42))

	// Test `void`
	t0 := time.Now()
	assert.NoError(t, Void(bg()))
	assertInstantExecution(t, t0)
}

func TestFuturesSleep(t *testing.T) {
	// Test `Sleep`
	t0 := time.Now()
	result, err := Sleep(bg(), 200)
	assert.NoError(t, err)

	assertDelayedExecution(t, t0, 200*time.Millisecond)
	assert.True(t, result)
}

func TestFuturesSequential(t *testing.T) {
	// Test sequential futures.
	t0 := time.Now()
	resultAlice, err := SayAfter(bg(), 100, "Alice")
	assert.NoError(t, err)
	resultBob, err := SayAfter(bg(), 200, "Bob")
	assert.NoError(t, err)

	assertDelayedExecution(t, t0, 300*time.Millisecond)
	assert.Equal(t, resultAlice, "Hello, Alice!")
	assert.Equal(t, resultBob, "Hello, Bob!")
}

func TestFuturesConcurrent(t *testing.T) {
	// Test concurrent futures.
	var wg sync.WaitGroup

	t0 := time.Now()
	wg.Add(1)
	go func() {
		defer wg.Done()
		result, err := SayAfter(bg(), 100, "Alice")
		assert.NoError(t, err)
		assert.Equal(t, result, "Hello, Alice!")
	}()

	wg.Add(1)
	go func() {
		defer wg.Done()
		result, err := SayAfter(bg(), 200, "Bob")
		assert.NoError(t, err)
		assert.Equal(t, result, "Hello, Bob!")
	}()

	wg.Wait()

	assertDelayedExecution(t, t0, 200*time.Millisecond)
}

func TestFuturesAsyncMethods(t *testing.T) {
	// Test async methods
	megaphone, err := NewMegaphone(bg())
	assert.NoError(t, err)

	t0 := time.Now()
	resultAlice, err := megaphone.SayAfter(bg(), 200, "Alice")
	assert.NoError(t, err)

	assertDelayedExecution(t, t0, 200*time.Millisecond)
	assert.Equal(t, resultAlice, "HELLO, ALICE!")
}

func TestFuturesAsyncConstructor(t *testing.T) {
	// Async constructors are supported
	megaphone, err := NewMegaphone(bg())
	assert.NoError(t, err)
	assert.NotNil(t, megaphone)

	megaphone, err = MegaphoneSecondary(bg())
	assert.NoError(t, err)
	t0 := time.Now()
	msg, err := megaphone.SayAfter(bg(), 20, "Alice")
	assert.NoError(t, err)
	assertDelayedExecution(t, t0, 20*time.Millisecond)
	assert.Equal(t, "HELLO, ALICE!", msg)

	udl_megaphone, err := MegaphoneSecondary(bg())
	assert.NoError(t, err)
	t0 = time.Now()
	msg, err = udl_megaphone.SayAfter(bg(), 25, "udl")
	assert.NoError(t, err)
	assertDelayedExecution(t, t0, 25*time.Millisecond)
	assert.Equal(t, "HELLO, UDL!", msg)
}

func TestFuturesAsyncTraitMethods(t *testing.T) {
	traits := GetSayAfterTraits()
	t0 := time.Now()

	res1, err := traits[0].SayAfter(bg(), 100, "Alice")
	assert.NoError(t, err)
	res2, err := traits[1].SayAfter(bg(), 100, "Bob")
	assert.NoError(t, err)

	assertDelayedExecution(t, t0, 200*time.Millisecond)
	assert.Equal(t, "Hello, Alice!", res1)
	assert.Equal(t, "Hello, Bob!", res2)
}

func TestFuturesAsyncUdlTraitMethods(t *testing.T) {
	traits := GetSayAfterUdlTraits()
	t0 := time.Now()

	res1, err := traits[0].SayAfter(bg(), 100, "Alice")
	assert.NoError(t, err)
	res2, err := traits[1].SayAfter(bg(), 100, "Bob")
	assert.NoError(t, err)

	assertDelayedExecution(t, t0, 200*time.Millisecond)
	assert.Equal(t, "Hello, Alice!", res1)
	assert.Equal(t, "Hello, Bob!", res2)
}

func TestFuturesRecordAndEnumTraits(t *testing.T) {
	record1 := MakeTraitRecord("first", 1)
	record2 := MakeTraitRecord("first", 1)
	record3 := MakeTraitRecord("second", 2)

	assert.Equal(t, "first:1", record1.String())
	assert.Equal(t, "TraitRecord { label: \"first\", rank: 1 }", record1.DebugString())
	assert.True(t, record1.Eq(record2))
	assert.False(t, record1.Eq(record3))
	assert.True(t, record1.Ne(record3))
	assert.Equal(t, record1.Hash(), record2.Hash())
	assert.Equal(t, int8(-1), record1.Cmp(record3))
	assert.Equal(t, int8(0), record1.Cmp(record2))

	enum1 := MakeTraitEnum(0)
	enum2 := MakeTraitEnum(0)
	enum3 := MakeTraitEnum(1)

	assert.Equal(t, "alpha", enum1.String())
	assert.Equal(t, "Alpha", enum1.DebugString())
	assert.True(t, enum1.Eq(enum2))
	assert.False(t, enum1.Eq(enum3))
	assert.True(t, enum1.Ne(enum3))
	assert.Equal(t, enum1.Hash(), enum2.Hash())
	assert.Equal(t, int8(-1), enum1.Cmp(enum3))
	assert.Equal(t, int8(0), enum1.Cmp(enum2))
	assert.Equal(t, TraitRecord{Label: "first-bonus", Rank: 1}, record1.Relabel("bonus"))
	assert.Equal(t, uint8(6), record1.Score(5))
	assert.Equal(t, TraitEnumBeta, enum1.Flipped())
	assert.Equal(t, TraitEnumAlpha, enum3.Flipped())
	assert.Equal(t, "alpha", enum1.Label())
	assert.Equal(t, "beta", enum3.Label())
}

type goAsyncParser struct {
	completedDelays uint
}

func (gap *goAsyncParser) AsString(ctx context.Context, delayMs int32, value int32) (string, error) {
	time.Sleep(time.Duration(delayMs) * time.Millisecond)
	return fmt.Sprintf("%d", value), nil
}

func (gap *goAsyncParser) TryFromString(ctx context.Context, delayMs int32, value string) (int32, error) {
	time.Sleep(time.Duration(delayMs) * time.Millisecond)
	if value == "force-unexpected-exception" {
		return 0, NewParserErrorUnexpectedError()
	}
	val, err := strconv.ParseInt(value, 10, 32)
	if err != nil {
		return int32(val), NewParserErrorNotAnInt()
	}
	return int32(val), nil
}

func (gap *goAsyncParser) Delay(ctx context.Context, delayMs int32) error {
	time.Sleep(time.Duration(delayMs) * time.Millisecond)
	gap.completedDelays += 1
	return nil
}

func (gap *goAsyncParser) TryDelay(ctx context.Context, delayMs string) error {
	ms, err := strconv.ParseInt(delayMs, 10, 32)
	if err != nil {
		return NewParserErrorNotAnInt()
	}

	time.Sleep(time.Duration(ms) * time.Millisecond)
	gap.completedDelays += 1
	return nil
}

func TestFuturesForeignAsyncTraitMethods(t *testing.T) {
	traitObj := &goAsyncParser{}

	t0 := time.Now()
	got, err := AsStringUsingTrait(bg(), traitObj, 10, 42)
	assert.NoError(t, err)
	assert.Equal(t, "42", got)
	val, err := TryFromStringUsingTrait(bg(), traitObj, 10, "42")
	if assert.Nil(t, err) {
		assert.Equal(t, int32(42), val)
	}
	assertDelayedExecution(t, t0, 20*time.Millisecond)
	val, err = TryFromStringUsingTrait(bg(), traitObj, 0, "fourty-two")
	assert.ErrorIs(t, err, ErrParserErrorNotAnInt)
	val, err = TryFromStringUsingTrait(bg(), traitObj, 0, "force-unexpected-exception")
	assert.ErrorIs(t, err, ErrParserErrorUnexpectedError)

	t0 = time.Now()
	assert.NoError(t, DelayUsingTrait(bg(), traitObj, 15))
	assert.NoError(t, TryDelayUsingTrait(bg(), traitObj, "15"))
	assertDelayedExecution(t, t0, 30*time.Millisecond)
	assert.Equal(t, uint(2), traitObj.completedDelays)

	assert.NoError(t, CancelDelayUsingTrait(bg(), traitObj, 10))
	// Sleep a bit longer then a delay to confirm that task was acutaly cancled
	time.Sleep(20)
	assert.Equal(t, uint(2), traitObj.completedDelays)
}

func TestFuturesForeignTraitRecordRoundtrip(t *testing.T) {
	traitObj := &goAsyncParser{}
	holder := RoundtripParserHolder(ParserHolder{Parser: traitObj})

	got, err := holder.Parser.AsString(bg(), 0, 7)
	assert.NoError(t, err)
	assert.Equal(t, "7", got)
	holder.Destroy()

	assert.Equal(t, uint(0), traitObj.completedDelays)
}

func TestFuturesAsyncObjectParam(t *testing.T) {
	megaphone, err := NewMegaphone(bg())
	assert.NoError(t, err)
	t0 := time.Now()
	resultAlice, err := SayAfterWithMegaphone(bg(), megaphone, 200, "Alice")
	assert.NoError(t, err)

	assertDelayedExecution(t, t0, 200*time.Millisecond)
	assert.Equal(t, resultAlice, "HELLO, ALICE!")
}

func TestFuturesAsyncFunctionRetObject(t *testing.T) {
	// Test async function returning an object
	megaphone, err := AsyncNewMegaphone(bg())
	assert.NoError(t, err)

	result, err := megaphone.FallibleMe(bg(), false)
	assert.Nil(t, err)
	assert.Equal(t, result, uint8(42))
}

func TestFuturesTokio(t *testing.T) {
	// Test with the Tokio runtime.
	t0 := time.Now()
	resultAlice, err := SayAfterWithTokio(bg(), 200, "Alice")
	assert.NoError(t, err)

	assertDelayedExecution(t, t0, 200*time.Millisecond)
	assert.Equal(t, resultAlice, "Hello, Alice (with Tokio)!")
}

func TestFuturesFallibleNoThrow(t *testing.T) {
	// Test fallible function/method…
	// … which doesn't throw.
	{
		t0 := time.Now()
		result, err := FallibleMe(bg(), false)

		assertInstantExecution(t, t0)
		assert.Nil(t, err)
		assert.Equal(t, result, uint8(42))

	}
	{
		m, err := FallibleStruct(bg(), false)
		assert.Nil(t, err)
		result, err := m.FallibleMe(bg(), false)
		assert.Nil(t, err)
		assert.Equal(t, result, uint8(42))
	}
	{
		megaphone, err := NewMegaphone(bg())
		assert.NoError(t, err)

		t0 := time.Now()
		result, err := megaphone.FallibleMe(bg(), false)
		assert.Nil(t, err)

		assertInstantExecution(t, t0)
		assert.Equal(t, result, uint8(42))
	}
}

func TestFuturesFallibleThrows(t *testing.T) {
	// … which does throw.
	{
		t0 := time.Now()

		_, err := FallibleMe(bg(), true)
		assert.EqualError(t, err, "MyError: Foo")
		assertInstantExecution(t, t0)

		_, err = FallibleStruct(bg(), true)
		assert.EqualError(t, err, "MyError: Foo")
	}
	{
		megaphone, err := NewMegaphone(bg())
		assert.NoError(t, err)

		t0 := time.Now()
		_, err = megaphone.FallibleMe(bg(), true)

		assertInstantExecution(t, t0)
		assert.EqualError(t, err, "MyError: Foo")
	}
}

func TestFuturesLockAndCancel(t *testing.T) {
	// Test a future that uses a lock and that is cancelled.
	ctx, cancel := context.WithCancel(bg())
	go func() {
		_ = UseSharedResource(ctx, SharedResourceOptions{ReleaseAfterMs: 100, TimeoutMs: 1000})
	}()

	// Wait some time to ensure the task has locked the shared resource
	time.Sleep(50 * time.Millisecond)

	// Cancel the job so the shared resource is released.
	cancel()

	// Try accessing the shared resource again.  The initial task should release the shared resource
	// before the timeout expires.
	assert.NoError(t, UseSharedResource(bg(), SharedResourceOptions{ReleaseAfterMs: 0, TimeoutMs: 1000}))
}

func TestFuturesLockAndNotCancel(t *testing.T) {
	// Test a future that uses a lock and that is not cancelled.
	go func() {
		_ = UseSharedResource(bg(), SharedResourceOptions{ReleaseAfterMs: 100, TimeoutMs: 1000})
	}()

	assert.NoError(t, UseSharedResource(bg(), SharedResourceOptions{ReleaseAfterMs: 0, TimeoutMs: 1000}))
}

func TestFuturesCancelSleep(t *testing.T) {
	runtime.GC()
	time.Sleep(20 * time.Millisecond)
	before := runtime.NumGoroutine()

	ctx, cancel := context.WithCancel(bg())
	errCh := make(chan error, 1)
	start := time.Now()
	go func() {
		_, err := Sleep(ctx, 5000)
		errCh <- err
	}()

	time.Sleep(20 * time.Millisecond)
	cancel()

	select {
	case err := <-errCh:
		assert.ErrorIs(t, err, context.Canceled)
		assert.Less(t, time.Since(start), 500*time.Millisecond)
	case <-time.After(time.Second):
		t.Fatal("cancelling ctx did not abort Sleep promptly")
	}

	runtime.GC()
	time.Sleep(50 * time.Millisecond)
	after := runtime.NumGoroutine()
	assert.LessOrEqual(t, after, before+2, "goroutines leaked: before=%d after=%d", before, after)
}

func TestFuturesCancelAlreadyDone(t *testing.T) {
	ctx, cancel := context.WithCancel(bg())
	cancel()
	start := time.Now()
	_, err := Sleep(ctx, 5000)
	assert.ErrorIs(t, err, context.Canceled)
	assert.Less(t, time.Since(start), 50*time.Millisecond)
}

func TestFuturesCancelDeadline(t *testing.T) {
	ctx, cancel := context.WithTimeout(bg(), 30*time.Millisecond)
	defer cancel()
	start := time.Now()
	_, err := Sleep(ctx, 5000)
	assert.ErrorIs(t, err, context.DeadlineExceeded)
	assert.Less(t, time.Since(start), 500*time.Millisecond)
}
