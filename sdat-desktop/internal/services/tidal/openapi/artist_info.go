package openapi

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var artistInfoLogger = logger.With("type", "ArtistInfo").WithGroup("artist_info")

type ArtistInfo struct {
	openapi.Artist
}

func (a ArtistInfo) ID() string {
	return a.Artist.Data.ID
}

func (a ArtistInfo) Cover(preferredSize int) tonearm.Cover {
	logger := artistInfoLogger.With("method", "ProfilePicture").WithGroup("profile_picture").With("preferred_size", preferredSize)

	if preferredSize < 0 {
		logger.Debug("defaulting to smallest picture size as a negative value was passed")
		preferredSize = 0
	}

	return coverHelper(a.Included.PlainArtworks(a.Data.Relationships.ProfileArt.Data...), preferredSize)
}

func (a ArtistInfo) Route() string {
	return "artist/" + a.ID()
}

func (a ArtistInfo) SourceType() tonearm.SourceType {
	return tonearm.SourceTypeArtist
}

func (a ArtistInfo) Title() string {
	return a.Artist.Data.Attributes.Name
}

func (a ArtistInfo) URL() string {
	return "https://tidal.com/artist/" + a.ID()
}

func NewArtistInfo(artist openapi.Artist) tonearm.ArtistInfo {
	return &ArtistInfo{artist}
}
