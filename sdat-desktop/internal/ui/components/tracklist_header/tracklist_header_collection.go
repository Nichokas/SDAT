package tracklist_header

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gio"
	"codeberg.org/puregotk/puregotk/v4/glib"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type shareablePlaybackSource interface {
	tonearm.PlaybackSource
	tonearm.Shareable
}

func secondaryControlsCollection(playbackSource shareablePlaybackSource, popover *gtk.PopoverMenu) schwifty.Box {
	return componentSecondaryControls(playbackSource, popover)
}

func NewCollection(playbackSource shareablePlaybackSource, playFunc func(), shuffleFunc func()) schwifty.Widget {
	cover := playbackSource.Cover(154)
	title := playbackSource.Title()
	subtitle := gettext.Get("My Collection")

	menu := gio.NewMenu()
	tracking.SetFinalizer("Menu", menu)

	queueAllItem := gio.NewMenuItem(gettext.Get("Add My Tracks to Queue"), "win.player.queue")
	queueAllItem.SetActionAndTargetValue("win.player.queue", glib.NewVariantString("my_collection/tracks"))
	menu.AppendItem(queueAllItem)
	tracking.SetFinalizer("MenuItem", queueAllItem)

	popover := gtk.NewPopoverMenuFromModel(&menu.MenuModel)
	tracking.SetFinalizer("Popover", popover)

	return template(cover, title, subtitle, "", componentControls(playFunc, shuffleFunc), secondaryControlsCollection(playbackSource, popover))
}
