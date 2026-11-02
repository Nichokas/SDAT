package openapi

import (
	"encoding/json"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/helper"
)

type Album Response[AlbumData]

const ObjectTypeAlbums = "albums"

type AlbumExternalLinkType string

const (
	AlbumExternalLinkTypeTidalSharing AlbumExternalLinkType = "TIDAL_SHARING"
)

type AlbumData struct {
	Attributes    AlbumAttributes    `json:"attributes"`
	ID            string             `json:"id"`
	Relationships AlbumRelationships `json:"relationships"`
	Type          string             `json:"type"`
}

type AlbumAttributes struct {
	AccessType   string   `json:"accessType"`
	AI           bool     `json:"ai"`
	Availability []string `json:"availability"`
	BarcodeID    string   `json:"barcodeId"`
	Copyright    struct {
		Text string `json:"text"`
	}
	Duration        Duration            `json:"duration"`
	Explicit        bool                `json:"explicit"`
	ExternalLinks   []AlbumExternalLink `json:"externalLinks"`
	MediaTags       []string            `json:"media_tags"`
	NumberOfItems   int                 `json:"numberOfItems"`
	NumberOfVolumes int                 `json:"numberOfVolumes"`
	Popularity      float64             `json:"popularity"`
	ReleaseDate     helper.TimeDateOnly `json:"releaseDate"`
	Title           string              `json:"title"`
	Type            string              `json:"type"`
}

type AlbumRelationships struct {
	Artists            Response[[]Relationship] `json:"artists"`
	CoverArt           Response[[]Relationship] `json:"coverArt"`
	Genres             Response[[]Relationship] `json:"genres"`
	Items              Response[[]Relationship] `json:"items"`
	Owners             Response[[]Relationship] `json:"owners"`
	Providers          Response[[]Relationship] `json:"providers"`
	SimilarAlbums      Response[[]Relationship] `json:"similarAlbums"`
	SuggestedCoverArts Response[[]Relationship] `json:"suggestedCoverArts"`
}

type AlbumExternalLink struct {
	Href string `json:"href"`
	Meta struct {
		Type AlbumExternalLinkType `json:"type"`
	} `json:"meta"`
}

func (i IncludedObjects) PlainAlbums(relationships ...Relationship) []AlbumData {
	var objects = i.FromRelationships(relationships, ObjectTypeAlbums)

	var albums []AlbumData
	for _, obj := range objects {
		var album AlbumData
		if err := json.Unmarshal(obj.Raw, &album); err != nil {
			continue
		}
		albums = append(albums, album)
	}
	return albums
}

func (i IncludedObjects) Albums(relationships ...Relationship) (responses []Album) {
	for _, albums := range i.PlainAlbums(relationships...) {
		responses = append(responses, Album{
			Data:     albums,
			Included: i,
		})
	}
	return
}
