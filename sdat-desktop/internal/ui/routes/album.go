package routes

import (
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist_header"
	"codeberg.org/dergs/tonearm/internal/ui/pages"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

var albumLogger = slog.With("module", "ui/routes", "route", "album")

func init() {
	router.Register("album/:id", Album)
}

func Album(albumId string) *router.Response {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return router.FromError(gettext.Get("Album"), err)
	}

	album, err := service.GetAlbum(albumId)
	if err != nil {
		return router.FromError(gettext.Get("Album"), err)
	}

	trackPaginator, err := service.GetAlbumTracks(albumId)
	if err != nil {
		return router.FromError(gettext.Get("Album"), err)
	}

	page, err := pages.NewPaginatedTracklistPage(trackPaginator, func(tl *tracklist.TrackList) schwifty.BaseWidgetable {
		tl.SetClickHandler(func(track tonearm.Track, position int) {
			go func() {
				if err := player.PlayAlbum(albumId, false, position); err != nil {
					notifications.OnToast.Notify(gettext.Get("An error occurred while playing the track"))
					albumLogger.Error("An error occurred while playing the album", "error", err.Error())
				}
			}()
		})
		return tl.HMargin(30).VAlign(gtk.AlignStartValue)
	}, tracklist.PositionColumn, tracklist.TitleColumn, tracklist.ArtistsColumn, tracklist.DurationColumn, tracklist.ControlsColumn)

	if err != nil {
		return router.FromError(album.Title(), err)
	}

	return &router.Response{
		PageTitle: album.Title(),
		Error:     err,
		View: VStack(
			components.MainContent(
				tracklist_header.NewAlbum(album, func() {
					go func() {
						if err := player.PlayAlbum(albumId, false, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the album"))
							albumLogger.Error("An error occurred while playing the album", "error", err.Error())
						}
					}()
				}, func() {
					go func() {
						if err := player.PlayAlbum(albumId, true, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the album"))
							albumLogger.Error("An error occurred while playing the album", "error", err.Error())
						}
					}()
				}).HMargin(40),
			),
			page.VExpand(true).MarginTop(20),
		),
	}
}
