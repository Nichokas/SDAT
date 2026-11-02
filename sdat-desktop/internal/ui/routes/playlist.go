package routes

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/internal/player"
	"codeberg.org/dergs/tonearm/internal/router"
	"codeberg.org/dergs/tonearm/internal/signals"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist"
	"codeberg.org/dergs/tonearm/internal/ui/components/tracklist_header"
	"codeberg.org/dergs/tonearm/internal/ui/pages"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

var canPlayPlaylistState = state.NewStateful(false)

func init() {
	player.PlaybackStateChanged.On(func(ps player.PlaybackState) bool {
		canPlayPlaylistState.SetValue(!ps.Loading)
		return signals.Continue
	})

	router.Register("playlist/:id", Playlist)
}

func Playlist(playlistID string) *router.Response {
	service, err := injector.Inject[tonearm.Service]()
	if err != nil {
		return router.FromError(gettext.Get("Playlist"), err)
	}

	playlist, err := service.GetPlaylist(playlistID)
	if err != nil {
		return router.FromError(gettext.Get("Playlist"), err)
	}

	trackPaginator, err := service.GetPlaylistTracks(playlistID)
	if err != nil {
		return router.FromError(gettext.Get("Playlist"), err)
	}

	page, err := pages.NewPaginatedTracklistPage(trackPaginator, func(tl *tracklist.TrackList) schwifty.BaseWidgetable {
		tl.SetClickHandler(func(track tonearm.Track, position int) {
			go func() {
				if err := player.PlayPlaylist(playlistID, false, position); err != nil {
					notifications.OnToast.Notify(gettext.Get("An error occurred while playing the track"))
					albumLogger.Error("An error occurred while playing the playlist", "error", err.Error())
				}
			}()
		})
		return tl.HMargin(30).VAlign(gtk.AlignStartValue)
	}, tracklist.CoverColumn, tracklist.TitleAlbumColumn, tracklist.ArtistsColumn, tracklist.DurationColumn, tracklist.ControlsColumn)

	return &router.Response{
		PageTitle: playlist.Title(),
		Error:     err,
		View: VStack(
			components.MainContent(
				tracklist_header.NewPlaylist(playlist, func() {
					go func() {
						if err := player.PlayPlaylist(playlistID, false, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the playlist"))
							albumLogger.Error("An error occurred while playing the playlist", "error", err.Error())
						}
					}()
				}, func() {
					go func() {
						if err := player.PlayPlaylist(playlistID, true, 0); err != nil {
							notifications.OnToast.Notify(gettext.Get("An error occurred while playing the playlist"))
							albumLogger.Error("An error occurred while playing the playlist", "error", err.Error())
						}
					}()
				}).HMargin(40),
			),
			page.VExpand(true).MarginTop(20),
		),
	}
}
