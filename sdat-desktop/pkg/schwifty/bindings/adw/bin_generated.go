package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type Bin func() *adw.Bin

func (f Bin) AddController(controller *gtk.EventController) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Bin) ConnectConstruct(cb func(*adw.Bin)) Bin {
	return func() *adw.Bin {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f Bin) ConnectDestroy(cb func(gtk.Widget)) Bin {
	return func() *adw.Bin {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f Bin) ConnectHide(cb func(gtk.Widget)) Bin {
	return func() *adw.Bin {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f Bin) ConnectMap(cb func(gtk.Widget)) Bin {
	return func() *adw.Bin {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f Bin) ConnectRealize(cb func(gtk.Widget)) Bin {
	return func() *adw.Bin {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f Bin) ConnectShow(cb func(gtk.Widget)) Bin {
	return func() *adw.Bin {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f Bin) ConnectUnmap(cb func(gtk.Widget)) Bin {
	return func() *adw.Bin {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f Bin) ConnectUnrealize(cb func(gtk.Widget)) Bin {
	return func() *adw.Bin {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f Bin) Controller(controller *gtk.EventController) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f Bin) Focusable(focusable bool) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f Bin) FocusOnClick(focusOnClick bool) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f Bin) HAlign(align gtk.Align) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f Bin) HExpand(expand bool) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f Bin) HMargin(horizontal int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f Bin) Margin(margin int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f Bin) MarginBottom(bottom int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f Bin) MarginEnd(end int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f Bin) MarginStart(start int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f Bin) MarginTop(top int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f Bin) Opacity(opacity float64) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f Bin) Overflow(overflow gtk.Overflow) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f Bin) Sensitive(sensitive bool) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f Bin) SizeRequest(width, height int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f Bin) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f Bin) VAlign(align gtk.Align) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f Bin) VExpand(expand bool) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f Bin) Visible(visible bool) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f Bin) VMargin(vertical int32) Bin {
	return func() *adw.Bin {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f Bin) Background(color string) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f Bin) CornerRadius(radius int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f Bin) CSS(css string) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f Bin) BindCSSClass(state *state.State[string]) Bin {
	return func() *adw.Bin {
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

func (f Bin) WithCSSClass(className string) Bin {
	return func() *adw.Bin {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f Bin) CSSWithCallback(cb func(elementName string) string) Bin {
	return func() *adw.Bin {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.Bin) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f Bin) HPadding(padding int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f Bin) MinHeight(minHeight int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f Bin) MinWidth(minWidth int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f Bin) Padding(padding int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f Bin) PaddingBottom(padding int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f Bin) PaddingEnd(padding int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f Bin) PaddingStart(padding int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f Bin) PaddingTop(padding int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f Bin) VPadding(padding int) Bin {
	return func() *adw.Bin {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f Bin) BindVisible(state *state.State[bool]) Bin {
	return func() *adw.Bin {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.Bin) {
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

func (f Bin) BindHMargin(state *state.State[int32]) Bin {
	return func() *adw.Bin {
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

func (f Bin) BindMargin(state *state.State[int32]) Bin {
	return func() *adw.Bin {
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

func (f Bin) BindMarginBottom(state *state.State[int32]) Bin {
	return func() *adw.Bin {
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

func (f Bin) BindMarginEnd(state *state.State[int32]) Bin {
	return func() *adw.Bin {
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

func (f Bin) BindMarginStart(state *state.State[int32]) Bin {
	return func() *adw.Bin {
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

func (f Bin) BindMarginTop(state *state.State[int32]) Bin {
	return func() *adw.Bin {
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

func (f Bin) BindSensitive(state *state.State[bool]) Bin {
	return func() *adw.Bin {
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
