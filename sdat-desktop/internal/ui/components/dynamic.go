package components

import (
	"log/slog"

	v2 "codeberg.org/dergs/tonearm/internal/services/tidal/v2"
	"codeberg.org/dergs/tonearm/internal/ui/components/horizontal_list"
	"codeberg.org/dergs/tonearm/internal/ui/components/media_card"
	"codeberg.org/dergs/tonearm/internal/ui/components/shortcut_list"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	modelv2 "codeberg.org/dergs/tonearm/pkg/tidalapi/models/v2"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var logger = slog.With("module", "components")

func ForPageItem(pageItem modelv2.PageItem) schwifty.BaseWidgetable {
	switch pageItem.Type {
	case modelv2.ItemTypeHorizontalList, modelv2.ItemTypeHorizontalListWithContext:
		title := pageItem.Title
		if pageItem.Header != nil {
			switch pageItem.Header.Type {
			case modelv2.ItemTypeAlbum:
				title = pageItem.Header.Data.Album.Title
			case modelv2.ItemTypeArtist:
				title = pageItem.Header.Data.Artist.Name
			case modelv2.ItemTypeDeepLink:
				title = pageItem.Header.Data.DeepLink.Title
			case modelv2.ItemTypeMix:
				title = pageItem.Header.Data.Mix.TitleTextInfo.Text
			case modelv2.ItemTypePlaylist:
				title = pageItem.Header.Data.Playlist.Title
			case modelv2.ItemTypeTrack:
				title = pageItem.Header.Data.Track.Title
			}
		}
		list := horizontal_list.NewHorizontalList(title)
		for _, item := range pageItem.Items {
			if item.Type == modelv2.ItemTypeAlbum {
				list.Append(media_card.NewAlbum(v2.NewAlbum(*item.Data.Album)))
			} else if item.Type == modelv2.ItemTypePlaylist {
				list.Append(media_card.NewPlaylist(v2.NewPlaylist(*item.Data.Playlist)))
			} else if item.Type == modelv2.ItemTypeArtist {
				list.Append(media_card.NewArtist(v2.NewArtistInfo(*item.Data.Artist)))
			} else if item.Type == modelv2.ItemTypeMix {
				list.Append(media_card.NewLegacyMix(item.Data.Mix))
			} else if item.Type == modelv2.ItemTypeTrack {
				list.Append(media_card.NewTrack(v2.NewTrack(*item.Data.Track)))
			} else if item.Type == modelv2.ItemTypeDeepLink {
				list.Append(media_card.NewLegacyDeeplink(item.Data.DeepLink))
			} else {
				list.Append(HStack(
					Label("Unsupported\n"+string(item.Type)).
						Justify(gtk.JustifyCenterValue).
						Padding(30).HExpand(true).CornerRadius(10),
				).SizeRequest(192, -1).CSS("box:hover { background: alpha(var(--view-fg-color), 0.1); }").CornerRadius(10))
			}
		}

		if pageItem.Header != nil {
			return VStack(
				Label(pageItem.Title).HAlign(gtk.AlignStartValue).WithCSSClass("dimmed").HMargin(50),
				list.SetPageMargin(40),
			)
		}

		return list.SetPageMargin(40)
	case modelv2.ItemTypeTrackList:
		list := tracklist.NewTrackList(
			tracklist.CoverColumn, tracklist.TitleAlbumColumn,
			tracklist.ArtistsColumn,
			tracklist.DurationColumn, tracklist.ControlsColumn,
		)
		for _, track := range pageItem.Items {
			list.AddTrack(v2.NewTrack(*track.Data.Track))
		}
		return VStack(
			NewRowTitle().SetTitle(pageItem.Title),
			list,
		).HMargin(40)
	case modelv2.ItemTypeShortcutList:
		list := shortcut_list.NewShortcutList()
		for _, item := range pageItem.Items {
			if item.Type == modelv2.ItemTypeDeepLink {
				list.Append(shortcut_list.NewLegacyDeepLink(item.Data.DeepLink))
			} else if item.Type == modelv2.ItemTypeAlbum {
				list.Append(shortcut_list.NewLegacyAlbum(item.Data.Album))
			} else if item.Type == modelv2.ItemTypeArtist {
				list.Append(shortcut_list.NewLegacyArtist(item.Data.Artist))
			} else if item.Type == modelv2.ItemTypePlaylist {
				list.Append(shortcut_list.NewLegacyPlaylist(item.Data.Playlist))
			} else if item.Type == modelv2.ItemTypeMix {
				list.Append(shortcut_list.NewLegacyMix(item.Data.Mix))
			} else {
				list.Append(syntax.Label("Unsupported: " + string(item.Type)).HMargin(10))
			}
		}
		return list.HMargin(50)
	default:
		logger.Warn("Unsupported item type", "type", pageItem.Type)
		return HStack(
			Label("Unsupported Element").
				Background("alpha(var(--view-fg-color), 0.1)").
				Padding(30).HExpand(true).CornerRadius(10),
		).HExpand(true).HMargin(40)
	}
}
