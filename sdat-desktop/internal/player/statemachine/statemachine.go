package statemachine

import (
	"time"

	"github.com/go-gst/go-gst/gst"
)

type State = string

type Trigger = string

const (
	// This trigger is used to signal that the transition has failed
	// and the state machine should transition to a fallback state.
	TRIGGER_FAILED Trigger = "TRIGGER_FAILED"
)

type Playbin interface {
	SeekTime(position time.Duration, flag gst.SeekFlags) bool
	Set(name string, value interface{}) error
	SetArg(key string, value string)
	SetState(state gst.State) error
	QueryDuration(format gst.Format) (bool, int64)
	QueryPosition(format gst.Format) (bool, int64)
}
