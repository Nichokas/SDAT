package statemachine

import (
	"testing"
	"time"

	"codeberg.org/dergs/tonearm/internal/signals"
	"github.com/go-gst/go-gst/gst"
)

func TestOnStoppedSetsPlaybinState(t *testing.T) {
	previousState := PlaybackStateChanged.CurrentValue()
	t.Cleanup(func() {
		PlaybackStateChanged.Set(previousState)
	})

	playbin := &Testbin{state: gst.StatePlaying}
	if err := OnStopped(playbin); err != nil {
		t.Fatalf("OnStopped() returned an unexpected error: %v", err)
	}

	if playbin.state != gst.StateNull {
		t.Errorf("OnStopped() set playbin state to %v; want %v", playbin.state, gst.StateNull)
	}
}

func TestOnStoppedEmitsPlaybackState(t *testing.T) {
	initialState := PlaybackState{
		Duration: 3 * time.Minute,
		Position: 45 * time.Second,
		Status:   PlaybackStatusPlaying,
		Loading:  true,
	}

	notifiedState := runOnStoppedAndCaptureState(t, &Testbin{}, initialState)
	expectedState := initialState
	expectedState.Status = PlaybackStatusStopped
	expectedState.Loading = false

	if notifiedState != expectedState {
		t.Errorf("OnStopped() notified playback state %+v; want %+v", notifiedState, expectedState)
	}
}

func TestOnStoppedReturnsStateErrorWithoutUpdatingPlaybackState(t *testing.T) {
	previousState := PlaybackStateChanged.CurrentValue()
	t.Cleanup(func() {
		PlaybackStateChanged.Set(previousState)
	})

	initialState := PlaybackState{
		Duration: time.Minute,
		Position: 15 * time.Second,
		Status:   PlaybackStatusPlaying,
		Loading:  true,
	}
	PlaybackStateChanged.Set(initialState)

	wasNotified := false
	subscription := PlaybackStateChanged.OnLazy(func(PlaybackState) bool {
		wasNotified = true
		return signals.Unsubscribe
	})
	t.Cleanup(func() {
		PlaybackStateChanged.Unsubscribe(subscription)
	})

	if err := OnStopped(&Testbin{stateShouldError: true}); err == nil {
		t.Fatal("OnStopped() returned nil; want a playbin state error")
	}

	if wasNotified {
		t.Error("OnStopped() notified PlaybackStateChanged after the playbin state failed")
	}

	if currentState := PlaybackStateChanged.CurrentValue(); currentState != initialState {
		t.Errorf("OnStopped() changed playback state to %+v after failure; want %+v", currentState, initialState)
	}
}

func TestOnStoppedStopClearsCurrentTrack(t *testing.T) {
	previousTrack := TrackChanged.CurrentValue()
	t.Cleanup(func() {
		TrackChanged.Set(previousTrack)
	})
	TrackChanged.Set(&TrackChangedEvent{})

	var (
		notifiedTrack *TrackChangedEvent
		wasNotified   bool
	)
	subscription := TrackChanged.OnLazy(func(track *TrackChangedEvent) bool {
		notifiedTrack = track
		wasNotified = true
		return signals.Unsubscribe
	})
	t.Cleanup(func() {
		TrackChanged.Unsubscribe(subscription)
	})

	if err := OnStoppedStop(); err != nil {
		t.Fatalf("OnStoppedStop() returned an unexpected error: %v", err)
	}

	if !wasNotified {
		t.Fatal("OnStoppedStop() did not notify TrackChanged")
	}

	if notifiedTrack != nil {
		t.Errorf("OnStoppedStop() notified track %+v; want nil", notifiedTrack)
	}
}

func runOnStoppedAndCaptureState(t *testing.T, playbin *Testbin, initialState PlaybackState) PlaybackState {
	t.Helper()

	previousState := PlaybackStateChanged.CurrentValue()
	t.Cleanup(func() {
		PlaybackStateChanged.Set(previousState)
	})
	PlaybackStateChanged.Set(initialState)

	var (
		notifiedState PlaybackState
		wasNotified   bool
	)
	subscription := PlaybackStateChanged.OnLazy(func(state PlaybackState) bool {
		notifiedState = state
		wasNotified = true
		return signals.Unsubscribe
	})
	t.Cleanup(func() {
		PlaybackStateChanged.Unsubscribe(subscription)
	})

	if err := OnStopped(playbin); err != nil {
		t.Fatalf("OnStopped() returned an unexpected error: %v", err)
	}

	if !wasNotified {
		t.Fatal("OnStopped() did not notify PlaybackStateChanged")
	}

	return notifiedState
}
