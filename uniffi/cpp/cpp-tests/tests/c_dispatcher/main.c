/*
 * The generated C API on a host-owned dispatcher: work only runs when the test drains its
 * queue, so cancellation and delivery are checked without timing. This fixture wakes every
 * future from a call on this thread, so the queue needs no lock.
 */
#include <c_test_support.h>

#include <capi.h>

#include <string.h>

#define QUEUE_CAPACITY 64

static CapiWork *queue[QUEUE_CAPACITY];
static size_t queued;
static bool accepting = true;
static int shutdowns;

static bool dispatch(void *user_data, CapiWork *work) {
    CHECK(user_data == &queued);
    if (!accepting) {
        return false;
    }
    CHECK(queued < QUEUE_CAPACITY);
    queue[queued++] = work;
    return true;
}

/* Runs the oldest queued work. */
static void run_one(void) {
    CapiWork *work;
    CHECK(queued > 0);
    work = queue[0];
    queued--;
    memmove(queue, queue + 1, queued * sizeof queue[0]);
    capi_work_run(work);
}

/* Runs queued work, including work that running it queues. Returns how many ran. */
static size_t drain(void) {
    size_t ran = 0;
    while (queued > 0) {
        run_one();
        ran++;
    }
    return ran;
}

static void stop(void *user_data) {
    CHECK(user_data == &queued);
    accepting = false;
    drain();
    shutdowns++;
}

typedef struct {
    int calls;
    int cancels;
    uint64_t value;
    CapiError *error;
    CapiTask *task;
    /* Another call's task, which the callback frees. */
    CapiTask *other;
} result;

static void on_sum(void *user_data, bool cancelled, uint64_t value) {
    result *r = (result *)user_data;
    r->calls++;
    r->cancels += cancelled;
    r->value = value;
}

static void on_next(void *user_data, bool cancelled, CapiError *error, uint64_t value) {
    result *r = (result *)user_data;
    r->calls++;
    r->cancels += cancelled;
    r->error = error;
    r->value = value;
}

static void on_wait(void *user_data, bool cancelled) {
    result *r = (result *)user_data;
    r->calls++;
    r->cancels += cancelled;
}

/* Frees its own task, and the other one, from inside the callback. */
static void on_sum_free(void *user_data, bool cancelled, uint64_t value) {
    result *r = (result *)user_data;
    r->calls++;
    r->cancels += cancelled;
    r->value = value;
    capi_task_free(r->task);
    r->task = NULL;
    capi_task_free(r->other);
    r->other = NULL;
}

int main(void) {
    const uint64_t values[] = {1, 2, 3};
    CapiU64List list;
    CapiDispatcher dispatcher;
    CapiStore *store;
    result r;

    dispatcher.dispatch = dispatch;
    dispatcher.shutdown = stop;
    dispatcher.user_data = &queued;
    capi_set_dispatcher(&dispatcher);

    store = capi_store_new("loop");
    list.data = values;
    list.len = 3;

    /* A future ready on its first poll still calls back only from the host's loop. */
    memset(&r, 0, sizeof r);
    r.task = capi_sum(&list, on_sum, &r);
    CHECK(r.calls == 0);
    CHECK(drain() > 0);
    CHECK(r.calls == 1 && r.value == 6);
    capi_task_free(r.task);

    /* A pending future calls back once woken and drained. */
    memset(&r, 0, sizeof r);
    r.task = capi_store_next(store, on_next, &r);
    drain();
    CHECK(r.calls == 0);
    capi_store_push(store, 11);
    CHECK(r.calls == 0);
    drain();
    CHECK(r.calls == 1 && r.error == NULL && r.value == 11);
    capi_task_free(r.task);

    /* Freeing a pending task drops the Rust future, then calls back once, cancelled, from the
     * host's loop. */
    memset(&r, 0, sizeof r);
    r.task = capi_store_wait_forever(store, on_wait, &r);
    CHECK(capi_pending_count() == 1);
    capi_task_free(r.task);
    CHECK(r.calls == 0);
    drain();
    CHECK(capi_pending_count() == 0);
    CHECK(r.calls == 1 && r.cancels == 1);

    /* Freeing a task whose result is ready but not yet delivered cancels it instead. */
    memset(&r, 0, sizeof r);
    r.task = capi_sum(&list, on_sum, &r);
    capi_task_free(r.task);
    drain();
    CHECK(r.calls == 1 && r.cancels == 1 && r.value == 0);

    /* Freeing a task after its callback was queued, but before it ran, cancels it once. */
    memset(&r, 0, sizeof r);
    r.task = capi_sum(&list, on_sum, &r);
    CHECK(queued == 1);
    run_one();
    CHECK(queued == 1 && r.calls == 0);
    capi_task_free(r.task);
    drain();
    CHECK(r.calls == 1 && r.cancels == 1 && r.value == 0);

    /* A callback may free its own task, which is no cancel, and another pending task, whose
     * cancel runs later on the loop rather than inside the callback. */
    {
        result other;
        memset(&other, 0, sizeof other);
        memset(&r, 0, sizeof r);
        r.other = capi_store_wait_forever(store, on_wait, &other);
        drain();
        r.task = capi_sum(&list, on_sum_free, &r);
        CHECK(queued == 1);
        run_one();
        CHECK(queued == 1 && r.calls == 0 && other.calls == 0);
        run_one();
        CHECK(r.calls == 1 && r.cancels == 0 && r.value == 6 && r.task == NULL && r.other == NULL);
        CHECK(other.calls == 0);
        drain();
        CHECK(r.calls == 1 && other.calls == 1 && other.cancels == 1);
        CHECK(capi_pending_count() == 0);
    }

    /* Rejected work abandons the result, so freeing the task cancels it. */
    memset(&r, 0, sizeof r);
    accepting = false;
    r.task = capi_sum(&list, on_sum, &r);
    accepting = true;
    CHECK(queued == 0);
    CHECK(r.calls == 0);
    capi_task_free(r.task);
    drain();
    CHECK(r.calls == 1 && r.cancels == 1);

    /* Shutdown drains through the host, then no call calls back, not even cancelled. */
    memset(&r, 0, sizeof r);
    r.task = capi_store_next(store, on_next, &r);
    capi_shutdown_dispatcher();
    CHECK(shutdowns == 1);
    capi_store_push(store, 5);
    CHECK(queued == 0);
    CHECK(r.calls == 0);
    capi_task_free(r.task);

    memset(&r, 0, sizeof r);
    r.task = capi_sum(&list, on_sum, &r);
    CHECK(queued == 0 && r.calls == 0);
    capi_task_free(r.task);

    capi_shutdown_dispatcher();
    CHECK(shutdowns == 1);

    capi_store_free(store);
    return 0;
}
