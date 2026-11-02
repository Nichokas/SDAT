package gtk

import "codeberg.org/puregotk/puregotk/v4/gtk"

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Grid *gtk.Grid gtk

func (f Grid) Attach(child any, column int32, row int32, width int32, height int32) Grid {
	return func() *gtk.Grid {
		grid := f()
		grid.Attach(ResolveWidget(child), column, row, width, height)
		return grid
	}
}

func (f Grid) ColumnHomogeneous(homogeneous bool) Grid {
	return func() *gtk.Grid {
		grid := f()
		grid.SetColumnHomogeneous(homogeneous)
		return grid
	}
}
