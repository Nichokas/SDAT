package openapi

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type MyTracksInfo struct{}

func (m MyTracksInfo) Cover(preferredSize int) tonearm.Cover {
	return tonearm.NewURLCover("", "https://tidal.com/assets/my-tracks-DTG3pLQW.png")
}

func (m MyTracksInfo) Route() string {
	return "my-collection/tracks"
}

func (m MyTracksInfo) Title() string {
	return gettext.Get("My Tracks")
}

func (m MyTracksInfo) SourceType() tonearm.SourceType {
	return tonearm.SourceTypePlaylist
}

func (m MyTracksInfo) URL() string {
	return "https://tidal.com/my-collection/tracks"
}
