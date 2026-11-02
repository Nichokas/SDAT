package player

import (
	"time"

	"codeberg.org/dergs/tonearm/internal/player/statemachine"
	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type PlaybackState = statemachine.PlaybackState
type TrackChangedEvent = statemachine.TrackChangedEvent

var PlaybackStateChanged = statemachine.PlaybackStateChanged
var (
	PlaybackStatusPaused  = statemachine.PlaybackStatusPaused
	PlaybackStatusPlaying = statemachine.PlaybackStatusPlaying
	PlaybackStatusStopped = statemachine.PlaybackStatusStopped
)

var PlaybackQualityChanged = statemachine.PlaybackQualityChanged

var TrackChanged = statemachine.TrackChanged

// Holds the user-selected repeat mode.
//
// The signal fires whenever the user changes the repeat mode.
var RepeatModeChanged = signals.NewStatefulSignal[RepeatMode](RepeatModeNone)

// Holds the seed chosen when the user selects shuffle mode.
//
// The signal fires whenever the user toggles the shuffle mode.
// The value is 0 if shuffle mode is disabled.
var ShuffleStateChanged = signals.NewStatefulSignal[bool](false)

var Seeked = signals.NewStatelessSignal[time.Duration]()

var SourceChanged = signals.NewStatefulSignal[tonearm.PlaybackSource](nil)

// Holds the current volume of the player as reported by playbin
//
// The signal fires whenever the volume changes. This could be either due to user input,
// external volume control or MPRIS.
var VolumeChanged = signals.NewStatefulSignal[float64](settings.Player().GetVolume())

type RepeatMode int

const (
	RepeatModeNone RepeatMode = iota
	RepeatModeQueue
	RepeatModeTrack
)
