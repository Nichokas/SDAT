package openapi

import "encoding/json"

type Playlist Response[PlaylistData]

const ObjectTypePlaylists = "playlists"

type PlaylistType string

const (
	PlaylistTypeEditorial PlaylistType = "EDITORIAL"
	PlaylistTypeMix       PlaylistType = "MIX"
	PlaylistTypeUser      PlaylistType = "USER"
)

type PlaylistExternalLinkType string

const (
	PlaylistExternalLinkTypeTidalSharing         PlaylistExternalLinkType = "TIDAL_SHARING"
	PlaylistExternalLinkTypeTidalAutoplayAndroid PlaylistExternalLinkType = "TIDAL_AUTOPLAY_ANDROID"
	PlaylistExternalLinkTypeTidalAutoplayIOS     PlaylistExternalLinkType = "TIDAL_AUTOPLAY_IOS"
	PlaylistExternalLinkTypeTidalAutoplayWeb     PlaylistExternalLinkType = "TIDAL_AUTOPLAY_WEB"
)

type PlaylistData struct {
	Attributes    PlaylistAttributes    `json:"attributes"`
	ID            string                `json:"id"`
	Relationships PlaylistRelationships `json:"relationships"`
	Type          string                `json:"type"`
}

type PlaylistAttributes struct {
	AccessType        string                 `json:"accessType"`
	Bounded           bool                   `json:"bounded"`
	CreatedAt         DateTime               `json:"createdAt"`
	Description       string                 `json:"description"`
	Duration          *Duration              `json:"duration,omitempty"`
	ExternalLinks     []PlaylistExternalLink `json:"externalLinks"`
	LastModifiedAt    DateTime               `json:"lastModifiedAt"`
	Name              string                 `json:"name"`
	NumberOfFollowers int                    `json:"numberOfFollowers"`
	NumberOfItems     int                    `json:"numberOfItems"`
	PlaylistType      PlaylistType           `json:"playlistType"`
}

type PlaylistRelationships struct {
	CoverArt      Response[[]Relationship] `json:"coverArt"`
	Items         Response[[]Relationship] `json:"items"`
	OwnerProfiles Response[[]Relationship] `json:"ownerProfiles"`
	Owners        Response[[]Relationship] `json:"owners"`
}

type PlaylistExternalLink struct {
	Href string `json:"href"`
	Meta struct {
		Type PlaylistExternalLinkType `json:"type"`
	} `json:"meta"`
}

func (i IncludedObjects) PlainPlaylists(relationships ...Relationship) []PlaylistData {
	var objects = i.FromRelationships(relationships, ObjectTypePlaylists)

	var playlists []PlaylistData
	for _, obj := range objects {
		var playlist PlaylistData
		if err := json.Unmarshal(obj.Raw, &playlist); err != nil {
			continue
		}
		playlists = append(playlists, playlist)
	}
	return playlists
}

func (i IncludedObjects) Playlists(relationships ...Relationship) (responses []Playlist) {
	for _, playlists := range i.PlainPlaylists(relationships...) {
		responses = append(responses, Playlist{
			Data:     playlists,
			Included: i,
		})
	}
	return
}
