package components

import (
	"strings"
	"time"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/router"
	v2 "codeberg.org/dergs/tonearm/internal/services/tidal/v2"
	"codeberg.org/dergs/tonearm/internal/ui/components/horizontal_list"
	"codeberg.org/dergs/tonearm/internal/ui/components/media_card"
	"codeberg.org/dergs/tonearm/internal/ui/components/shortcut_list"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tidalapi/helper"
	v1 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v1"
	modelv2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func ForModule(module v1.Module) schwifty.BaseWidgetable {
	switch module.Type {
	case v1.ModuleTypeVideoList:
		// TODO: Implement video lists
		return HStack()
	case v1.ModuleTypeFeaturedPromotions:
		list := horizontal_list.NewHorizontalList(module.Title)
		for _, item := range module.Items {
			if item.Type == v1.ItemTypeCategoryPages {
				continue
			} else if item.Type == v1.ItemTypePlaylist {
				list.Append(media_card.NewMixGeneric(item.ArtifactID, item.ShortHeader, item.ShortSubHeader, tidalapi.ImageURLWithSize(item.ImageID, 550, 400)))
				continue
			} else {
				list.Append(HStack(
					Label("Unsupported\n"+string(item.Type)).
						Justify(gtk.JustifyCenterValue).
						Padding(30).HExpand(true).CornerRadius(10),
				).SizeRequest(192, -1).CSS("box:hover { background: alpha(var(--view-fg-color), 0.1); }").CornerRadius(10))
			}
		}
		return list.SetPageMargin(40)
	case v1.ModuleTypePlaylistList:
		list := horizontal_list.NewHorizontalList(module.Title)
		for _, item := range module.PagedList.Items {
			fakePlaylist := modelv2.PlaylistItemData{
				Creator: struct {
					ID      int    "json:\"id\""
					Name    string "json:\"name,omitempty\""
					Picture string "json:\"picture,omitempty\""
					Type    string "json:\"type\""
				}{
					ID:   item.Creator.ID,
					Name: item.Creator.Name,
				},
				Duration:       item.Duration,
				NumberOfTracks: item.NumberOfTracks,
				SquareImage:    item.SquareImage,
				Title:          item.Title,
				UUID:           item.UUID,
			}
			list.Append(media_card.NewPlaylist(v2.NewPlaylist(fakePlaylist)))
			continue
		}
		return list.SetPageMargin(40)
	case v1.ModuleTypeArtistList:
		list := horizontal_list.NewHorizontalList(module.Title)
		for _, item := range module.PagedList.Items {
			fakeArtist := modelv2.ArtistItemData{
				Id:      item.ID.Int,
				Name:    item.Name,
				Picture: item.Picture,
			}
			list.Append(media_card.NewArtist(v2.NewArtistInfo(fakeArtist)))
			continue
		}
		return list.SetPageMargin(40)
	case v1.ModuleTypeAlbumList:
		list := horizontal_list.NewHorizontalList(module.Title)
		for _, item := range module.PagedList.Items {
			releaseDate, _ := time.Parse(time.DateOnly, item.ReleaseDate)
			artists := make([]modelv2.ArtistItemData, 0)
			for _, artist := range item.Artists {
				artists = append(artists, modelv2.ArtistItemData{
					Id:   artist.ID,
					Name: artist.Name,
				})
			}
			fakeAlbum := modelv2.AlbumItemData{
				Artists:  artists,
				Cover:    item.Cover,
				Id:       item.ID.Int,
				Duration: item.Duration,
				ReleaseDate: helper.TimeDateOnly{
					Time: releaseDate,
				},
				Title: item.Title,
				Type:  "ALBUM",
			}
			list.Append(media_card.NewAlbum(v2.NewAlbum(fakeAlbum)))
			continue
		}
		return list.SetPageMargin(40)
	case v1.ModuleTypeTrackList:
		list := tracklist.NewTrackList(
			tracklist.CoverColumn, tracklist.TitleAlbumColumn,
			tracklist.ArtistsColumn,
			tracklist.DurationColumn, tracklist.ControlsColumn,
		)
		for _, item := range module.PagedList.Items {
			artists := make([]modelv2.ArtistItemData, 0)
			for _, artist := range item.Artists {
				artists = append(artists, modelv2.ArtistItemData{
					Id:   artist.ID,
					Name: artist.Name,
				})
			}
			fakeTrack := &modelv2.TrackItemData{
				Album: modelv2.TrackItemDataAlbum{
					Cover: item.Album.Cover,
					ID:    item.Album.ID,
					Title: item.Album.Title,
				},
				Artists:   artists,
				Duration:  item.Duration,
				Following: false,
				ID:        item.ID.Int,
				Title:     item.Title,
			}
			list.AddTrack(v2.NewTrack(*fakeTrack))
		}
		return VStack(
			NewRowTitle().SetTitle(module.Title),
			list,
		).HMargin(40)
	case v1.ModuleTypePageLinksCloud:
		list := shortcut_list.NewShortcutList()
		for _, item := range module.PagedList.Items {
			list.Append(shortcut_list.NewTextShortcut(item.Title, "").ConnectClicked(func(b gtk.Button) {
				router.Navigate(strings.ReplaceAll(item.APIPath, "pages/", "explore/"))
			}))
		}
		return VStack(
			NewRowTitle().SetTitle(module.Title).HPadding(40),
			list.HMargin(50),
		)
	case v1.ModuleTypePageLinks:
		list := shortcut_list.NewShortcutList()
		for _, item := range module.PagedList.Items {
			list.Append(shortcut_list.NewTextShortcut(item.Title, "").ConnectClicked(func(b gtk.Button) {
				router.Navigate(strings.ReplaceAll(item.APIPath, "pages/", "explore/"))
			}))
		}
		return VStack(
			NewRowTitle().SetTitle(gettext.Get("More")).HPadding(40),
			list.HMargin(50),
		)
	default:
		logger.Warn("Unsupported module type", "type", module.Type)
		return HStack(
			Label("Unsupported Element").
				Background("alpha(var(--view-fg-color), 0.1)").
				Padding(30).HExpand(true).CornerRadius(10),
		).HExpand(true).HMargin(40)
	}
}
