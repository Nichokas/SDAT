package adw

import (
	gtkbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen ClampScrollable *adw.ClampScrollable adw

func (f ClampScrollable) Child(widget any) ClampScrollable {
	return func() *adw.ClampScrollable {
		clamp := f()
		clamp.SetChild(gtkbindings.ResolveWidget(widget))
		return clamp
	}
}

func (f ClampScrollable) MaximumSize(size int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		clamp := f()
		clamp.SetMaximumSize(size)
		return clamp
	}
}

func (f ClampScrollable) Orientation(orientation gtk.Orientation) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetOrientation(orientation)
		return widget
	}
}

func (f ClampScrollable) TighteningThreshold(threshold int32) ClampScrollable {
	return func() *adw.ClampScrollable {
		widget := f()
		widget.SetTighteningThreshold(threshold)
		return widget
	}
}
