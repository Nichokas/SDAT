package openapi

import "encoding/json"

type Lyrics Response[LyricsData]

const ObjectTypeLyrics = "lyrics"

type LyricsDirection string

const (
	LyricsDirectionLTR LyricsDirection = "LEFT_TO_RIGHT"
	LyricsDirectionRTL LyricsDirection = "RIGHT_TO_LEFT"
)

type LyricsData struct {
	Attributes    LyricsAttributes    `json:"attributes"`
	ID            string              `json:"id"`
	Relationships LyricsRelationships `json:"relationships"`
	Type          string              `json:"type"`
}

type LyricsAttributes struct {
	Direction       LyricsDirection          `json:"direction"`
	LRCText         string                   `json:"lrcText"`
	Provider        LyricsAttributesProvider `json:"provider"`
	TechnicalStatus string                   `json:"technicalStatus"`
	Text            string                   `json:"text"`
}

type LyricsRelationships struct {
	Owners Response[[]Relationship] `json:"owners"`
	Track  Response[Relationship]   `json:"track"`
}

type LyricsAttributesProviderSource string

const (
	LyricsAttributesProviderSourceThirdParty LyricsAttributesProviderSource = "THIRD_PARTY"
)

type LyricsAttributesProvider struct {
	CommonTrackID string                         `json:"commonTrackId"`
	LyricsID      string                         `json:"lyricsId"`
	Name          string                         `json:"name"`
	Source        LyricsAttributesProviderSource `json:"source"`
}

func (i IncludedObjects) PlainLyrics(relationships ...Relationship) []LyricsData {
	var objects = i.FromRelationships(relationships, ObjectTypeLyrics)

	var lyrics []LyricsData
	for _, obj := range objects {
		var lyric LyricsData
		if err := json.Unmarshal(obj.Raw, &lyric); err != nil {
			continue
		}
		lyrics = append(lyrics, lyric)
	}
	return lyrics
}

func (i IncludedObjects) Lyrics(relationships ...Relationship) (responses []Lyrics) {
	for _, lyrics := range i.PlainLyrics(relationships...) {
		responses = append(responses, Lyrics{
			Data:     lyrics,
			Included: i,
		})
	}
	return
}
