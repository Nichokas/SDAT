package tracklist_header

import (
	"fmt"

	"codeberg.org/dergs/tonearm/internal/gettext"
	appState "codeberg.org/dergs/tonearm/internal/state"
	favouritebutton "codeberg.org/dergs/tonearm/internal/ui/components/favourite_button"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func secondaryControlsPlaylist(playlist tonearm.Playlist, popover *gtk.PopoverMenu) schwifty.Box {
	var appCache appState.FavouriteCache
	if playlist.IsMix() {
		appCache = appState.MixesCache
	} else {
		appCache = appState.PlaylistsCache
	}
	favoriteButton := favouritebutton.FavouriteButton(appCache, playlist.ID())
	return componentSecondaryControls(playlist, popover, favoriteButton)
}

func NewPlaylist(playlist tonearm.Playlist, playFunc func(), shuffleFunc func()) schwifty.Widget {
	cover := playlist.Cover(154)
	title := playlist.Title()
	releaseDate := playlist.CreatedAt().Format("2006")
	creator := "TIDAL"
	if c := playlist.Creator(); c != nil {
		creator = c.Title()
	}
	var description string
	if playlist.IsMix() {
		description = gettext.Get("Personal Mix")
	} else {
		description = fmt.Sprintf("%d Track (%s)", playlist.Count(), tidalapi.FormatDuration(playlist.Duration()))
	}

	menu := gio.NewMenu()
	tracking.SetFinalizer("Menu", menu)

	queueAllItem := gio.NewMenuItem(gettext.Get("Add Playlist to Queue"), "win.player.queue")
	queueAllItem.SetActionAndTargetValue("win.player.queue", glib.NewVariantString(fmt.Sprintf("playlist/%s", playlist.ID())))
	menu.AppendItem(queueAllItem)
	tracking.SetFinalizer("MenuItem", queueAllItem)

	popover := gtk.NewPopoverMenuFromModel(&menu.MenuModel)
	tracking.SetFinalizer("Popover", popover)

	return template(cover, title, releaseDate+" • "+creator, "\n"+description+"\n", componentControls(playFunc, shuffleFunc), secondaryControlsPlaylist(playlist, popover))
}
