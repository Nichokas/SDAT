package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type SearchEntry func() *gtk.SearchEntry

func (f SearchEntry) AddController(controller *gtk.EventController) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f SearchEntry) ConnectConstruct(cb func(*gtk.SearchEntry)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f SearchEntry) ConnectDestroy(cb func(gtk.Widget)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f SearchEntry) ConnectHide(cb func(gtk.Widget)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f SearchEntry) ConnectMap(cb func(gtk.Widget)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f SearchEntry) ConnectRealize(cb func(gtk.Widget)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f SearchEntry) ConnectShow(cb func(gtk.Widget)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f SearchEntry) ConnectUnmap(cb func(gtk.Widget)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f SearchEntry) ConnectUnrealize(cb func(gtk.Widget)) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f SearchEntry) Controller(controller *gtk.EventController) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f SearchEntry) Focusable(focusable bool) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f SearchEntry) FocusOnClick(focusOnClick bool) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f SearchEntry) HAlign(align gtk.Align) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f SearchEntry) HExpand(expand bool) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f SearchEntry) HMargin(horizontal int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f SearchEntry) Margin(margin int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f SearchEntry) MarginBottom(bottom int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f SearchEntry) MarginEnd(end int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f SearchEntry) MarginStart(start int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f SearchEntry) MarginTop(top int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f SearchEntry) Opacity(opacity float64) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f SearchEntry) Overflow(overflow gtk.Overflow) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f SearchEntry) Sensitive(sensitive bool) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f SearchEntry) SizeRequest(width, height int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f SearchEntry) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f SearchEntry) VAlign(align gtk.Align) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f SearchEntry) VExpand(expand bool) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f SearchEntry) Visible(visible bool) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f SearchEntry) VMargin(vertical int32) SearchEntry {
	return func() *gtk.SearchEntry {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f SearchEntry) Background(color string) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f SearchEntry) CornerRadius(radius int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f SearchEntry) CSS(css string) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f SearchEntry) BindCSSClass(state *state.State[string]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue string) {
				oldValue := state.Value()
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()

						w := gtk.WidgetNewFromInternalPtr(obj.Ptr)
						styleContext := w.GetStyleContext()
						defer styleContext.Unref()

						styleContext.RemoveClass(oldValue)
						styleContext.AddClass(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) WithCSSClass(className string) SearchEntry {
	return func() *gtk.SearchEntry {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f SearchEntry) CSSWithCallback(cb func(elementName string) string) SearchEntry {
	return func() *gtk.SearchEntry {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.SearchEntry) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f SearchEntry) HPadding(padding int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f SearchEntry) MinHeight(minHeight int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f SearchEntry) MinWidth(minWidth int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f SearchEntry) Padding(padding int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f SearchEntry) PaddingBottom(padding int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f SearchEntry) PaddingEnd(padding int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f SearchEntry) PaddingStart(padding int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f SearchEntry) PaddingTop(padding int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f SearchEntry) VPadding(padding int) SearchEntry {
	return func() *gtk.SearchEntry {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f SearchEntry) BindVisible(state *state.State[bool]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.SearchEntry) {
			ref = weak.NewObjectRef(&w.Widget)
			callbackId = state.AddCallback(func(newValue bool) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetVisible(newValue)
					}
				})
			})
		}).ConnectDestroy(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) BindHMargin(state *state.State[int32]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginEnd(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginStart(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) BindMargin(state *state.State[int32]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginBottom(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginEnd(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginStart(newValue)
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginTop(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) BindMarginBottom(state *state.State[int32]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginBottom(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) BindMarginEnd(state *state.State[int32]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginEnd(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) BindMarginStart(state *state.State[int32]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginStart(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) BindMarginTop(state *state.State[int32]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue int32) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetMarginTop(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}

func (f SearchEntry) BindSensitive(state *state.State[bool]) SearchEntry {
	return func() *gtk.SearchEntry {
		var callbackId string
		var ref weak.WidgetRef
		return f.ConnectRealize(func(w gtk.Widget) {
			ref = weak.NewWidgetRef(&w)
			callbackId = state.AddCallback(func(newValue bool) {
				callback.OnMainThreadOncePure(func() {
					if obj := ref.Get(); obj != nil {
						defer obj.Unref()
						gtk.WidgetNewFromInternalPtr(obj.Ptr).SetSensitive(newValue)
					}
				})
			})
		}).ConnectUnrealize(func(w gtk.Widget) {
			state.RemoveCallback(callbackId)
		})()
	}
}
