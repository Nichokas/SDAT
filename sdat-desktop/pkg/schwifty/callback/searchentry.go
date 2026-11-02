package callback

import "codeberg.org/puregotk/puregotk/v4/gtk"

var (
	SearchEntryActivateCallback = func(widget gtk.SearchEntry) {
		CallbackHandler[any](widget.Object, "activate", widget)
	}
	SearchChangedCallback = func(widget gtk.SearchEntry) {
		CallbackHandler[any](widget.Object, "search-changed", widget)
	}
)
