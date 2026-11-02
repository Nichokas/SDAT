package v2

import (
	"strconv"
	"time"

	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var albumInfoLogger = logger.With("type", "AlbumInfo").WithGroup("album_info")

type AlbumInfo struct {
	v2.AlbumItemData
}

func (a AlbumInfo) Cover(preferredSize int) tonearm.Cover {
	logger := albumInfoLogger.With("method", "Cover").WithGroup("cover").With("preferred_size", preferredSize)

	if preferredSize < 0 {
		logger.Debug("legacy api does not support preferred size")
	}

	return coverHelper(a.AlbumItemData.Cover, a.AlbumItemData.VideoCover)
}

func (a AlbumInfo) Duration() time.Duration {
	return time.Duration(a.AlbumItemData.Duration) * time.Second
}

func (a AlbumInfo) ID() string {
	return strconv.Itoa(a.AlbumItemData.Id)
}

func (a AlbumInfo) ReleasedAt() time.Time {
	return a.AlbumItemData.ReleaseDate.Time
}

func (a AlbumInfo) Route() string {
	return "album/" + a.ID()
}

func (a AlbumInfo) SourceType() tonearm.SourceType {
	return tonearm.SourceTypeAlbum
}

func (a AlbumInfo) Title() string {
	return a.AlbumItemData.Title
}

func (a AlbumInfo) URL() string {
	return "https://tidal.com/album/" + a.ID()
}

func NewAlbumInfo(album v2.AlbumItemData) tonearm.AlbumInfo {
	return &AlbumInfo{album}
}
