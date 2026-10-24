/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

package binding_tests

import (
	"context"
	"testing"

	"github.com/NordSecurity/uniffi-bindgen-go/binding_tests/generated/media"
	"github.com/NordSecurity/uniffi-bindgen-go/binding_tests/generated/net"

	"github.com/stretchr/testify/assert"
)

// Every `media` type shares its name with the `net` type it holds or refers to.

func netGroup(sequence uint64, priority net.Priority, payloads ...[]byte) *net.GroupConsumer {
	frames := []net.Frame{}
	for _, payload := range payloads {
		frames = append(frames, net.Frame{Payload: payload})
	}
	return net.NewGroupConsumer(sequence, priority, frames)
}

func TestNamespacesObjectWrapsItsNamesake(t *testing.T) {
	group := media.NewGroupConsumer(netGroup(3, net.PriorityLow, []byte{1, 'a'}, []byte{0, 'b'}))
	assert.Equal(t, uint64(3), group.Group().Sequence())
	assert.Equal(t, media.Priority(media.PriorityNet{Priority: net.PriorityLow}), group.Priority())

	frame, err := group.ReadFrame()
	assert.NoError(t, err)
	assert.Equal(t, media.Frame{Raw: net.Frame{Payload: []byte{1, 'a'}}, Keyframe: true}, frame)
	frame, err = group.ReadFrame()
	assert.NoError(t, err)
	assert.False(t, frame.Keyframe)

	// Each layer's error, both named `GroupError`.
	_, err = group.ReadFrame()
	var mediaErr *media.GroupErrorNet
	assert.ErrorAs(t, err, &mediaErr)
	var netErr *net.GroupErrorClosed
	assert.ErrorAs(t, mediaErr.Source, &netErr)
	assert.Equal(t, uint32(7), netErr.Code)

	_, err = media.Decode(net.Frame{Payload: []byte{}})
	assert.ErrorIs(t, err, media.ErrGroupErrorEmpty)
}

func TestNamespacesAsyncThrowsTheNetError(t *testing.T) {
	group := media.NewGroupConsumer(netGroup(1, net.PriorityHigh, []byte{1}))
	assert.Equal(t, media.Priority(media.PriorityRealtime{}), group.Priority())

	frame, err := group.NextFrame(context.Background())
	assert.NoError(t, err)
	assert.Equal(t, []byte{1}, frame.Raw.Payload)

	_, err = group.NextFrame(context.Background())
	var netErr *net.GroupErrorClosed
	assert.ErrorAs(t, err, &netErr)
	assert.Equal(t, uint32(7), netErr.Code)
}

func TestNamespacesContainersOfBoth(t *testing.T) {
	frame, err := media.Decode(net.Frame{Payload: []byte{0, 'z'}})
	assert.NoError(t, err)
	assert.Equal(t, net.Frame{Payload: []byte{0, 'z'}}, media.Encode(frame))

	frames := media.DecodeAll([]net.Frame{{Payload: []byte{0, 'a'}}, {Payload: []byte{}}, {Payload: []byte{1, 'b'}}})
	assert.Equal(t, 2, len(frames))
	assert.False(t, frames[0].Keyframe)
	assert.True(t, frames[1].Keyframe)

	bounds := media.FindBounds(frames)
	assert.Equal(t, &frames[1], bounds.Keyframe)
	assert.Equal(t, &net.Frame{Payload: []byte{1, 'b'}}, bounds.Last)
	assert.Equal(t, media.Bounds{}, media.FindBounds([]media.Frame{}))

	groups := media.WrapAll([]*net.GroupConsumer{netGroup(1, net.PriorityLow), netGroup(2, net.PriorityHigh)})
	assert.Equal(t, 2, len(groups.Groups))
	assert.Equal(t, uint64(1), groups.Groups[0].Group().Sequence())
	assert.Equal(t, uint64(2), groups.Groups[1].Group().Sequence())
	assert.Equal(t, map[uint64]net.Priority{1: net.PriorityLow, 2: net.PriorityHigh}, groups.Net)
	assert.Equal(t, map[uint64]media.Priority{
		1: media.PriorityNet{Priority: net.PriorityLow},
		2: media.PriorityRealtime{},
	}, groups.Media)
}
