package player

import (
	"fmt"

	"codeberg.org/dergs/tonearm/internal/player/statemachine"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

func nextTrack(peek bool) (tonearm.Track, bool, error) {
	nextTrack := tonearm.Track(statemachine.TrackChanged.CurrentValue())
	repeatMode := RepeatModeChanged.CurrentValue()

	// If we are not repeating the current track, fetch the next streamable track from the queue.
	if repeatMode != RepeatModeTrack {
		// Fetch the next track from the queue, keep fetching until
		// we hit the next streamable track.
		nextTrack = getNextTrackFromQueue(true)
		for nextTrack != nil && !nextTrack.IsStreamable() {
			getNextTrackFromQueue(false)
			nextTrack = getNextTrackFromQueue(true)
		}
		if !peek {
			getNextTrackFromQueue(false)
		}
	}

	return nextTrack, repeatMode == RepeatModeTrack, nil
}

func previousTrack(peek bool) (tonearm.Track, error) {
	currentTrack := statemachine.TrackChanged.CurrentValue()
	repeatMode := RepeatModeChanged.CurrentValue()
	if repeatMode == RepeatModeTrack && currentTrack != nil {
		return currentTrack, nil
	}

	historyTrack := History.Peek()
	if !peek {
		historyTrack = History.Pop()
	}

	if historyTrack != nil {
		return historyTrack, nil
	}

	return nil, fmt.Errorf("no previous track available")
}
