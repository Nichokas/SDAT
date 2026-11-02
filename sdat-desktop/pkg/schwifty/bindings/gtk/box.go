package gtk

import "codeberg.org/puregotk/puregotk/v4/gtk"

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Box *gtk.Box gtk

func (f Box) Append(child any) Box {
	return func() *gtk.Box {
		box := f()
		box.Append(ResolveWidget(child))
		return box
	}
}

func (f Box) Orientation(orientation gtk.Orientation) Box {
	return func() *gtk.Box {
		box := f()
		box.SetOrientation(orientation)
		return box
	}
}

func (f Box) Spacing(spacing int32) Box {
	return func() *gtk.Box {
		box := f()
		box.SetSpacing(spacing)
		return box
	}
}
