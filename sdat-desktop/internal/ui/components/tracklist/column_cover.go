package tracklist

import (
	"codeberg.org/dergs/tonearm/internal/resources"
	"codeberg.org/dergs/tonearm/internal/settings"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/dergs/tonearm/pkg/utils/imgutil"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"github.com/infinytum/injector"
)

func CoverColumn(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
	grid.Attach(
		Image().
			FromPaintable(resources.MissingAlbum()).
			PixelSize(54).
			HExpand(false).
			VExpand(false).
			CornerRadius(10).
			Margin(10).
			HAlign(gtk.AlignStartValue).
			Overflow(gtk.OverflowHiddenValue).
			ConnectRealize(func(i gtk.Widget) {
				cover := track.Cover(80)
				if settings.Performance().AllowTracklistImages() && cover.HasStatic() {
					injector.MustInject[*imgutil.ImgUtil]().LoadIntoImage(cover.Static(), gtk.ImageNewFromInternalPtr(i.Ptr))
				}
			}).ToGTK(),
		column,
		0,
		1,
		1,
	)
	return 1
}
