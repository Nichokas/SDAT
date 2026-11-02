package weak

import (
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type widget interface {
	gObject
	GetRealized() bool
}

type WidgetRef = weakRef[widget, *gtk.Widget]

type widgetRef struct {
	ref ObjectRef
}

func (x *widgetRef) Clear() {
	x.ref.Clear()
}

func (x *widgetRef) Get() *gtk.Widget {
	obj := x.ref.Get()
	if obj == nil {
		return nil
	}

	widget := gtk.WidgetNewFromInternalPtr(obj.Ptr)
	if !widget.GetRealized() {
		widget.Unref()
		return nil
	}

	return widget
}

func (x *widgetRef) Init(widget widget) {
	x.ref.Init(widget)
}

func (x *widgetRef) Set(widget widget) {
	x.ref.Set(widget)
}

func (x *widgetRef) Use(cb func(widget *gtk.Widget)) bool {
	if widget := x.Get(); widget != nil {
		cb(widget)
		widget.Unref()
		return true
	}
	return false
}

func NewWidgetRef(obj widget) WidgetRef {
	x := &widgetRef{
		ref: NewObjectRef(obj),
	}
	return x
}
