#include <c_test_support.h>

#include <condition_variable>
#include <mutex>

struct test_latch {
    std::mutex mutex;
    std::condition_variable signalled;
    int count = 0;
};

test_latch *test_latch_new(void) {
    return new test_latch();
}

void test_latch_signal(test_latch *latch) {
    {
        std::lock_guard<std::mutex> guard(latch->mutex);
        latch->count++;
    }
    latch->signalled.notify_all();
}

void test_latch_wait(test_latch *latch) {
    std::unique_lock<std::mutex> lock(latch->mutex);
    latch->signalled.wait(lock, [latch] { return latch->count > 0; });
    latch->count--;
}

void test_latch_free(test_latch *latch) {
    delete latch;
}
