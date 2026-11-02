package statemachine

import (
	"github.com/go-gst/go-gst/gst"
)

const (
	// This is the default state for the state machine to be in after being initialized
	// or when no playback is happening / scheduled. From there, the state machine has
	// multiple ways to go.
	STATE_STOPPED State = "STOPPED"

	// This trigger is used to signal that the player should stop playback.
	TRIGGER_STOP Trigger = "TRIGGER_STOP"
)

func OnStopped(playbin Playbin) error {
	// When entering the stopped state, we want to ensure playbin is definitely
	// unable to continue playback by setting it to null state.
	if err := playbin.SetState(gst.StateNull); err != nil {
		return err
	}

	// Notify the playback state changed signal that we are now stopped.
	PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
		state.Loading = false
		state.Status = PlaybackStatusStopped
		return state
	})

	return nil
}

func OnStoppedStop() error {
	// Notify that there is no current track being played.
	TrackChanged.Set(nil)

	// Remove metadata from the playback state.
	PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
		state.Duration = 0
		state.Position = 0
		return state
	})
	return nil
}
