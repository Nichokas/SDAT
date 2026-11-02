package v2

import (
	"strconv"

	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var artistInfoLogger = logger.With("type", "ArtistInfo").WithGroup("artist_info")

type ArtistInfo struct {
	v2.ArtistItemData
}

func (a ArtistInfo) ID() string {
	return strconv.Itoa(a.Id)
}

func (a ArtistInfo) Cover(preferredSize int) tonearm.Cover {
	logger := artistInfoLogger.With("method", "ProfilePicture").WithGroup("profile_picture").With("preferred_size", preferredSize)

	if preferredSize > 0 {
		logger.Debug("legacy api does not support preferred size")
	}

	return coverHelper(a.ArtistItemData.Picture, "")
}

func (a ArtistInfo) Route() string {
	return "artist/" + a.ID()
}

func (a ArtistInfo) SourceType() tonearm.SourceType {
	return tonearm.SourceTypeArtist
}

func (a ArtistInfo) Title() string {
	return a.ArtistItemData.Name
}

func (a ArtistInfo) URL() string {
	return "https://tidal.com/artist/" + a.ID()
}

func NewArtistInfo(artist v2.ArtistItemData) tonearm.ArtistInfo {
	return &ArtistInfo{
		ArtistItemData: artist,
	}
}
