package statemachine

import (
	"github.com/go-gst/go-gst/gst"
)

const (
	// This state is active as long as the player has a track loaded, but is paused.
	STATE_PAUSED State = "PAUSED"

	// This trigger is used to signal that the player should pause playback.
	TRIGGER_PAUSE Trigger = "TRIGGER_PAUSE"

	// This trigger is used to signal that the player should resume playback.
	TRIGGER_RESUME Trigger = "TRIGGER_RESUME"
)

func OnPaused(playbin Playbin) error {
	if err := playbin.SetState(gst.StatePaused); err != nil {
		return err
	}

	PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
		state.Status = PlaybackStatusPaused
		return state
	})

	return nil
}
