package statemachine

import (
	"testing"

	"codeberg.org/dergs/tonearm/internal/signals"
	"github.com/go-gst/go-gst/gst"
)

func TestOnPausedSetsPlaybinState(t *testing.T) {
	playbin := &Testbin{state: gst.StatePlaying}

	if err := OnPaused(playbin); err != nil {
		t.Fatalf("OnPaused() returned an unexpected error: %v", err)
	}

	if playbin.state != gst.StatePaused {
		t.Errorf("OnPaused() set playbin state to %v; want %v", playbin.state, gst.StatePaused)
	}
}

func TestOnPausedUpdatesPlaybackState(t *testing.T) {
	previousState := PlaybackStateChanged.CurrentValue()
	t.Cleanup(func() {
		PlaybackStateChanged.Set(previousState)
	})

	initialState := PlaybackState{Status: PlaybackStatusPlaying}
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

	if err := OnPaused(&Testbin{}); err != nil {
		t.Fatalf("OnPaused() returned an unexpected error: %v", err)
	}

	if !wasNotified {
		t.Fatal("OnPaused() did not notify PlaybackStateChanged")
	}

	if notifiedState.Status != PlaybackStatusPaused {
		t.Errorf("OnPaused() notified playback status %q; want %q", notifiedState.Status, PlaybackStatusPaused)
	}
}
