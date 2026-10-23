package binding_tests

import (
	"context"
	"testing"
	"time"

	. "github.com/NordSecurity/uniffi-bindgen-go/binding_tests/generated/uniffi_example_futures"

	"github.com/stretchr/testify/assert"
)

func TestSayAfter(t *testing.T) {
	start := time.Now()
	msg, err := SayAfter(context.Background(), 20, "Alice")
	elapsed := time.Since(start)
	assert.NoError(t, err)
	assert.Equal(t, "Hello, Alice!", msg)
	assert.GreaterOrEqual(t, elapsed, 20*time.Millisecond)
}
