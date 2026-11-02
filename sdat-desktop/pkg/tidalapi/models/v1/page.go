package v1

import (
	"encoding/json"
	"fmt"
	"strconv"
)

type ModuleType string

const (
	ModuleTypeAlbumList          ModuleType = "ALBUM_LIST"
	ModuleTypeArtistList         ModuleType = "ARTIST_LIST"
	ModuleTypeFeaturedPromotions ModuleType = "FEATURED_PROMOTIONS"
	ModuleTypePageLinksCloud     ModuleType = "PAGE_LINKS_CLOUD"
	ModuleTypePageLinks          ModuleType = "PAGE_LINKS"
	ModuleTypePlaylistList       ModuleType = "PLAYLIST_LIST"
	ModuleTypeTrackList          ModuleType = "TRACK_LIST"
	ModuleTypeVideoList          ModuleType = "VIDEO_LIST"
)

type ItemType string

const (
	ItemTypePlaylist      ItemType = "PLAYLIST"
	ItemTypeCategoryPages ItemType = "CATEGORY_PAGES"
)

type Page struct {
	ID    string `json:"id"`
	Rows  []Row  `json:"rows"`
	Title string `json:"title"`
}

type Row struct {
	Modules []Module `json:"modules"`
}

type Module struct {
	ID          string     `json:"id"`
	Type        ModuleType `json:"type"`
	Title       string     `json:"title"`
	Description string     `json:"description"`
	Items       []Item     `json:"items"`
	PagedList   PagedList  `json:"pagedList"`
}

type Item struct {
	Header         string   `json:"header"`
	ShortHeader    string   `json:"shortHeader"`
	ShortSubHeader string   `json:"shortSubHeader"`
	ImageID        string   `json:"imageId"`
	Type           ItemType `json:"type"`
	ArtifactID     string   `json:"artifactId"`
	Text           string   `json:"text"`
	Featured       bool     `json:"featured"`
}

type PagedList struct {
	Items              []PagedItem `json:"items"`
	TotalNumberOfItems int         `json:"totalNumberOfItems"`
}

type PagedItem struct {
	Album struct {
		Cover       string `json:"cover"`
		ID          int    `json:"id"`
		Title       string `json:"title"`
		ReleaseDate string `json:"releaseDate"`
	} `json:"album"`
	APIPath string `json:"apiPath"`
	Artists []struct {
		ID   int    `json:"id"`
		Name string `json:"name"`
	} `json:"artists"`
	Cover    string `json:"cover"`
	Creators []struct {
		ID   int    `json:"id"`
		Name string `json:"name"`
	} `json:"creators"`
	Creator struct {
		ID   int    `json:"id"`
		Name string `json:"name"`
	} `json:"creator"`
	Duration       int     `json:"duration"`
	Title          string  `json:"title"`
	ID             MagicID `json:"id"`
	Icon           string  `json:"icon"`
	Name           string  `json:"name"`
	NumberOfTracks int     `json:"numberOfTracks"`
	Picture        string  `json:"picture"`
	ReleaseDate    string  `json:"releaseDate"`
	UUID           string  `json:"uuid"`
	SquareImage    string  `json:"squareImage"`
}

type MagicID struct {
	String string
	Int    int
}

func (m *MagicID) UnmarshalJSON(data []byte) error {
	var s string
	if err := json.Unmarshal(data, &s); err == nil {
		m.String = s
		n, _ := strconv.Atoi(s)
		m.Int = n
		return nil
	}

	var n float64
	if err := json.Unmarshal(data, &n); err == nil {
		m.Int = int(n)
		m.String = strconv.Itoa(m.Int)
		return nil
	}

	return fmt.Errorf("MagicID: cannot unmarshal %s", string(data))
}
