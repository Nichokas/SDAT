package statemachine

import (
	"fmt"
	"time"

	"github.com/go-gst/go-gst/gst"
)

type Testbin struct {
	duration            int64
	durationShouldError bool

	position            int64
	positionShouldError bool

	volume            int64
	volumeShouldError bool

	state            gst.State
	stateShouldError bool

	uri            string
	uriShouldError bool
}

func (t *Testbin) SeekTime(position time.Duration, flag gst.SeekFlags) bool {
	t.position = int64(position)
	return !t.positionShouldError
}

func (t *Testbin) Set(name string, value interface{}) error {
	switch name {
	case "volume":
		if t.volumeShouldError {
			return fmt.Errorf("set volume failed")
		}
		t.volume = int64(value.(float64))
	}
	return nil
}

func (t *Testbin) SetArg(key string, value string) {
	switch key {
	case "uri":
		t.uri = value
	}
}

func (t *Testbin) SetState(state gst.State) error {
	if t.stateShouldError {
		return fmt.Errorf("set state failed")
	}

	t.state = state
	return nil
}

func (t *Testbin) QueryDuration(format gst.Format) (bool, int64) {
	return !t.durationShouldError, t.duration
}

func (t *Testbin) QueryPosition(format gst.Format) (bool, int64) {
	return !t.positionShouldError, t.position
}
