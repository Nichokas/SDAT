package tracklist_header

import (
	"fmt"
	"strings"

	"codeberg.org/dergs/tonearm/internal/gettext"
	appState "codeberg.org/dergs/tonearm/internal/state"
	favouritebutton "codeberg.org/dergs/tonearm/internal/ui/components/favourite_button"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/dergs/tonearm/pkg/tidalapi"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func secondaryControlsAlbum(album tonearm.Album, popover *gtk.PopoverMenu) schwifty.Box {
	favoriteButton := favouritebutton.FavouriteButton(appState.AlbumsCache, album.ID())
	var artistButton any
	if artists := album.Artists(); len(artists) > 1 {
		menu := gio.NewMenu()
		defer menu.Unref()
		for _, artist := range artists {
			menu.AppendItem(gio.NewMenuItem(artist.Title(), "win.route.artist::"+artist.ID()))
		}

		artistButton = MenuButton().
			TooltipText(gettext.Get("Navigate to Artist")).
			IconName("music-artist2-symbolic").
			WithCSSClass("flat").
			MenuModel(&menu.MenuModel)()
	} else if len(artists) == 1 {
		artist := artists[0]
		artistButton = Button().
			TooltipText(gettext.Get("Navigate to Artist")).
			IconName("music-artist2-symbolic").
			WithCSSClass("flat").
			ActionName("win.route.artist").
			ActionTargetValue(glib.NewVariantString(artist.ID()))()
	}
	return componentSecondaryControls(album, popover, artistButton, favoriteButton)
}

func NewAlbum(album tonearm.Album, playFunc func(), shuffleFunc func()) schwifty.Widget {
	cover := album.Cover(154)
	title := album.Title()
	releaseDate := album.ReleasedAt().Format("2006")
	artists := strings.Join(album.Artists().Names(), ", ")
	description := gettext.GetN("%d Track (%s)", "%d Tracks (%s)", album.Count(), album.Count(), tidalapi.FormatDuration(album.Duration()))

	menu := gio.NewMenu()
	tracking.SetFinalizer("Menu", menu)

	queueAllItem := gio.NewMenuItem(gettext.Get("Add Album to Queue"), "win.player.queue")
	queueAllItem.SetActionAndTargetValue("win.player.queue", glib.NewVariantString(fmt.Sprintf("album/%s", album.ID())))
	menu.AppendItem(queueAllItem)
	tracking.SetFinalizer("MenuItem", queueAllItem)

	popover := gtk.NewPopoverMenuFromModel(&menu.MenuModel)
	tracking.SetFinalizer("Popover", popover)

	return template(cover, title, releaseDate+" • "+artists, "\n"+description+"\n", componentControls(playFunc, shuffleFunc), secondaryControlsAlbum(album, popover))
}
