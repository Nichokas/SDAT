package v2

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2/albums"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2/artists"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2/playlists"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2/search_results"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2/track_manifests"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2/tracks"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/openapi/v2/user_collections"
)

type V2 struct {
	Albums          *albums.Albums
	Artists         *artists.Artists
	Playlists       *playlists.Playlists
	SearchResults   *search_results.SearchResults
	TrackManifests  *track_manifests.TrackManifests
	Tracks          *tracks.Tracks
	UserCollections *user_collections.UserCollections
}

func New(client *internal.Client) *V2 {
	return &V2{
		Albums:          albums.New(client),
		Artists:         artists.New(client),
		Playlists:       playlists.New(client),
		SearchResults:   search_results.New(client),
		TrackManifests:  track_manifests.New(client),
		Tracks:          tracks.New(client),
		UserCollections: user_collections.New(client),
	}
}
