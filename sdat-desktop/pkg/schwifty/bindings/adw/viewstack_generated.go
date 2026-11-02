package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type ViewStack func() *adw.ViewStack

func (f ViewStack) AddController(controller *gtk.EventController) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ViewStack) ConnectConstruct(cb func(*adw.ViewStack)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f ViewStack) ConnectDestroy(cb func(gtk.Widget)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f ViewStack) ConnectHide(cb func(gtk.Widget)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f ViewStack) ConnectMap(cb func(gtk.Widget)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f ViewStack) ConnectRealize(cb func(gtk.Widget)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f ViewStack) ConnectShow(cb func(gtk.Widget)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f ViewStack) ConnectUnmap(cb func(gtk.Widget)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f ViewStack) ConnectUnrealize(cb func(gtk.Widget)) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f ViewStack) Controller(controller *gtk.EventController) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f ViewStack) Focusable(focusable bool) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f ViewStack) FocusOnClick(focusOnClick bool) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f ViewStack) HAlign(align gtk.Align) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f ViewStack) HExpand(expand bool) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f ViewStack) HMargin(horizontal int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f ViewStack) Margin(margin int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f ViewStack) MarginBottom(bottom int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f ViewStack) MarginEnd(end int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f ViewStack) MarginStart(start int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f ViewStack) MarginTop(top int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f ViewStack) Opacity(opacity float64) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f ViewStack) Overflow(overflow gtk.Overflow) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f ViewStack) Sensitive(sensitive bool) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f ViewStack) SizeRequest(width, height int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f ViewStack) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f ViewStack) VAlign(align gtk.Align) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f ViewStack) VExpand(expand bool) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f ViewStack) Visible(visible bool) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f ViewStack) VMargin(vertical int32) ViewStack {
	return func() *adw.ViewStack {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f ViewStack) Background(color string) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f ViewStack) CornerRadius(radius int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f ViewStack) CSS(css string) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f ViewStack) BindCSSClass(state *state.State[string]) ViewStack {
	return func() *adw.ViewStack {
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

func (f ViewStack) WithCSSClass(className string) ViewStack {
	return func() *adw.ViewStack {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f ViewStack) CSSWithCallback(cb func(elementName string) string) ViewStack {
	return func() *adw.ViewStack {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.ViewStack) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f ViewStack) HPadding(padding int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f ViewStack) MinHeight(minHeight int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f ViewStack) MinWidth(minWidth int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f ViewStack) Padding(padding int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f ViewStack) PaddingBottom(padding int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f ViewStack) PaddingEnd(padding int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f ViewStack) PaddingStart(padding int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f ViewStack) PaddingTop(padding int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f ViewStack) VPadding(padding int) ViewStack {
	return func() *adw.ViewStack {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f ViewStack) BindVisible(state *state.State[bool]) ViewStack {
	return func() *adw.ViewStack {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.ViewStack) {
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

func (f ViewStack) BindHMargin(state *state.State[int32]) ViewStack {
	return func() *adw.ViewStack {
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

func (f ViewStack) BindMargin(state *state.State[int32]) ViewStack {
	return func() *adw.ViewStack {
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

func (f ViewStack) BindMarginBottom(state *state.State[int32]) ViewStack {
	return func() *adw.ViewStack {
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

func (f ViewStack) BindMarginEnd(state *state.State[int32]) ViewStack {
	return func() *adw.ViewStack {
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

func (f ViewStack) BindMarginStart(state *state.State[int32]) ViewStack {
	return func() *adw.ViewStack {
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

func (f ViewStack) BindMarginTop(state *state.State[int32]) ViewStack {
	return func() *adw.ViewStack {
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

func (f ViewStack) BindSensitive(state *state.State[bool]) ViewStack {
	return func() *adw.ViewStack {
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
