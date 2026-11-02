package callback

import "codeberg.org/puregotk/puregotk/v4/gtk"

var (
	ScrolledWindowEdgeReachedCallback = func(widget gtk.ScrolledWindow, positionType gtk.PositionType) {
		CallbackHandler[any](widget.Object, "edge-reached", widget, positionType)
	}
)
