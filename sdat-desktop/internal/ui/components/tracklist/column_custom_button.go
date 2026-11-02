package tracklist

import (
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/dergs/tonearm/pkg/tonearm"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type buttonColumnCallback func(trackID string, position int, column int32)

func customButtonColumn(trackId string, grid *gtk.Grid, position int, column int32, onClick buttonColumnCallback) int {
	grid.Attach(
		Button().
			ConnectClicked(func(b gtk.Button) {
				onClick(trackId, position, column)
			}).
			WithCSSClass("flat").
			ToGTK(),
		0,
		0,
		column,
		1,
	)
	return 0
}

func customWidgetButtonColumn(trackId string, grid *gtk.Grid, position int, column int32, button func(trackID string, position int, column int32) *gtk.Widget) int {
	grid.Attach(
		HStack(
			button(trackId, position, column),
		).
			HAlign(gtk.AlignCenterValue).
			VAlign(gtk.AlignCenterValue).
			ToGTK(),
		column,
		0,
		1,
		1,
	)
	return 0
}

func CustomButtonColumn(onClick buttonColumnCallback) ColumnFunc {
	return func(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
		if track == nil {
			grid.Attach(
				Box(gtk.OrientationHorizontalValue).ToGTK(),
				0,
				0,
				column,
				1,
			)
			return 1
		}
		return customButtonColumn(track.ID(), grid, position, column, onClick)
	}
}

func CustomWidgetButtonColumn(button func(string, int, int32) *gtk.Widget) ColumnFunc {
	return func(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
		if track == nil {
			grid.Attach(
				Box(gtk.OrientationHorizontalValue).ToGTK(),
				column,
				0,
				1,
				1,
			)
			return 1
		}
		return customWidgetButtonColumn(track.ID(), grid, position, column, button)
	}
}

func ExpandCustomButtonColumn(additionalWidth int32, onClick buttonColumnCallback) ColumnFunc {
	return func(track tonearm.Track, grid *gtk.Grid, position int, column int32) int {
		return CustomButtonColumn(onClick)(track, grid, position, column+additionalWidth)
	}
}
