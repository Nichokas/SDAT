package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type DragSource func() *gtk.DragSource

func (f DragSource) Actions(actions gdk.DragAction) DragSource {
	return func() *gtk.DragSource {
		dragSource := f()
		dragSource.SetActions(actions)
		return dragSource
	}
}

func (f DragSource) ConnectPrepare(cb func(dragSource gtk.DragSource, x float64, y float64) gdk.ContentProvider) DragSource {
	return func() *gtk.DragSource {
		dragSource := f()
		callback.HandleCallback(dragSource.Object, "prepare", cb)
		return dragSource
	}
}

func (f DragSource) ConnectDragBegin(cb func(dragSource gtk.DragSource, drag gdk.Drag)) DragSource {
	return func() *gtk.DragSource {
		dragSource := f()
		callback.HandleCallback(dragSource.Object, "drag-begin", cb)
		return dragSource
	}
}

func (f DragSource) ConnectDragCancel(cb func(dragSource gtk.DragSource, drag gdk.Drag, reason gdk.DragCancelReason) bool) DragSource {
	return func() *gtk.DragSource {
		dragSource := f()
		callback.HandleCallback(dragSource.Object, "drag-cancel", cb)
		return dragSource
	}
}
