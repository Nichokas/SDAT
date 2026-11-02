package tracklist_header

import (
	"fmt"

	"codeberg.org/dergs/tonearm/internal/gettext"
	appState "codeberg.org/dergs/tonearm/internal/state"
	favouritebutton "codeberg.org/dergs/tonearm/internal/ui/components/favourite_button"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func secondaryControlsArtist(artist tonearm.Artist, popover *gtk.PopoverMenu) schwifty.Box {
	favoriteButton := favouritebutton.FavouriteButton(appState.ArtistsCache, artist.ID())
	return componentSecondaryControls(artist, popover, favoriteButton)
}

func NewArtist(artist tonearm.Artist, playFunc func(), shuffleFunc func()) schwifty.Widget {
	cover := artist.Cover(154)
	title := artist.Title()
	fans := gettext.GetN("%d Fan", "%d Fans", artist.FollowerCount(), artist.FollowerCount())

	menu := gio.NewMenu()
	tracking.SetFinalizer("Menu", menu)

	queueAllItem := gio.NewMenuItem(gettext.Get("Add Top Tracks to Queue"), "win.player.queue")
	queueAllItem.SetActionAndTargetValue("win.player.queue", glib.NewVariantString(fmt.Sprintf("artist/%s", artist.ID())))
	menu.AppendItem(queueAllItem)
	tracking.SetFinalizer("MenuItem", queueAllItem)

	popover := gtk.NewPopoverMenuFromModel(&menu.MenuModel)
	tracking.SetFinalizer("Popover", popover)

	return template(cover, title, fans, artist.Description(), componentControls(playFunc, shuffleFunc), secondaryControlsArtist(artist, popover))
}
