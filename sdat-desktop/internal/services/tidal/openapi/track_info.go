package openapi

import (
	"slices"
	"time"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
)

type TrackInfo struct {
	openapi.Track
}

func (t TrackInfo) Duration() time.Duration {
	return t.Data.Attributes.Duration.Duration
}

func (t TrackInfo) Flags() tonearm.TrackFlag {
	flags := tonearm.TrackFlag(0)
	if t.Data.Attributes.AI {
		flags |= tonearm.TrackFlagAI
	}
	if t.Data.Attributes.Explicit {
		flags |= tonearm.TrackFlagExplicit
	}
	return flags
}

func (t TrackInfo) ID() string {
	return t.Data.ID
}

func (t TrackInfo) IsStreamable() bool {
	return slices.Contains(t.Data.Attributes.Availability, openapi.TrackAvailabilityStream)
}

func (t TrackInfo) Title() string {
	return t.Data.Attributes.Title
}

func (t TrackInfo) URL() string {
	return "https://tidal.com/track/" + t.ID()
}

func (t TrackInfo) Version() string {
	return t.Data.Attributes.Version
}

func NewTrackInfo(item openapi.Track) tonearm.TrackInfo {
	return &TrackInfo{item}
}
