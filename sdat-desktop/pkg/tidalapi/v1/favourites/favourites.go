package favourites

import "codeberg.org/dergs/tonearm/pkg/tidalapi/internal"

type Favourites struct {
	client    *internal.Client
	Albums    *FavouriteAlbum
	Artists   *FavouriteArtist
	Playlists *FavouritePlaylist
	Tracks    *FavouriteTrack
}

func New(client *internal.Client) *Favourites {
	return &Favourites{
		client:    client,
		Albums:    NewFavouriteAlbum(client),
		Artists:   NewFavouriteArtist(client),
		Playlists: NewFavouritePlaylist(client),
		Tracks:    NewFavouriteTrack(client),
	}
}
