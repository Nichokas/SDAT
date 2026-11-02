package v2

import (
	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var playlistLogger = logger.With("type", "Playlist").WithGroup("playlist")

type Playlist struct {
	PlaylistInfo
}

func (p Playlist) Count() int {
	return p.PlaylistItemData.NumberOfTracks
}

func (p Playlist) Creator() tonearm.ArtistInfo {
	if p.PlaylistItemData.Creator.ID == 0 {
		return nil
	}

	return NewArtistInfo(v2.ArtistItemData{
		Id:      p.PlaylistItemData.Creator.ID,
		Name:    p.PlaylistItemData.Creator.Name,
		Picture: p.PlaylistItemData.Creator.Picture,
	})
}

func NewPlaylist(playlist v2.PlaylistItemData) tonearm.Playlist {
	return &Playlist{PlaylistInfo{playlist}}
}
