package components

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty"
	. "codeberg.org/dergs/tonearm/pkg/schwifty/syntax"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func MainContent(body schwifty.BaseWidgetable) schwifty.BaseWidgetable {
	return Clamp().Orientation(gtk.OrientationHorizontalValue).MaximumSize(2000).TighteningThreshold(2000).Child(body)
}

// MainContentScrollable mirrors MainContent but uses AdwClampScrollable so the
// clamp passes through the GtkScrollable interface.
// If we don't do that, we break virtualized row rendering
func MainContentScrollable(body schwifty.BaseWidgetable) schwifty.BaseWidgetable {
	return ClampScrollable().Orientation(gtk.OrientationHorizontalValue).MaximumSize(2000).TighteningThreshold(2000).Child(body)
}
