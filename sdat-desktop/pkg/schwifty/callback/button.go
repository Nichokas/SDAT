package callback

import "codeberg.org/puregotk/puregotk/v4/gtk"

var (
	ButtonClickedCallback = func(widget gtk.Button) {
		CallbackHandler[any](widget.Object, "clicked", widget)
	}
)
