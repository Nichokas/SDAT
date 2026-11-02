package tracklist_header

import (
	"codeberg.org/dergs/tonearm/internal/gettext"
	"codeberg.org/dergs/tonearm/internal/notifications"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func componentSecondaryControls(shareable tonearm.Shareable, popover *gtk.PopoverMenu, buttons ...any) schwifty.Box {
	buttons = append(buttons, Button().
		TooltipText(gettext.Get("Copy URL")).
		IconName("share-alt-symbolic").
		WithCSSClass("flat").
		ConnectClicked(func(gtk.Button) {
			display := gdk.DisplayGetDefault()
			defer display.Unref()
			clipboard := display.GetClipboard()
			defer clipboard.Unref()

			clipboard.SetText(shareable.URL())
			notifications.OnToast.Notify(gettext.Get("Copied URL to clipboard"))
		}),
		MenuButton().
			TooltipText(gettext.Get("More…")).
			Popover(popover).
			WithCSSClass("flat").
			WithCSSClass("circular").
			IconName("view-more-symbolic"))
	return HStack(
		buttons...,
	).Spacing(12).HAlign(gtk.AlignEndValue).HExpand(true).VAlign(gtk.AlignCenterValue)
}
