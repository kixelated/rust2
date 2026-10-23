{#/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */#}

const (
	uniffiRustFuturePollReady      int8 = 0
	uniffiRustFuturePollMaybeReady int8 = 1
	uniffiRustCallStatusCancelled  int8 = 3
)

type rustFuturePollFunc func(C.uint64_t, C.UniffiRustFutureContinuationCallback, C.uint64_t)
type rustFutureCompleteFunc[T any] func(C.uint64_t, *C.RustCallStatus) T
type rustFutureFreeFunc func(C.uint64_t)

//export {{ config|future_continuation_name }}
func {{ config|future_continuation_name }}(data C.uint64_t, pollResult C.int8_t) {
	h := cgo.Handle(uintptr(data))
	waiter := h.Value().(chan int8)
	waiter <- int8(pollResult)
}

func uniffiErrorFromRust[E any](err E) error {
	value := reflect.ValueOf(err)
	if !value.IsValid() || value.IsZero() {
		return nil
	}
	if native, ok := any(err).(NativeError); ok {
		return native.AsError()
	}
	if e, ok := any(err).(error); ok {
		return e
	}
	return fmt.Errorf("%v", err)
}

func uniffiCompleteRustFuture[E any, T any, F any](
	errConverter BufReader[E],
	completeFunc rustFutureCompleteFunc[F],
	liftFunc func(F) T,
	rustFuture C.uint64_t,
) (T, error) {
	var goValue T
	var status C.RustCallStatus
	ffiValue := completeFunc(rustFuture, &status)
	switch int8(status.code) {
	case 0:
		return liftFunc(ffiValue), nil
	case uniffiRustCallStatusCancelled:
		return goValue, nil
	default:
		return goValue, uniffiErrorFromRust(checkCallStatus(errConverter, status))
	}
}

func uniffiRustCallAsync[E any, T any, F any](
	ctx context.Context,
	errConverter BufReader[E],
	completeFunc rustFutureCompleteFunc[F],
	liftFunc func(F) T,
	rustFutureFunc func() C.uint64_t,
	pollFunc rustFuturePollFunc,
	cancelFunc rustFutureFreeFunc,
	freeFunc rustFutureFreeFunc,
) (T, error) {
	var goValue T
	if err := ctx.Err(); err != nil {
		return goValue, err
	}

	rustFuture := rustFutureFunc()
	defer freeFunc(rustFuture)

	pollResult := int8(-1)
	waiter := make(chan int8, 1)
	cancelled := false

	chanHandle := cgo.NewHandle(waiter)
	defer chanHandle.Delete()

	for pollResult != uniffiRustFuturePollReady {
		pollFunc(
			rustFuture,
			(C.UniffiRustFutureContinuationCallback)(C.{{ config|future_continuation_name }}),
			C.uint64_t(chanHandle),
		)
		select {
		case pollResult = <-waiter:
		case <-ctx.Done():
			cancelled = true
			cancelFunc(rustFuture)
			pollResult = <-waiter
		}
	}

	result, err := uniffiCompleteRustFuture(errConverter, completeFunc, liftFunc, rustFuture)
	if cancelled {
		if err := ctx.Err(); err != nil {
			return goValue, err
		}
		return goValue, context.Canceled
	}
	if err != nil {
		return goValue, err
	}
	return result, nil
}

//export {{ config|free_gorutine_callback }}
func {{ config|free_gorutine_callback }}(data C.uint64_t) {
	handle := cgo.Handle(uintptr(data))
	defer handle.Delete()

	guard := handle.Value().(chan struct{})
	guard <- struct{}{}
}
