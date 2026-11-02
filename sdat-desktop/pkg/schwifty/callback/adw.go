package callback

import (
	"codeberg.org/dergs/tonearm/pkg/utils/cutil"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

var (
	AlertDialogClosed = func(widget adw.Dialog) {
		CallbackHandler[any](widget.Object, "closed", widget)
	}
	AlertDialogCloseAttempt = func(widget adw.Dialog) {
		CallbackHandler[any](widget.Object, "close-attempt", widget)
	}
	AlertDialogResponse = func(widget adw.AlertDialog, response string) {
		response = cutil.ParseNullTerminatedString(response)
		CallbackHandler[string](widget.Object, "response", widget, response)
	}
)

var (
	ActionRowActivated = func(widget adw.ActionRow) {
		CallbackHandler[any](widget.Object, "activated", widget)
	}
)
