package openapi

import (
	"fmt"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var trackLogger = logger.With("type", "Track").WithGroup("track")

type Track struct {
	TrackInfo
}

func (t Track) Album() tonearm.AlbumInfo {
	albums := t.Included.Albums(t.Data.Relationships.Albums.Data...)

	for _, album := range albums {
		return NewAlbumInfo(album)
	}
	return nil
}

func (t Track) Artists() tonearm.ArtistInfos {
	artworks := t.Included.Artists(t.Data.Relationships.Artists.Data...)

	artists := make(tonearm.ArtistInfos, 0)
	for _, artist := range artworks {
		artists = append(artists, NewArtistInfo(artist))
	}
	return artists
}

func (t Track) Cover(perferredSize int) tonearm.Cover {
	return t.Album().Cover(perferredSize)
}

func (t Track) Route() string {
	return fmt.Sprintf("album/%s", t.Album().ID())
}

func (t Track) SourceType() tonearm.SourceType {
	return tonearm.SourceTypeTrack
}

func NewTrack(item openapi.Track) tonearm.Track {
	return &Track{TrackInfo{item}}
}
