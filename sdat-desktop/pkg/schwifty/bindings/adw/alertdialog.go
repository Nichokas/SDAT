package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/adw"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen AlertDialog *adw.AlertDialog adw

func (f AlertDialog) CanClose(canClose bool) AlertDialog {
	return func() *adw.AlertDialog {
		dialog := f()
		dialog.SetCanClose(canClose)
		return dialog
	}
}

func (f AlertDialog) ConnectClosed(cb func(adw.Dialog)) AlertDialog {
	return func() *adw.AlertDialog {
		dialog := f()
		callback.HandleCallback(dialog.Object, "closed", cb)
		return dialog
	}
}

func (f AlertDialog) ConnectCloseAttempt(cb func(adw.Dialog)) AlertDialog {
	return func() *adw.AlertDialog {
		dialog := f()
		callback.HandleCallback(dialog.Object, "close-attempt", cb)
		return dialog
	}
}

func (f AlertDialog) ConnectResponse(cb func(adw.AlertDialog, string)) AlertDialog {
	return func() *adw.AlertDialog {
		dialog := f()
		callback.HandleCallback(dialog.Object, "response", cb)
		return dialog
	}
}

func (f AlertDialog) ExtraChild(widget any) AlertDialog {
	return func() *adw.AlertDialog {
		dialog := f()
		dialog.SetExtraChild(gtk.ResolveWidget(widget))
		return dialog
	}
}

func (f AlertDialog) CloseResponseID(id string) AlertDialog {
	return func() *adw.AlertDialog {
		dialog := f()
		dialog.SetCloseResponse(id)
		return dialog
	}
}

type AlertDialogResponse struct {
	ID         string
	Label      string
	Appearance adw.ResponseAppearance
}

func (f AlertDialog) WithResponse(response AlertDialogResponse) AlertDialog {
	return func() *adw.AlertDialog {
		dialog := f()
		dialog.AddResponse(response.ID, response.Label)
		dialog.SetResponseAppearance(response.ID, response.Appearance)
		return dialog
	}
}
