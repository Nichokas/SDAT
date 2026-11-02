package syntax

import (
	gtkbindings "codeberg.org/dergs/tonearm/pkg/schwifty/bindings/gtk"
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gdk"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

func AspectFrame(child any) gtkbindings.AspectFrame {
	return managedWidget("Scale", func() *gtk.AspectFrame {
		aspectFrame := gtk.NewAspectFrame(0.5, 0.5, 1.0, false)
		aspectFrame.SetChild(gtkbindings.ResolveWidget(child))
		return aspectFrame
	})
}

func Box(orientation gtk.Orientation, children ...any) gtkbindings.Box {
	return managedWidget("Box", func() *gtk.Box {
		box := gtk.NewBox(orientation, 0)
		for _, child := range children {
			box.Append(gtkbindings.ResolveWidget(child))
		}
		return box
	})
}

func HStack(children ...any) gtkbindings.Box {
	return Box(gtk.OrientationHorizontalValue, children...)
}

func Spacer() gtkbindings.Box {
	return HStack().VExpand(true).HExpand(true)
}

func VStack(children ...any) gtkbindings.Box {
	return Box(gtk.OrientationVerticalValue, children...)
}

func Button() gtkbindings.Button {
	return managedWidget("Button", func() *gtk.Button {
		btn := gtk.NewButton()
		btn.ConnectClicked(&callback.ButtonClickedCallback)
		return btn
	})
}

func CenterBox() gtkbindings.CenterBox {
	return managedWidget("CenterBox", func() *gtk.CenterBox {
		return gtk.NewCenterBox()
	})
}

func DragSource() gtkbindings.DragSource {
	return managedObject("DragSource", func() *gtk.DragSource {
		dragSource := gtk.NewDragSource()
		dragSource.ConnectDragBegin(&callback.DragSourceDragBegin)
		dragSource.ConnectDragCancel(&callback.DragSourceDragCancel)
		dragSource.ConnectPrepare(&callback.DragSourcePrepare)
		return dragSource
	})
}

func DropTargetAsync(formats *gdk.ContentFormats, actions gdk.DragAction) gtkbindings.DropTargetAsync {
	return managedObject("DropTargetAsync", func() *gtk.DropTargetAsync {
		dragSource := gtk.NewDropTargetAsync(formats, actions)
		dragSource.ConnectAccept(&callback.DropTargetAsyncAccept)
		dragSource.ConnectDragMotion(&callback.DropTargetAsyncDragMotion)
		dragSource.ConnectDrop(&callback.DropTargetAsyncDrop)
		return dragSource
	})
}

func Grid() gtkbindings.Grid {
	return managedWidget("Grid", func() *gtk.Grid {
		return gtk.NewGrid()
	})
}

func Image() gtkbindings.Image {
	return managedWidget("Image", func() *gtk.Image {
		return gtk.NewImage()
	})
}

func Label(text string) gtkbindings.Label {
	return managedWidget("Label", func() *gtk.Label {
		return gtk.NewLabel(text)
	})
}

func MenuButton() gtkbindings.MenuButton {
	return managedWidget("MenuButton", func() *gtk.MenuButton {
		return gtk.NewMenuButton()
	})
}

func Overlay(child any) gtkbindings.Overlay {
	return managedWidget("Overlay", func() *gtk.Overlay {
		overlay := gtk.NewOverlay()
		overlay.SetChild(gtkbindings.ResolveWidget(child))
		return overlay
	})
}

func ProgressBar() gtkbindings.ProgressBar {
	return managedWidget("ProgressBar", func() *gtk.ProgressBar {
		return gtk.NewProgressBar()
	})
}

func Picture() gtkbindings.Picture {
	return managedWidget("Picture", func() *gtk.Picture {
		return gtk.NewPicture()
	})
}

func Popover(child any) gtkbindings.Popover {
	return managedWidget("Popover", func() *gtk.Popover {
		popover := gtk.NewPopover()
		popover.SetChild(gtkbindings.ResolveWidget(child))
		return popover
	})
}

func Revealer(child any) gtkbindings.Revealer {
	return managedWidget("Revealer", func() *gtk.Revealer {
		revealer := gtk.NewRevealer()
		revealer.SetChild(gtkbindings.ResolveWidget(child))
		return revealer
	})
}

func Scale(orientation gtk.Orientation) gtkbindings.Scale {
	return managedWidget("Scale", func() *gtk.Scale {
		scale := gtk.NewScale(orientation, nil)
		scale.ConnectChangeValue(&callback.RangeChangeValueCallback)
		return scale
	})
}

func ShortcutController() gtkbindings.ShortcutController {
	return managedObject("ShortcutController", func() *gtk.ShortcutController {
		controller := gtk.NewShortcutController()
		return controller
	})
}

func ScrolledWindow() gtkbindings.ScrolledWindow {
	return managedWidget("ScrolledWindow", func() *gtk.ScrolledWindow {
		scrolledWindow := gtk.NewScrolledWindow()
		scrolledWindow.ConnectEdgeReached(&callback.ScrolledWindowEdgeReachedCallback)
		return scrolledWindow
	})
}

func SearchEntry() gtkbindings.SearchEntry {
	return managedWidget("SearchEntry", func() *gtk.SearchEntry {
		searchEntry := gtk.NewSearchEntry()
		searchEntry.ConnectActivate(&callback.SearchEntryActivateCallback)
		searchEntry.ConnectSearchChanged(&callback.SearchChangedCallback)
		return searchEntry
	})
}

func Widget(w *gtk.Widget) gtkbindings.Widget {
	return func() *gtkbindings.WrappedWidget {
		return &gtkbindings.WrappedWidget{Widget: *w}
	}
}

// WARN: Do not manage reference counting for a schwifty-managed widget. If you are not in control of the widget's lifecycle, use Widget() instead.
func ManagedWidget(w *gtk.Widget) gtkbindings.Widget {
	return managedWidget("ManagedWidget", func() *gtkbindings.WrappedWidget {
		return &gtkbindings.WrappedWidget{Widget: *w}
	})
}
