package adw

import (
	gtkbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Clamp *adw.Clamp adw

func (f Clamp) Child(widget any) Clamp {
	return func() *adw.Clamp {
		clamp := f()
		clamp.SetChild(gtkbindings.ResolveWidget(widget))
		return clamp
	}
}

func (f Clamp) MaximumSize(size int32) Clamp {
	return func() *adw.Clamp {
		clamp := f()
		clamp.SetMaximumSize(size)
		return clamp
	}
}

func (f Clamp) Orientation(orientation gtk.Orientation) Clamp {
	return func() *adw.Clamp {
		widget := f()
		widget.SetOrientation(orientation)
		return widget
	}
}

func (f Clamp) TighteningThreshold(threshold int32) Clamp {
	return func() *adw.Clamp {
		widget := f()
		widget.SetTighteningThreshold(threshold)
		return widget
	}
}
