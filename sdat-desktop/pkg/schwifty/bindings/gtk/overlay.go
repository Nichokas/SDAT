package gtk

import (
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Overlay *gtk.Overlay gtk

func (f Overlay) AddOverlay(widget any) Overlay {
	return func() *gtk.Overlay {
		bin := f()
		bin.AddOverlay(ResolveWidget(widget))
		return bin
	}
}

func (f Overlay) Child(widget any) Overlay {
	return func() *gtk.Overlay {
		bin := f()
		bin.SetChild(ResolveWidget(widget))
		return bin
	}
}
