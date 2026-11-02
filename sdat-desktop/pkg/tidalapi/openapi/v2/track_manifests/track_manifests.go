package track_manifests

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type TrackManifests struct {
	client *internal.Client
}

func New(client *internal.Client) *TrackManifests {
	return &TrackManifests{
		client: client,
	}
}
