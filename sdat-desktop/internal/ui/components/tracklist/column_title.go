package tracklist

import (
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"codeberg.org/puregotk/puregotk/v4/pango"
)

func TitleColumn(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
	widget := Label(tonearm.FormatTitle(track)).FontWeight(500).Ellipsis(pango.EllipsizeEndValue).HAlign(gtk.AlignStartValue).HExpand(true).Margin(10)
	grid.Attach(widget.ToGTK(), column, 0, 1, 1)
	return 1
}
