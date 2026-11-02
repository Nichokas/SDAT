package schwifty

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

type TEMPLATE_BASE_TYPE struct{ gtk.Widget }

type TEMPLATE_TYPE func() TEMPLATE_BASE_TYPE

func (f TEMPLATE_TYPE) AddController(controller *gtk.EventController) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectConstruct(cb func(TEMPLATE_BASE_TYPE)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectDestroy(cb func(gtk.Widget)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectHide(cb func(gtk.Widget)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectMap(cb func(gtk.Widget)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectRealize(cb func(gtk.Widget)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectShow(cb func(gtk.Widget)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectUnmap(cb func(gtk.Widget)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f TEMPLATE_TYPE) ConnectUnrealize(cb func(gtk.Widget)) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f TEMPLATE_TYPE) Controller(controller *gtk.EventController) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f TEMPLATE_TYPE) Focusable(focusable bool) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f TEMPLATE_TYPE) FocusOnClick(focusOnClick bool) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f TEMPLATE_TYPE) HAlign(align gtk.Align) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f TEMPLATE_TYPE) HExpand(expand bool) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f TEMPLATE_TYPE) HMargin(horizontal int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f TEMPLATE_TYPE) Margin(margin int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f TEMPLATE_TYPE) MarginBottom(bottom int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f TEMPLATE_TYPE) MarginEnd(end int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f TEMPLATE_TYPE) MarginStart(start int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f TEMPLATE_TYPE) MarginTop(top int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f TEMPLATE_TYPE) Opacity(opacity float64) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f TEMPLATE_TYPE) Overflow(overflow gtk.Overflow) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f TEMPLATE_TYPE) Sensitive(sensitive bool) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f TEMPLATE_TYPE) SizeRequest(width, height int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f TEMPLATE_TYPE) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f TEMPLATE_TYPE) VAlign(align gtk.Align) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f TEMPLATE_TYPE) VExpand(expand bool) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f TEMPLATE_TYPE) Visible(visible bool) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f TEMPLATE_TYPE) VMargin(vertical int32) TEMPLATE_TYPE {
	return func() TEMPLATE_BASE_TYPE {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}
