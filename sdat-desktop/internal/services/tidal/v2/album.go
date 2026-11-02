package v2

import (
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var albumLogger = logger.With("type", "Album").WithGroup("album")

type Album struct {
	AlbumInfo
}

func (a *Album) Artists() tonearm.ArtistInfos {
	artists := make(tonearm.ArtistInfos, 0)
	for _, artist := range a.AlbumItemData.Artists {
		artists = append(artists, NewArtistInfo(artist))
	}
	return artists
}

func (a AlbumInfo) Count() int {
	logger := albumLogger.With("method", "Count").WithGroup("count")
	logger.Warn("v2 album does not support Count")
	return -1
}

func NewAlbum(album v2.AlbumItemData) tonearm.Album {
	return &Album{AlbumInfo{album}}
}
