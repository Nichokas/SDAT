package syntax

import (
	"runtime/debug"

	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/tracking"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type manageableObject interface {
	tracking.Trackable
}

type manageableWidget interface {
	tracking.Trackable
	ConnectDestroy(cb *func(gtk.Widget)) uint32
	ConnectHide(cb *func(gtk.Widget)) uint32
	ConnectMap(cb *func(gtk.Widget)) uint32
	ConnectRealize(cb *func(gtk.Widget)) uint32
	ConnectShow(cb *func(gtk.Widget)) uint32
	ConnectUnmap(cb *func(gtk.Widget)) uint32
	ConnectUnrealize(cb *func(gtk.Widget)) uint32
}

func managedWidget[InnerType manageableWidget, T func() InnerType](tag string, fn T) T {
	// Track where the schwifty expression was originally created
	// as it may be actually constructed somewhere entirely different
	// when getting rendered.
	creationStack := debug.Stack()

	return func() InnerType {
		widget := fn()
		widget.ConnectDestroy(&callback.DestroyCallback)
		widget.ConnectHide(&callback.HideCallback)
		widget.ConnectMap(&callback.MapCallback)
		widget.ConnectRealize(&callback.RealizedCallback)
		widget.ConnectShow(&callback.ShowCallback)
		widget.ConnectUnmap(&callback.UnmapCallback)
		widget.ConnectUnrealize(&callback.UnrealizedCallback)
		tracking.SetFinalizer(tag, widget)
		tracking.TrackStack(widget.GoPointer(), creationStack)
		return widget
	}
}

func managedObject[InnerType manageableObject, T func() InnerType](tag string, fn T) T {
	return func() InnerType {
		widget := fn()
		tracking.SetFinalizer(tag, widget)
		return widget
	}
}
