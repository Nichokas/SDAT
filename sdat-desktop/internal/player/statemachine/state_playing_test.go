package statemachine

import (
	"testing"
	"time"

	"codeberg.org/dergs/tonearm/internal/signals"
	"github.com/go-gst/go-gst/gst"
)

func TestOnPlayingSetsPlaybinState(t *testing.T) {
	previousPlaybackState := PlaybackStateChanged.CurrentValue()
	t.Cleanup(func() {
		PlaybackStateChanged.Set(previousPlaybackState)
	})
	t.Cleanup(func() {
		if err := OnPlayingExit(); err != nil {
			t.Errorf("OnPlayingExit() returned an unexpected error: %v", err)
		}
	})

	playbin := &Testbin{state: gst.StatePaused}
	if err := OnPlaying(playbin); err != nil {
		t.Fatalf("OnPlaying() returned an unexpected error: %v", err)
	}

	if playbin.state != gst.StatePlaying {
		t.Errorf("OnPlaying() set playbin state to %v; want %v", playbin.state, gst.StatePlaying)
	}
}

func TestOnPlayingEmitsPlaybackState(t *testing.T) {
	expectedDuration := 3*time.Minute + 12*time.Second
	playbin := &Testbin{duration: int64(expectedDuration)}
	initialState := PlaybackState{
		Duration: 30 * time.Second,
		Position: 12 * time.Second,
		Status:   PlaybackStatusPaused,
		Loading:  true,
	}

	notifiedState := runOnPlayingAndCaptureState(t, playbin, initialState)
	expectedState := initialState
	expectedState.Duration = expectedDuration
	expectedState.Status = PlaybackStatusPlaying
	expectedState.Loading = false

	if notifiedState != expectedState {
		t.Errorf("OnPlaying() notified playback state %+v; want %+v", notifiedState, expectedState)
	}
}

func TestOnPlayingReturnsStateErrorWithoutUpdatingPlaybackState(t *testing.T) {
	previousState := PlaybackStateChanged.CurrentValue()
	t.Cleanup(func() {
		PlaybackStateChanged.Set(previousState)
	})

	initialState := PlaybackState{
		Duration: time.Minute,
		Position: 15 * time.Second,
		Status:   PlaybackStatusPaused,
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
	t.Cleanup(stopWorker)

	if err := OnPlaying(&Testbin{stateShouldError: true}); err == nil {
		t.Fatal("OnPlaying() returned nil; want a playbin state error")
	}

	if wasNotified {
		t.Error("OnPlaying() notified PlaybackStateChanged after the playbin state failed")
	}

	if currentState := PlaybackStateChanged.CurrentValue(); currentState != initialState {
		t.Errorf("OnPlaying() changed playback state to %+v after failure; want %+v", currentState, initialState)
	}
}

func TestOnPlayingPreservesDurationWhenQueryFails(t *testing.T) {
	initialState := PlaybackState{Duration: 2 * time.Minute}
	playbin := &Testbin{
		duration:            int64(5 * time.Minute),
		durationShouldError: true,
	}

	notifiedState := runOnPlayingAndCaptureState(t, playbin, initialState)

	if notifiedState.Duration != initialState.Duration {
		t.Errorf("OnPlaying() notified duration %v after query failure; want %v", notifiedState.Duration, initialState.Duration)
	}
}

func runOnPlayingAndCaptureState(t *testing.T, playbin *Testbin, initialState PlaybackState) PlaybackState {
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
	t.Cleanup(func() {
		if err := OnPlayingExit(); err != nil {
			t.Errorf("OnPlayingExit() returned an unexpected error: %v", err)
		}
	})

	if err := OnPlaying(playbin); err != nil {
		t.Fatalf("OnPlaying() returned an unexpected error: %v", err)
	}

	if !wasNotified {
		t.Fatal("OnPlaying() did not notify PlaybackStateChanged")
	}

	return notifiedState
}
