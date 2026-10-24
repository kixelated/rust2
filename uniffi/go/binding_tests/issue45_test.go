/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package binding_tests

import (
	"context"
	"testing"

	"github.com/NordSecurity/uniffi-bindgen-go/binding_tests/generated/issue45"
	"github.com/stretchr/testify/assert"
)

// Ensure multiple futures packages work fine together, the other one being
// the "futures" fixture from uniffi-rs.
// https://github.com/NordSecurity/uniffi-bindgen-go/issues/45

func TestIssue45(t *testing.T) {
	record, err := issue45.GetAsyncRecord(context.Background())
	assert.NoError(t, err)
	assert.Equal(t, record.Id, "foo")
	assert.Equal(t, record.Tag, "bar")
}
