package tracklist_header

import (
	"codeberg.org/dergs/tonearm/internal/resources"
	"codeberg.org/dergs/tonearm/internal/ui/components"
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func componentCover(cover tonearm.Cover) schwifty.AspectFrame {
	coverState := state.New[schwifty.Paintable](resources.MissingAlbum())
	return AspectFrame(
		Image().
			PixelSize(146).
			BindPaintable(coverState).
			ConnectRealize(func(w gtk.Widget) {
				components.RenderCover(cover, coverState)
			}),
	).CornerRadius(10).Overflow(gtk.OverflowHiddenValue).WithCSSClass("cover")
}
