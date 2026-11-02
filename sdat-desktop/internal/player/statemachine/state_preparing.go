package statemachine

import (
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"github.com/go-gst/go-gst/gst"
)

const (
	// This state is active as long as we are before actual sound comes
	// out of the speakers. Playback is not yet started or stopped.
	STATE_PREPARING State = "PREPARING"

	// Used whenever the user or app logic requests to play a
	// different track by ID.
	//
	// Trigger Arguments:
	// - TrackID (String): The ID of the track to be resolved and played
	// - SkipHistory (Bool): Whether the track should not create an entry in the playback history
	TRIGGER_PLAY_TRACK_ID Trigger = "TRIGGER_PLAY_TRACK_ID"

	// Used whenever the user or app logic requests to play a
	// different track.
	//
	// Trigger Arguments:
	// - Track (tonearm.Track): A reference to the track to be played
	// - SkipHistory (Bool): Whether the track should not create an entry in the playback history
	TRIGGER_PLAY_TRACK Trigger = "TRIGGER_PLAY_TRACK"
)

func OnPreparing() error {
	// Whenever we are in the PREPARING state, we set the loading flag to true
	// to indicate that the player is preparing to play a track.
	PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
		state.Loading = true
		return state
	})

	return nil
}

func OnPlayTrackID(playbin Playbin, service interface {
	tonearm.TrackProvider
	tonearm.TrackManifestProvider
}, trackID string, skipHistory bool) error {
	track, err := service.GetTrack(trackID)
	if err != nil {
		return err
	}

	return OnPlayTrack(playbin, service, track, skipHistory)
}

func OnPlayTrack(playbin Playbin, provider tonearm.TrackManifestProvider, track tonearm.Track, skipHistory bool) error {
	// Ensure playbin is prepared to handle a track change. If something is playing, we want to stop it.
	if err := playbin.SetState(gst.StateReady); err != nil {
		return err
	}

	trackManifest, err := fetchAndEnqueueTrack(playbin, provider, track.ID())
	if err != nil {
		return err
	}

	// Since we previously stopped playback, we now have to tell playbin to resume playback.
	// Otherwise, it will remain in the Ready state and not play anything.
	if err := playbin.SetState(gst.StatePlaying); err != nil {
		return err
	}

	// Now that we are reasonably sure that playback is going to be successful,
	// we can notify the rest of the application about the new playback information.
	TrackChanged.Set(&TrackChangedEvent{track, skipHistory})

	PlaybackStateChanged.Notify(func(state PlaybackState) PlaybackState {
		state.Duration = track.Duration()
		return state
	})

	PlaybackQualityChanged.Notify(func(oldValue tonearm.TrackQuality) tonearm.TrackQuality {
		return trackManifest.Quality()
	})

	return nil
}
