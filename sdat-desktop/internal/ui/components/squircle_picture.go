package components

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func SquirclePicture(picture schwifty.Picture) schwifty.Picture {
	return picture.
		Background("alpha(var(--view-fg-color), 0.1)").
		Overflow(gtk.OverflowHiddenValue).CornerRadius(10)
}
