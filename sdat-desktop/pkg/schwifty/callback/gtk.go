package callback

import (
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

var (
	AdjustmentValueChangedCallback = func(adj gtk.Adjustment) {
		CallbackHandler[any](adj.Object, "value-changed", adj)
	}
	AdjustmentChangedCallback = func(adj gtk.Adjustment) {
		CallbackHandler[any](adj.Object, "changed", adj)
	}

	DragSourcePrepare = func(dragSource gtk.DragSource, x float64, y float64) gdk.ContentProvider {
		results := CallbackHandler[gdk.ContentProvider](dragSource.Object, "prepare", dragSource, x, y)
		if len(results) > 0 {
			for _, result := range results {
				if result.GoPointer() > 0 {
					return result
				}
			}
		}
		panic("DragSource did not have a valid content provider")
	}
	DragSourceDragBegin = func(dragSource gtk.DragSource, dragPtr uintptr) {
		drag := gdk.DragNewFromInternalPtr(dragPtr)
		CallbackHandler[any](dragSource.Object, "drag-begin", dragSource, *drag)
	}
	DragSourceDragCancel = func(dragSource gtk.DragSource, dragPtr uintptr, reason gdk.DragCancelReason) bool {
		drag := gdk.DragNewFromInternalPtr(dragPtr)
		results := CallbackHandler[bool](dragSource.Object, "drag-cancel", dragSource, *drag, reason)
		if len(results) > 0 {
			for _, result := range results {
				if result {
					return true
				}
			}
		}
		return false
	}

	DropTargetAsyncAccept = func(dropTarget gtk.DropTargetAsync, dropPtr uintptr) bool {
		drop := gdk.DropNewFromInternalPtr(dropPtr)
		results := CallbackHandler[bool](dropTarget.Object, "accept", dropTarget, *drop)
		if len(results) > 0 {
			for _, result := range results {
				if result {
					return result
				}
			}
		}
		return false
	}
	DropTargetAsyncDragMotion = func(dropTarget gtk.DropTargetAsync, dropPtr uintptr, x, y float64) gdk.DragAction {
		drop := gdk.DropNewFromInternalPtr(dropPtr)
		results := CallbackHandler[gdk.DragAction](dropTarget.Object, "drag-motion", dropTarget, *drop, x, y)
		if len(results) > 0 {
			for _, result := range results {
				if result != gdk.ActionNoneValue {
					return result
				}
			}
		}
		return gdk.ActionNoneValue
	}
	DropTargetAsyncDrop = func(dropTarget gtk.DropTargetAsync, dropPtr uintptr, x, y float64) bool {
		drop := gdk.DropNewFromInternalPtr(dropPtr)
		results := CallbackHandler[bool](dropTarget.Object, "drop", dropTarget, *drop, x, y)
		if len(results) > 0 {
			for _, result := range results {
				if result {
					return result
				}
			}
		}
		return false
	}

	ListViewActivate = func(listView gtk.ListView, index uint32) {
		CallbackHandler[any](listView.Object, "activate", listView, index)
	}
)
