package callback

import "codeberg.org/puregotk/puregotk/v4/gtk"

var (
	DestroyCallback = func(widget gtk.Widget) {
		CallbackHandler[any](widget.Object, "destroy", widget)
	}
	HideCallback = func(widget gtk.Widget) {
		CallbackHandler[any](widget.Object, "hide", widget)
	}
	MapCallback = func(widget gtk.Widget) {
		CallbackHandler[any](widget.Object, "map", widget)
	}
	RealizedCallback = func(widget gtk.Widget) {
		CallbackHandler[any](widget.Object, "realize", widget)
	}
	ShowCallback = func(widget gtk.Widget) {
		CallbackHandler[any](widget.Object, "show", widget)
	}
	UnmapCallback = func(widget gtk.Widget) {
		CallbackHandler[any](widget.Object, "unmap", widget)
	}
	UnrealizedCallback = func(widget gtk.Widget) {
		CallbackHandler[any](widget.Object, "unrealize", widget)
	}
)
