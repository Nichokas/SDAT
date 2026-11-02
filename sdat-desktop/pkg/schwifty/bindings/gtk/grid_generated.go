package gtk

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Grid func() *gtk.Grid

func (f Grid) AddController(controller *gtk.EventController) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Grid) ConnectConstruct(cb func(*gtk.Grid)) Grid {
	return func() *gtk.Grid {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Grid) ConnectDestroy(cb func(gtk.Widget)) Grid {
	return func() *gtk.Grid {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Grid) ConnectHide(cb func(gtk.Widget)) Grid {
	return func() *gtk.Grid {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Grid) ConnectMap(cb func(gtk.Widget)) Grid {
	return func() *gtk.Grid {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Grid) ConnectRealize(cb func(gtk.Widget)) Grid {
	return func() *gtk.Grid {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Grid) ConnectShow(cb func(gtk.Widget)) Grid {
	return func() *gtk.Grid {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Grid) ConnectUnmap(cb func(gtk.Widget)) Grid {
	return func() *gtk.Grid {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Grid) ConnectUnrealize(cb func(gtk.Widget)) Grid {
	return func() *gtk.Grid {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Grid) Controller(controller *gtk.EventController) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Grid) Focusable(focusable bool) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Grid) FocusOnClick(focusOnClick bool) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Grid) HAlign(align gtk.Align) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Grid) HExpand(expand bool) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Grid) HMargin(horizontal int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Grid) Margin(margin int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Grid) MarginBottom(bottom int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Grid) MarginEnd(end int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Grid) MarginStart(start int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Grid) MarginTop(top int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Grid) Opacity(opacity float64) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Grid) Overflow(overflow gtk.Overflow) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Grid) Sensitive(sensitive bool) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Grid) SizeRequest(width, height int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Grid) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Grid) VAlign(align gtk.Align) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Grid) VExpand(expand bool) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Grid) Visible(visible bool) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Grid) VMargin(vertical int32) Grid {
	return func() *gtk.Grid {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Grid) Background(color string) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Grid) CornerRadius(radius int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Grid) CSS(css string) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Grid) BindCSSClass(state *state.State[string]) Grid {
	return func() *gtk.Grid {
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

func (f Grid) WithCSSClass(className string) Grid {
	return func() *gtk.Grid {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Grid) CSSWithCallback(cb func(elementName string) string) Grid {
	return func() *gtk.Grid {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *gtk.Grid) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Grid) HPadding(padding int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Grid) MinHeight(minHeight int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Grid) MinWidth(minWidth int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Grid) Padding(padding int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Grid) PaddingBottom(padding int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Grid) PaddingEnd(padding int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Grid) PaddingStart(padding int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Grid) PaddingTop(padding int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Grid) VPadding(padding int) Grid {
	return func() *gtk.Grid {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Grid) BindVisible(state *state.State[bool]) Grid {
	return func() *gtk.Grid {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *gtk.Grid) {
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

func (f Grid) BindHMargin(state *state.State[int32]) Grid {
	return func() *gtk.Grid {
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

func (f Grid) BindMargin(state *state.State[int32]) Grid {
	return func() *gtk.Grid {
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

func (f Grid) BindMarginBottom(state *state.State[int32]) Grid {
	return func() *gtk.Grid {
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

func (f Grid) BindMarginEnd(state *state.State[int32]) Grid {
	return func() *gtk.Grid {
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

func (f Grid) BindMarginStart(state *state.State[int32]) Grid {
	return func() *gtk.Grid {
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

func (f Grid) BindMarginTop(state *state.State[int32]) Grid {
	return func() *gtk.Grid {
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

func (f Grid) BindSensitive(state *state.State[bool]) Grid {
	return func() *gtk.Grid {
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
