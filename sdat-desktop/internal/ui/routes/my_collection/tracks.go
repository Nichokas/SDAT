package my_collection

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/secrets"
	"codeberg.org/dergs/tonearm/internal/services/tidal/openapi"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist_header"
	"codeberg.org/dergs/tonearm/internal/ui/pages"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	modelopenapi "codeberg.org/dergs/tonearm/pkg/tidalapi/models/openapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func Tracks() *router.Response {
	logger := slog.With("module", "ui").WithGroup("ui").With("route", "my_collection").WithGroup("my_collection")
	if !secrets.HasRefreshToken() {
		return &router.Response{
			PageTitle: gettext.Get("My Collection"),
			View:      components.AuthRequired(gettext.Get("Please sign in to view your collection")),
		}
	}

	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	paginator := openapi.NewPaginator(tidal.OpenAPI.V2.UserCollections.Tracks, "me", func(r *modelopenapi.Response[[]modelopenapi.Relationship]) []tonearm.Track {
		results := r.Included.Tracks(r.Data...)
		tracks := make([]tonearm.Track, len(results))
		for i, track := range results {
			tracks[i] = openapi.NewTrack(track)
		}
		return tracks
	}, "items.artists", "items.albums.coverArt")

	page, err := pages.NewPaginatedTracklistPage(paginator, func(tl *tracklist.TrackList) schwifty.BaseWidgetable {
		return tl.HMargin(40).VAlign(gtk.AlignStartValue)
	}, tracklist.CoverColumn, tracklist.TitleAlbumColumn, tracklist.ArtistsColumn, tracklist.DurationColumn, tracklist.ControlsColumn)

	playControlsMenu := gio.NewMenu()
	queueAllItem := gio.NewMenuItem(gettext.Get("Add My Tracks to Queue"), "win.player.queue")
	queueAllItem.SetActionAndTargetValue("win.player.queue", glib.NewVariantString("my_collection/tracks"))
	playControlsMenu.AppendItem(queueAllItem)

	return &router.Response{
		PageTitle: gettext.Get("My Tracks"),
		Error:     err,
		View: VStack(
			components.MainContent(
				tracklist_header.NewCollection(&openapi.MyTracksInfo{}, func() {
					go func() {
						tracks, err := paginator.GetAll()
						if err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the tracks"))
							logger.Error("An error occurred while fetching the tracks", "error", err.Error())
							return
						}

						if err := player.PlayTracklist(new(openapi.MyTracksInfo), tracks, false, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the tracks"))
							logger.Error("An error occurred while playing the tracks", "error", err.Error())
						}
					}()
				}, func() {
					go func() {
						tracks, err := paginator.GetAll()
						if err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while shuffling the tracks"))
							logger.Error("An error occurred while fetching the tracks", "error", err.Error())
							return
						}

						if err := player.PlayTracklist(new(openapi.MyTracksInfo), tracks, true, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while shuffling the tracks"))
							logger.Error("An error occurred while playing the tracks", "error", err.Error())
						}
					}()
				}).HMargin(40),
			),
			page.
				VExpand(true),
		).Spacing(20),
	}
}
