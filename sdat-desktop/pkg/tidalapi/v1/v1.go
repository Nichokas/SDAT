package v1

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/internal"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/v1/albums"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/v1/favourites"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/v1/pages"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/v1/playlists"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/v1/tracks"
)

type V1 struct {
	client     *internal.Client
	Albums    *albums.Albums
	Favourites *favourites.Favourites
	Pages      *pages.Pages
	Playlists  *playlists.Playlists
	Tracks     *tracks.Tracks
}

func New(client *internal.Client) *V1 {
	return &V1{
		client:     client,
		Albums:    albums.New(client),
		Favourites: favourites.New(client),
		Pages:      pages.New(client),
		Playlists:  playlists.New(client),
		Tracks:     tracks.New(client),
	}
}
