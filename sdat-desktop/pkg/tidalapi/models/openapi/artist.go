package openapi

import (
	"encoding/json"
)

type Artist Response[ArtistData]

const ObjectTypeArtists = "artists"

type ArtistExternalLinkType string

const (
	ArtistExternalLinkTypeTidalSharing ArtistExternalLinkType = "TIDAL_SHARING"
)

type ArtistData struct {
	Attributes    ArtistAttributes    `json:"attributes"`
	ID            string              `json:"id"`
	Relationships ArtistRelationships `json:"relationships"`
	Type          string              `json:"type"`
}

type ArtistAttributes struct {
	ContributionsEnabled bool                 `json:"contributionsEnabled"`
	ExternalLinks        []ArtistExternalLink `json:"externalLinks"`
	Name                 string               `json:"name"`
	Popularity           float64              `json:"popularity"`
	Spotlighted          bool                 `json:"spotlighted"`
}

type ArtistRelationships struct {
	Albums         Response[[]Relationship] `json:"albums"`
	Biography      Response[[]Relationship] `json:"biography"`
	Followers      Response[[]Relationship] `json:"followers"`
	Following      Response[[]Relationship] `json:"following"`
	Owners         Response[[]Relationship] `json:"owners"`
	ProfileArt     Response[[]Relationship] `json:"profileArt"`
	Radio          Response[[]Relationship] `json:"radio"`
	Roles          Response[[]Relationship] `json:"roles"`
	SimilarArtists Response[[]Relationship] `json:"similarArtists"`
	TrackProviders Response[[]Relationship] `json:"trackProviders"`
	Tracks         Response[[]Relationship] `json:"tracks"`
	Videos         Response[[]Relationship] `json:"videos"`
}

type ArtistExternalLink struct {
	Href string `json:"href"`
	Meta struct {
		Type ArtistExternalLinkType `json:"type"`
	} `json:"meta"`
}

func (i IncludedObjects) PlainArtists(relationships ...Relationship) []ArtistData {
	var objects = i.FromRelationships(relationships, ObjectTypeArtists)

	var artists []ArtistData
	for _, obj := range objects {
		var artist ArtistData
		if err := json.Unmarshal(obj.Raw, &artist); err != nil {
			continue
		}
		artists = append(artists, artist)
	}
	return artists
}

func (i IncludedObjects) Artists(relationships ...Relationship) (responses []Artist) {
	for _, artists := range i.PlainArtists(relationships...) {
		responses = append(responses, Artist{
			Data:     artists,
			Included: i,
		})
	}
	return
}
