package tracklist

import (
	"strconv"

	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func PositionColumn(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
	widget := Label(strconv.Itoa(position + 1)).
		FontWeight(500).
		HAlign(gtk.AlignStartValue).
		HExpand(false).
		Margin(10).HPadding(10)
	grid.Attach(widget.ToGTK(), column, 0, 1, 1)
	return 1
}
