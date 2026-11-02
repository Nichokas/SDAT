package v2

import (
	"encoding/json"
	"fmt"
	"log/slog"
	"strconv"

	"codeberg.org/dergs/tonearm/pkg/tidalapi/helper"
)

type baseItem struct {
	Following         bool            `json:"following"`
	NumberOfFollowers int             `json:"numberOfFollowers"`
	Type              ItemType        `json:"type"`
	RawData           json.RawMessage `json:"data"`
}

type Item struct {
	baseItem
	Data ItemData
}

func (i *Item) MarshalJSON() ([]byte, error) {
	switch i.Type {
	case ItemTypeArtist:
		if raw, err := json.Marshal(i.Data.Artist); err != nil {
			return nil, err
		} else {
			i.RawData = raw
		}
	case ItemTypeDeepLink:
		if raw, err := json.Marshal(i.Data.DeepLink); err != nil {
			return nil, err
		} else {
			i.RawData = raw
		}
	case ItemTypeMix:
		if raw, err := json.Marshal(i.Data.Mix); err != nil {
			return nil, err
		} else {
			i.RawData = raw
		}
	case ItemTypePlaylist:
		if raw, err := json.Marshal(i.Data.Playlist); err != nil {
			return nil, err
		} else {
			i.RawData = raw
		}
	case ItemTypeTrack:
		if raw, err := json.Marshal(i.Data.Track); err != nil {
			return nil, err
		} else {
			i.RawData = raw
		}
	case ItemTypeAlbum:
		if raw, err := json.Marshal(i.Data.Album); err != nil {
			return nil, err
		} else {
			i.RawData = raw
		}
	default:
		return nil, fmt.Errorf("Unknown item type %s", i.Type)
	}
	return json.Marshal(i.baseItem)
}

func (i *Item) UnmarshalJSON(data []byte) error {
	if err := json.Unmarshal(data, &i.baseItem); err != nil {
		return err
	}

	i.Following = i.baseItem.Following
	i.NumberOfFollowers = i.baseItem.NumberOfFollowers
	i.Type = i.baseItem.Type
	i.Data = ItemData{}
	switch i.baseItem.Type {
	case ItemTypeArtist:
		if err := json.Unmarshal(i.baseItem.RawData, &i.Data.Artist); err != nil {
			return err
		}
	case ItemTypeDeepLink:
		if err := json.Unmarshal(i.baseItem.RawData, &i.Data.DeepLink); err != nil {
			return err
		}
	case ItemTypeMix:
		if err := json.Unmarshal(i.baseItem.RawData, &i.Data.Mix); err != nil {
			return err
		}
	case ItemTypePlaylist:
		if err := json.Unmarshal(i.baseItem.RawData, &i.Data.Playlist); err != nil {
			return err
		}
	case ItemTypeTrack:
		if err := json.Unmarshal(i.baseItem.RawData, &i.Data.Track); err != nil {
			return err
		}
	case ItemTypeAlbum:
		if err := json.Unmarshal(i.baseItem.RawData, &i.Data.Album); err != nil {
			return err
		}
	default:
		slog.Error("Unknown item type", "item_type", i.Type)
	}
	return nil
}

type ItemData struct {
	Artist   *ArtistItemData
	Album    *AlbumItemData
	DeepLink *DeepLinkItemData
	Mix      *MixItemData
	Playlist *PlaylistItemData
	Track    *TrackItemData
}

type ArtistItemData struct {
	Id      int    `json:"id"`
	Name    string `json:"name"`
	Picture string `json:"picture"`
}

type AlbumItemData struct {
	AI          bool                `json:"ai"`
	Artists     []ArtistItemData    `json:"artists"`
	Cover       string              `json:"cover"`
	Id          int                 `json:"id"`
	Duration    int                 `json:"duration"`
	Explicit    bool                `json:"explicit"`
	ReleaseDate helper.TimeDateOnly `json:"releaseDate"`
	Title       string              `json:"title"`
	Type        string              `json:"type"`
	VideoCover  string              `json:"videoCover"`
}

type DeepLinkItemData struct {
	ExternalURL bool   `json:"externalUrl"`
	Id          string `json:"id"`
	Title       string `json:"title"`
	URL         string `json:"url"`
}

type MixItemData struct {
	Id        string `json:"id"`
	MixImages []struct {
		URL string `json:"url"`
	} `json:"mixImages"`
	TitleTextInfo struct {
		Text string `json:"text"`
	} `json:"titleTextInfo"`
	SubtitleTextInfo struct {
		Text string `json:"text"`
	} `json:"subtitleTextInfo"`
	ShortSubtitleTextInfo struct {
		Text string `json:"text"`
	} `json:"shortSubtitleTextInfo"`
	Type string `json:"type"`
}

type PlaylistItemData struct {
	Creator struct {
		ID      int    `json:"id"`
		Name    string `json:"name,omitempty"`
		Picture string `json:"picture,omitempty"`
		Type    string `json:"type"`
	}
	CreatedAt      string `json:"created"`
	Description    string `json:"description,omitempty"`
	Duration       int    `json:"duration"`
	NumberOfTracks int    `json:"numberOfTracks"`
	SquareImage    string `json:"squareImage"`
	Title          string `json:"title"`
	Type           string `json:"type"`
	UUID           string `json:"uuid"`
}

type TrackItemData struct {
	AI             bool `json:"ai"`
	Album          TrackItemDataAlbum
	AllowStreaming bool             `json:"allowStreaming"`
	Artists        []ArtistItemData `json:"artists"`
	Duration       int              `json:"duration"`
	Explicit       bool             `json:"explicit"`
	// In UI terms: Indicates whether the track has been "heart"-ed
	Following bool   `json:"following"`
	ID        int    `json:"id"`
	Title     string `json:"title"`
	Version   string `json:"version"`
}

func (t TrackItemData) GetID() string {
	return strconv.Itoa(t.ID)
}

type TrackItemDataAlbum struct {
	AI          bool                `json:"ai"`
	Cover       string              `json:"cover"`
	ID          int                 `json:"id"`
	ReleaseDate helper.TimeDateOnly `json:"releaseDate"`
	Title       string              `json:"title"`
	VideoCover  string              `json:"videoCover"`
}
