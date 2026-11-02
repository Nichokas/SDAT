package statemachine

import (
	"fmt"
	"time"

	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"github.com/go-gst/go-gst/gst"
)

const (
	// This state is active as long as the player is playing a track and
	// sound is coming out of the speakers.
	STATE_PLAYING State = "PLAYING"

	// This state is active when the player has enqueued a gapless track
	// and is waiting for the gapless transition to complete.
	STATE_PLAYING_GAPLESS State = STATE_PLAYING + ":GAPLESS"

	// This state is active when the player has encountered an issue during
	// playback (e.g. token expired, network error) and is attempting to
	// recover by retrying the track.
	STATE_PLAYING_RETRYING State = STATE_PLAYING + ":RETRYING"

	// This trigger is used to signal that the player is ready for us to
	// enqueue the next track playback url.
	TRIGGER_GAPLESS_HINT Trigger = "TRIGGER_GAPLESS_HINT"

	// This trigger is used to signal that the playback token has likely
	// expired and the player should transition to the RETRYING state.
	TRIGGER_TOKEN_EXPIRED Trigger = "TRIGGER_TOKEN_EXPIRED"
)

func OnPlaying(playbin Playbin) error {
	// To ensure playbin is definitely in the right state, we set it to playing
	// This might not be necessary. (But its cheap insurance)
	if err := playbin.SetState(gst.StatePlaying); err != nil {
		return err
	}

	PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
		// Instead of polling playbin for the duration, we query it once here.
		// This is more efficient AND it solves an issue during gapless playback
		// where the returned value will be updated too early.
		if ok, duration := playbin.QueryDuration(gst.FormatTime); ok {
			state.Duration = time.Duration(duration)
		}
		state.Loading = false
		state.Status = PlaybackStatusPlaying
		return state
	})

	// The playing state owns the update runner, so whenever we enter it
	// we start the update runner to keep the playback state updated.
	startWorker(playbin)
	return nil
}

func OnPlayingExit() error {
	// When we exit the playing state, we stop the update runner as we
	// own its lifecycle.
	stopWorker()
	return nil
}

func OnPlayingGapless(playbin Playbin, provider tonearm.TrackManifestProvider, track tonearm.Track) (tonearm.TrackManifest, error) {
	// If we can't find a streamable track, we return to
	// the default playback state as there is nothing to do.
	if track == nil {
		return nil, fmt.Errorf("no track available for gapless playback")
	}

	return fetchAndEnqueueTrack(playbin, provider, track.ID())
}

func OnPlayingGaplessExit(track tonearm.Track, trackManifest tonearm.TrackManifest, didRepeat bool) error {
	PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
		state.Duration = track.Duration()
		state.Position = 0
		return state
	})

	if didRepeat {
		return nil
	}

	// Since the track has changed, notify listeners of the new track.
	TrackChanged.Set(&TrackChangedEvent{track, false})

	PlaybackQualityChanged.Notify(func(oldValue tonearm.TrackQuality) tonearm.TrackQuality {
		return trackManifest.Quality()
	})

	return nil
}

func OnPlayingRetry(playbin Playbin, provider tonearm.TrackManifestProvider) (time.Duration, error) {
	// When retrying, we need to prepare playbin for a playback url change.
	// Also stopping playback allows us to get a precise position to restore to later.
	if err := playbin.SetState(gst.StateReady); err != nil {
		return 0, err
	}

	if _, err := fetchAndEnqueueTrack(playbin, provider, TrackChanged.CurrentValue().ID()); err != nil {
		return 0, err
	}

	// Fetch the current position so we can restore it after retrying.
	position := PlaybackStateChanged.CurrentValue().Position
	if ok, pos := playbin.QueryPosition(gst.FormatTime); ok {
		position = time.Duration(pos)
	}

	// Now that the updated playback url is enqueued, we can set playbin to playing again.
	if err := playbin.SetState(gst.StatePlaying); err != nil {
		return position, err
	}

	return position, nil
}

func OnPlayingRetryExit(playbin Playbin, position time.Duration) error {
	playbin.SeekTime(position, gst.SeekFlagFlush|gst.SeekFlagAccurate)
	return nil
}
