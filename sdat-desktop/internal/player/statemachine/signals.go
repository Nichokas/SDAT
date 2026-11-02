package statemachine

import (
	"time"

	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

// Holds the current playback state including the expected duration and playing position of the
// currently playing track, if any.
//
// This signal fires when the player changes between playback states such as buffering, playing, paused, or stopped.
// This signal also fires whenever the expected track duration changes or at least every 250 milliseconds during playback.
var PlaybackStateChanged = signals.NewStatefulSignal[PlaybackState](PlaybackState{Status: PlaybackStatusStopped})

// PlaybackStatus is used to report the intended player state
// This can differ from the actual state machine state during playback transitions.
type PlaybackStatus string

const (
	// The player is currently playing a track or is intending to play a track
	// without further user intervention
	PlaybackStatusPlaying PlaybackStatus = "playing"

	// The player is currently paused and will not play any track, but has the
	// option to resume track playback when the user interacts with the player.
	PlaybackStatusPaused PlaybackStatus = "paused"

	// The player is currently stopped and will not play any track. Playback
	// cannot be resumed. The user must start playback from the beginning.
	PlaybackStatusStopped PlaybackStatus = "stopped"
)

// PlaybackState holds the current playback state including the expected duration and playing position of the
// currently playing track, if any.
type PlaybackState struct {
	// The effective duration of the currently playing track as reported by playbin
	// This may differ from the advertised track duration, e.g. during unauthenticated demo playback
	Duration time.Duration

	// The current position of the playback as reported by playbin
	Position time.Duration

	// The current playback status
	Status PlaybackStatus

	// Whether the player is currently loading a track
	Loading bool
}

// Holds the definitive playback quality of the currently playing or last played track
// This can be lower than the chosen quality depending on the tracks original quality
// or any other reason TIDAL's API may have for offering a lower quality.
//
// The signal fires shortly after the new track information has been broadcasted on
// the TrackChanged signal. The signal may fire either before the track starts playing
// in case of a cold start or extremely shortly after a track has started playing in
// case of gapless playback.
var PlaybackQualityChanged = signals.NewStatefulSignal[tonearm.TrackQuality](tonearm.TrackQualityHighResLossless)

// Holds the relevant information about the currently playing or last played track.
// This can be nil if no track is currently playing. This is especially the case when
// the player has just been created.
//
// The signal fires after the new track information has been retrieved from the TIDAL API
// but always before the track starts playing.
var TrackChanged = signals.NewStatefulSignal[*TrackChangedEvent](nil)

type TrackChangedEvent struct {
	tonearm.Track

	// SkipHistory indicates whether the track should create a new entry in the playback history or not.
	SkipHistory bool
}

var EnqueuedTrack = signals.NewStatefulSignal[tonearm.TrackManifest](nil)
