package openapi

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var albumLogger = logger.With("type", "Album").WithGroup("album")

type Album struct {
	AlbumInfo
}

func (a *Album) Artists() tonearm.ArtistInfos {
	artworks := a.Included.Artists(a.Data.Relationships.Artists.Data...)

	artists := make(tonearm.ArtistInfos, 0)
	for _, artist := range artworks {
		artists = append(artists, NewArtistInfo(artist))
	}
	return artists
}

func (a AlbumInfo) Count() int {
	return a.Data.Attributes.NumberOfItems
}

func NewAlbum(album openapi.Album) tonearm.Album {
	return &Album{AlbumInfo{album}}
}
