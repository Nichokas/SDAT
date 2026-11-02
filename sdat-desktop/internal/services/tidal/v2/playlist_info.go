package v2

import (
	"time"

	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

var playlistInfoLogger = logger.With("type", "PlaylistInfo").WithGroup("playlist_info")

type PlaylistInfo struct {
	v2.PlaylistItemData
}

func (p PlaylistInfo) Cover(preferredSize int) tonearm.Cover {
	logger := playlistInfoLogger.With("method", "Cover").WithGroup("cover").With("preferred_size", preferredSize)

	if preferredSize < 0 {
		logger.Debug("legacy api does not support preferred size")
	}

	return coverHelper(p.PlaylistItemData.SquareImage, "")
}

func (p PlaylistInfo) CreatedAt() time.Time {
	t, err := time.Parse("2006-01-02T15:04:05.000+0000", p.PlaylistItemData.CreatedAt)
	if err != nil {
		playlistInfoLogger.With("method", "CreatedAt").Debug("failed to parse created at time", "error", err)
		return time.Time{}
	}
	return t
}

func (p PlaylistInfo) Duration() time.Duration {
	return time.Duration(p.PlaylistItemData.Duration) * time.Second
}

func (p PlaylistInfo) ID() string {
	return p.PlaylistItemData.UUID
}

func (p PlaylistInfo) IsMix() bool {
	return false
}

func (p PlaylistInfo) Route() string {
	return "playlist/" + p.ID()
}

func (p PlaylistInfo) SourceType() tonearm.SourceType {
	return tonearm.SourceTypePlaylist
}

func (p PlaylistInfo) Title() string {
	return p.PlaylistItemData.Title
}

func (p PlaylistInfo) URL() string {
	return "https://tidal.com/playlist/" + p.ID()
}

func NewPlaylistInfo(playlist v2.PlaylistItemData) tonearm.PlaylistInfo {
	return &PlaylistInfo{playlist}
}
