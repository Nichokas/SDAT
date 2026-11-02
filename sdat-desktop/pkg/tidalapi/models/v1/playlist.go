package v1

import (
	"codeberg.org/dergs/tonearm/pkg/tidalapi/helper"
)

type Playlist struct {
	Created helper.TidalDateTime `json:"created"`
	Creator struct {
		ID   int    `json:"id"`
		Name string `json:"name,omitempty"`
	} `json:"creator"`
	Description    string `json:"description"`
	Duration       int    `json:"duration"`
	NumberOfTracks int    `json:"numberOfTracks"`
	SquareImage    string `json:"squareImage"`
	Title          string `json:"title"`
	UUID           string `json:"uuid"`
}

type PlaylistItems struct {
	Items              []PlaylistItem `json:"items"`
	Limit              int            `json:"limit"`
	Offset             int            `json:"offset"`
	TotalNumberOfItems int            `json:"totalNumberOfItems"`
}

type PlaylistItem struct {
	AI    bool `json:"ai"`
	Album struct {
		ID    int    `json:"id"`
		Title string `json:"title"`
	} `json:"album"`
	Artists []struct {
		ID   int    `json:"id"`
		Name string `json:"name"`
	} `json:"artists"`
	Duration    int    `json:"duration"`
	Explicit    bool   `json:"explicit"`
	ID          int    `json:"id"`
	Title       string `json:"title"`
	TrackNumber int    `json:"trackNumber"`
}
