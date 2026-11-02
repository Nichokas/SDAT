package tracklist

import (
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func groupedColumn(columns []any, width int32, grid *gtk.Grid, row int32, column int32) int32 {
	grid.Attach(
		HStack(columns...).
			ToGTK(),
		column,
		row,
		width,
		1,
	)
	return width
}

func GroupedColumn(width int32, align gtk.Align, columns ...ColumnFunc) ColumnFunc {
	return func(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
		subGrid := gtk.NewGrid()
		subGrid.SetValign(gtk.AlignCenterValue)
		subGrid.SetHalign(align)
		subWidth := 0
		for i, c := range columns {
			subWidth += c(track, subGrid, position, column+int32(i))
		}
		defer subGrid.Unref()
		grid.Attach(
			&subGrid.Widget,
			column,
			0,
			width,
			1,
		)
		return int(width)
	}
}
