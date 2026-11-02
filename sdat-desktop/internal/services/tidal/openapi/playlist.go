package openapi

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var playlistLogger = logger.With("type", "Playlist").WithGroup("playlist")

type Playlist struct {
	PlaylistInfo
}

func (p Playlist) Count() int {
	return p.Data.Attributes.NumberOfItems
}

func (p Playlist) Creator() tonearm.ArtistInfo {
	artists := p.Included.Artists(p.Data.Relationships.OwnerProfiles.Data...)

	if len(artists) == 0 {
		return nil
	}
	return NewArtistInfo(artists[0])
}

func NewPlaylist(playlist openapi.Playlist) tonearm.Playlist {
	return &Playlist{PlaylistInfo{playlist}}
}
