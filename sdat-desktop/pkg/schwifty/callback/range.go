package callback

import "codeberg.org/puregotk/puregotk/v4/gtk"

var (
	RangeChangeValueCallback = func(widget gtk.Range, scrollType gtk.ScrollType, value float64) bool {
		results := CallbackHandler[bool](widget.Object, "change-value", widget, scrollType, value)
		if len(results) > 0 {
			for _, result := range results {
				if result {
					return true
				}
			}
		}
		return false
	}
)
