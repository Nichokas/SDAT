package routes

import (
	"context"
	"log/slog"

	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist_header"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func init() {
	router.Register("artist/:id", Artist)
}

var artistLogger = slog.With("module", "ui/routes", "route", "artist")

func Artist(artistId string) *router.Response {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return router.FromError(gettext.Get("Artist"), err)
	}

	artist, err := service.GetArtist(artistId)
	if err != nil {
		return router.FromError(gettext.Get("Artist"), err)
	}

	tidal := injector.MustInject[*tidalapi.TidalAPI]()
	artistPage, err := tidal.V2.Artist.Artist(context.Background(), artistId)
	if err != nil {
		return router.FromError(gettext.Get("Artist"), err)
	}

	body := VStack().Spacing(25).MarginTop(20).PaddingBottom(20)
	for _, item := range artistPage.Items {
		body = body.Append(components.ForPageItem(item))
	}

	return &router.Response{
		PageTitle: gettext.Get("Artist"),
		View: VStack(
			components.MainContent(
				tracklist_header.NewArtist(artist, func() {
					go func() {
						if err := player.PlayArtistTopSongs(artistId, false, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the top tracks"))
							albumLogger.Error("An error occurred while playing the top tracks", "error", err.Error())
						}
					}()
				}, func() {
					go func() {
						if err := player.PlayArtistTopSongs(artistId, true, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the top tracks"))
							albumLogger.Error("An error occurred while playing the top tracks", "error", err.Error())
						}
					}()
				}).HMargin(40),
			),
			ScrolledWindow().
				Child(
					components.MainContent(
						body,
					),
				).
				Policy(gtk.PolicyNeverValue, gtk.PolicyAutomaticValue).
				VExpand(true).
				MarginTop(20),
		),
	}
}
