package statemachine

import (
	"errors"
	"fmt"

	"codeberg.org/dergs/tonearm/internal/settings"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var ErrFetchAndEnqueueTrack = fmt.Errorf("failed to fetch and enqueue track")

func fetchAndEnqueueTrack(playbin Playbin, provider tonearm.TrackManifestProvider, trackId string) (tonearm.TrackManifest, error) {
	trackManifest, err := provider.GetTrackManifest(trackId)
	if err != nil {
		return nil, errors.Join(ErrFetchAndEnqueueTrack, err)
	}

	playbin.SetArg("uri", trackManifest.URL())
	if err := playbin.Set("volume", settings.Player().GetVolume()); err != nil {
		return nil, errors.Join(ErrFetchAndEnqueueTrack, err)
	}

	EnqueuedTrack.Set(trackManifest)
	return trackManifest, nil
}
