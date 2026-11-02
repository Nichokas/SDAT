package openapi

import (
	"encoding/json"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/helper"
)

type Track Response[TrackData]

func (t Track) GetID() string {
	return t.Data.ID
}

const ObjectTypeTracks = "tracks"

type TrackExternalLinkType string

const (
	TrackExternalLinkTypeTidalSharing TrackExternalLinkType = "TIDAL_SHARING"
)

type TrackAvailability string

const (
	TrackAvailabilityDJ     TrackAvailability = "DJ"
	TrackAvailabilityStream TrackAvailability = "STREAM"
)

type TrackData struct {
	Attributes    TrackAttributes    `json:"attributes"`
	ID            string             `json:"id"`
	Relationships TrackRelationships `json:"relationships"`
	Type          string             `json:"type"`
}

type TrackAttributes struct {
	AccessType   string              `json:"accessType"`
	AI           bool                `json:"ai"`
	Availability []TrackAvailability `json:"availability"`
	BPM          float64             `json:"bpm"`
	Copyright    struct {
		Text string `json:"text"`
	}
	CreatedAt     DateTime               `json:"createdAt"`
	Duration      helper.DurationISO8601 `json:"duration"`
	Explicit      bool                   `json:"explicit"`
	ExternalLinks []TrackExternalLink    `json:"externalLinks"`
	ISRC          string                 `json:"isrc"`
	Key           string                 `json:"key"`
	KeyScale      string                 `json:"keyScale"`
	MediaTags     []string               `json:"media_tags"`
	Popularity    float64                `json:"popularity"`
	Spotlighted   bool                   `json:"spotlighted"`
	Title         string                 `json:"title"`
	ToneTags      []string               `json:"toneTags"`
	Version       string                 `json:"version"`
}

type TrackRelationships struct {
	Albums          Response[[]Relationship] `json:"albums"`
	Artists         Response[[]Relationship] `json:"artists"`
	Genres          Response[[]Relationship] `json:"genres"`
	Lyrics          Response[[]Relationship] `json:"lyrics"`
	Owners          Response[[]Relationship] `json:"owners"`
	Providers       Response[[]Relationship] `json:"providers"`
	Radio           Response[[]Relationship] `json:"radio"`
	Shares          Response[[]Relationship] `json:"shares"`
	SimilarTracks   Response[[]Relationship] `json:"similarTracks"`
	SourceFile      Response[[]Relationship] `json:"sourceFile"`
	TrackStatistics Response[[]Relationship] `json:"trackStatistics"`
}

type TrackExternalLink struct {
	Href string `json:"href"`
	Meta struct {
		Type TrackExternalLinkType `json:"type"`
	} `json:"meta"`
}

func (i IncludedObjects) PlainTracks(relationships ...Relationship) []TrackData {
	var objects = i.FromRelationships(relationships, ObjectTypeTracks)

	var tracks []TrackData
	for _, obj := range objects {
		var track TrackData
		if err := json.Unmarshal(obj.Raw, &track); err != nil {
			continue
		}
		tracks = append(tracks, track)
	}
	return tracks
}

func (i IncludedObjects) Tracks(relationships ...Relationship) (responses []Track) {
	for _, tracks := range i.PlainTracks(relationships...) {
		responses = append(responses, Track{
			Data:     tracks,
			Included: i,
		})
	}
	return
}
