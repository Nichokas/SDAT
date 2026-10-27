package player

import (
	"math"
	"time"

	"codeberg.org/dergs/tonearm/internal/player/statemachine"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"github.com/go-gst/go-gst/gst"
)

func CycleRepeatMode() {
	switch RepeatModeChanged.CurrentValue() {
	case RepeatModeNone:
		SetRepeatMode(RepeatModeQueue)
	case RepeatModeQueue:
		SetRepeatMode(RepeatModeTrack)
	case RepeatModeTrack:
		SetRepeatMode(RepeatModeNone)
	}
}

func Next() {
	nextTrack, _, err := nextTrack(false)
	if err != nil {
		logger.Error("failed to get next track", "error", err)
		return
	}
	if nextTrack == nil {
		logger.Debug("no next track in queue, stopping playback", "action", "next")
		Stop()
		return
	}
	stateMachine.Fire(statemachine.TRIGGER_PLAY_TRACK, nextTrack, false)
}

func Pause() {
	logger.Debug("player controls requested to pause")
	stateMachine.Fire(statemachine.TRIGGER_PAUSE)
}

func Play() {
	logger.Debug("player controls requested to start playing")
	stateMachine.Fire(statemachine.TRIGGER_RESUME)
}

func PlayPause() {
	logger.Debug("player controls requested to start playing or pause")
	switch PlaybackStateChanged.CurrentValue().Status {
	case PlaybackStatusPlaying:
		Pause()
	case PlaybackStatusPaused:
		Play()
	case PlaybackStatusStopped:
		Play()
	}
}

func Previous() {
	ok, position := playbin.QueryPosition(gst.FormatTime)
	if (ok && time.Duration(position) > 5*time.Second) || History.Peek() == nil {
		logger.Debug("above the 5 second mark, replaying song", "action", "previous")
		SeekToPosition(0, false)
		return
	}

	previousTrack, err := previousTrack(false)
	if err != nil {
		logger.Error("failed to get previous track", "error", err)
		return
	}
	currentTrack := tonearm.Track(TrackChanged.CurrentValue())
	if currentTrack != nil {
		UserQueue.Prepend(currentTrack)
	}

	stateMachine.Fire(statemachine.TRIGGER_PLAY_TRACK, previousTrack, false)
}

func SeekToPercent(percent float64) {
	if percent < 0 || percent > 100 {
		percent = math.Max(0, math.Min(100, percent))
		logger.Warn("percent must be between 0 and 100, clamping to nearest value", "percent", percent)
	}

	position := float64(PlaybackStateChanged.CurrentValue().Duration) * (percent / 100.0)
	SeekToPosition(time.Duration(int64(position)), false)
}

func SeekToPosition(position time.Duration, precise bool) {
	logger.Debug("player controls requested to seek to position", "position", position)
	if precise {
		playbin.SeekTime(position, gst.SeekFlagFlush|gst.SeekFlagAccurate)
	} else {
		playbin.SeekTime(position, gst.SeekFlagFlush|gst.SeekFlagKeyUnit)
	}
	Seeked.Notify(position)
}

func SetVolume(volume float64) {
	if volume < 0 {
		logger.Info("Volume is lower than 0, overriding back to 0.")
		volume = 0
	} else if volume > 1 {
		logger.Warn("Volume is higher than 1. This will cause overdrive to the speakers.")
	}
	if playbin != nil {
		playbin.SetProperty("volume", volume)
	}
}

func Stop() {
	logger.Debug("player controls requested to stop")
	SourceChanged.Set(nil)
	stateMachine.Fire(statemachine.TRIGGER_STOP)
}

func SetRepeatMode(m RepeatMode) {
	if m == RepeatModeChanged.CurrentValue() {
		return
	}

	RepeatModeChanged.Notify(func(oldValue RepeatMode) RepeatMode {
		return m
	})
}

func SetShuffle(shuffle bool, currentTrackID string) {
	if shuffle {
		BaseQueue.Shuffle(currentTrackID)
	} else {
		BaseQueue.Restore(currentTrackID)
	}
	ShuffleStateChanged.Set(shuffle)
}

func ToggleShuffle() {
	currentTrack := ""
	if current := TrackChanged.CurrentValue(); current != nil {
		currentTrack = current.ID()
	}
	SetShuffle(!BaseQueue.IsShuffled(), currentTrack)
}
