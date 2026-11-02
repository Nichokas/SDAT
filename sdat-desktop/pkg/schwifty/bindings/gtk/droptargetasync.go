package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type DropTargetAsync func() *gtk.DropTargetAsync

func (f DropTargetAsync) ConnectAccept(cb func(dropTarget gtk.DropTargetAsync, drop gdk.Drop) bool) DropTargetAsync {
	return func() *gtk.DropTargetAsync {
		dropTarget := f()
		callback.HandleCallback(dropTarget.Object, "accept", cb)
		return dropTarget
	}
}

func (f DropTargetAsync) ConnectDragMotion(cb func(dropTarget gtk.DropTargetAsync, drop gdk.Drop, x, y float64) gdk.DragAction) DropTargetAsync {
	return func() *gtk.DropTargetAsync {
		dropTarget := f()
		callback.HandleCallback(dropTarget.Object, "drag-motion", cb)
		return dropTarget
	}
}

func (f DropTargetAsync) ConnectDrop(cb func(dropTarget gtk.DropTargetAsync, drop gdk.Drop, x, y float64) bool) DropTargetAsync {
	return func() *gtk.DropTargetAsync {
		dropTarget := f()
		callback.HandleCallback(dropTarget.Object, "drop", cb)
		return dropTarget
	}
}
