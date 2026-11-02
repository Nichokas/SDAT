package v2

import (
	"strconv"
	"time"

	v2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type TrackInfo struct {
	v2.TrackItemData
}

func (t TrackInfo) Duration() time.Duration {
	return time.Duration(t.TrackItemData.Duration) * time.Second
}

func (t TrackInfo) Flags() tonearm.TrackFlag {
	flags := tonearm.TrackFlag(0)
	if t.AI {
		flags |= tonearm.TrackFlagAI
	}
	if t.Explicit {
		flags |= tonearm.TrackFlagExplicit
	}
	return flags
}

func (t TrackInfo) ID() string {
	return strconv.Itoa(t.TrackItemData.ID)
}

func (t TrackInfo) IsStreamable() bool {
	return t.AllowStreaming
}

func (t TrackInfo) Title() string {
	return t.TrackItemData.Title
}

func (t TrackInfo) URL() string {
	return "https://tidal.com/track/" + t.ID()
}

func (t TrackInfo) Version() string {
	return t.TrackItemData.Version
}

func NewTrackInfo(item v2.TrackItemData) tonearm.TrackInfo {
	return &TrackInfo{TrackItemData: item}
}
