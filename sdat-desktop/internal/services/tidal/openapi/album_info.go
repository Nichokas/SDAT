package openapi

import (
	"time"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var albumInfoLogger = logger.With("type", "AlbumInfo").WithGroup("album_info")

type AlbumInfo struct {
	openapi.Album
}

func (a AlbumInfo) Cover(preferredSize int) tonearm.Cover {
	logger := albumInfoLogger.With("method", "Cover").WithGroup("cover").With("preferred_size", preferredSize)

	if preferredSize < 0 {
		logger.Debug("defaulting to smallest picture size as a negative value was passed")
		preferredSize = 0
	}

	return coverHelper(a.Included.PlainArtworks(a.Data.Relationships.CoverArt.Data...), preferredSize)
}

func (a AlbumInfo) Duration() time.Duration {
	return a.Data.Attributes.Duration.Duration
}

func (a AlbumInfo) ID() string {
	return a.Album.Data.ID
}

func (a AlbumInfo) ReleasedAt() time.Time {
	return a.Data.Attributes.ReleaseDate.Time
}

func (a AlbumInfo) Route() string {
	return "album/" + a.ID()
}

func (a AlbumInfo) SourceType() tonearm.SourceType {
	return tonearm.SourceTypeAlbum
}

func (a AlbumInfo) Title() string {
	return a.Album.Data.Attributes.Title
}

func (a AlbumInfo) URL() string {
	return "https://tidal.com/album/" + a.ID()
}

func NewAlbumInfo(album openapi.Album) tonearm.AlbumInfo {
	return &AlbumInfo{album}
}
