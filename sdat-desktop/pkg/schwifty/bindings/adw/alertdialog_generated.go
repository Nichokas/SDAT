package adw

import (
	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/dergs/tonearm/pkg/schwifty/state"
	"codeberg.org/dergs/tonearm/pkg/schwifty/utils/weak"
	"codeberg.org/puregotk/puregotk/v4/adw"
	"codeberg.org/puregotk/puregotk/v4/gtk"
	"fmt"
)


type AlertDialog func() *adw.AlertDialog

func (f AlertDialog) AddController(controller *gtk.EventController) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f AlertDialog) ConnectConstruct(cb func(*adw.AlertDialog)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		cb(widget)
		return widget
	}
}

func (f AlertDialog) ConnectDestroy(cb func(gtk.Widget)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		callback.HandleCallback(widget.Object, "destroy", cb)
		return widget
	}
}

func (f AlertDialog) ConnectHide(cb func(gtk.Widget)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		callback.HandleCallback(widget.Object, "hide", cb)
		return widget
	}
}

func (f AlertDialog) ConnectMap(cb func(gtk.Widget)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		callback.HandleCallback(widget.Object, "map", cb)
		return widget
	}
}

func (f AlertDialog) ConnectRealize(cb func(gtk.Widget)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		callback.HandleCallback(widget.Object, "realize", cb)
		return widget
	}
}

func (f AlertDialog) ConnectShow(cb func(gtk.Widget)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		callback.HandleCallback(widget.Object, "show", cb)
		return widget
	}
}

func (f AlertDialog) ConnectUnmap(cb func(gtk.Widget)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		callback.HandleCallback(widget.Object, "unmap", cb)
		return widget
	}
}

func (f AlertDialog) ConnectUnrealize(cb func(gtk.Widget)) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		callback.HandleCallback(widget.Object, "unrealize", cb)
		return widget
	}
}

func (f AlertDialog) Controller(controller *gtk.EventController) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.AddController(controller)
		return widget
	}
}

func (f AlertDialog) Focusable(focusable bool) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetFocusable(focusable)
		return widget
	}
}

func (f AlertDialog) FocusOnClick(focusOnClick bool) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetFocusOnClick(focusOnClick)
		return widget
	}
}

func (f AlertDialog) HAlign(align gtk.Align) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetHalign(align)
		return widget
	}
}

func (f AlertDialog) HExpand(expand bool) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetHexpand(expand)
		return widget
	}
}

func (f AlertDialog) HMargin(horizontal int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetMarginEnd(horizontal)
		widget.SetMarginStart(horizontal)
		return widget
	}
}

func (f AlertDialog) Margin(margin int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetMarginBottom(margin)
		widget.SetMarginEnd(margin)
		widget.SetMarginStart(margin)
		widget.SetMarginTop(margin)
		return widget
	}
}

func (f AlertDialog) MarginBottom(bottom int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetMarginBottom(bottom)
		return widget
	}
}

func (f AlertDialog) MarginEnd(end int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetMarginEnd(end)
		return widget
	}
}

func (f AlertDialog) MarginStart(start int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetMarginStart(start)
		return widget
	}
}

func (f AlertDialog) MarginTop(top int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetMarginTop(top)
		return widget
	}
}

func (f AlertDialog) Opacity(opacity float64) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetOpacity(opacity)
		return widget
	}
}

func (f AlertDialog) Overflow(overflow gtk.Overflow) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetOverflow(overflow)
		return widget
	}
}

func (f AlertDialog) Sensitive(sensitive bool) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetSensitive(sensitive)
		return widget
	}
}

func (f AlertDialog) SizeRequest(width, height int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetSizeRequest(width, height)
		return widget
	}
}

func (f AlertDialog) ToGTK() *gtk.Widget {
	val := f()
	return &val.Widget
}

func (f AlertDialog) VAlign(align gtk.Align) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetValign(align)
		return widget
	}
}

func (f AlertDialog) VExpand(expand bool) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetVexpand(expand)
		return widget
	}
}

func (f AlertDialog) Visible(visible bool) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetVisible(visible)
		return widget
	}
}

func (f AlertDialog) VMargin(vertical int32) AlertDialog {
	return func() *adw.AlertDialog {
		widget := f()
		widget.SetMarginTop(vertical)
		widget.SetMarginBottom(vertical)
		return widget
	}
}



func (f AlertDialog) Background(color string) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { background-color: %s; }", elementName, color)
		})()
	}
}

func (f AlertDialog) CornerRadius(radius int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { border-radius: %dpx; }", elementName, radius)
		})()
	}
}

func (f AlertDialog) CSS(css string) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return css
		})()
	}
}

func (f AlertDialog) BindCSSClass(state *state.State[string]) AlertDialog {
	return func() *adw.AlertDialog {
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

func (f AlertDialog) WithCSSClass(className string) AlertDialog {
	return func() *adw.AlertDialog {
		w := f()
		styleContext := w.GetStyleContext()
		defer styleContext.Unref()

		styleContext.AddClass(className)
		return w
	}
}

func (f AlertDialog) CSSWithCallback(cb func(elementName string) string) AlertDialog {
	return func() *adw.AlertDialog {
		provider := gtk.NewCssProvider()
		return f.ConnectConstruct(func(t *adw.AlertDialog) {
			provider.LoadFromString(cb(t.GetCssName()))
			t.GetStyleContext().AddProvider(provider, uint32(gtk.STYLE_PROVIDER_PRIORITY_APPLICATION))
		}).ConnectDestroy(func(w gtk.Widget) {
			w.GetStyleContext().RemoveProvider(provider)
			provider.Unref()
			provider = nil
		})()
	}
}

func (f AlertDialog) HPadding(padding int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; padding-right: %dpx; }", elementName, padding, padding)
		})()
	}
}

func (f AlertDialog) MinHeight(minHeight int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-height: %dpx; }", elementName, minHeight)
		})()
	}
}

func (f AlertDialog) MinWidth(minWidth int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { min-width: %dpx; }", elementName, minWidth)
		})()
	}
}

func (f AlertDialog) Padding(padding int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding: %dpx; }", elementName, padding)
		})()
	}
}

func (f AlertDialog) PaddingBottom(padding int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; }", elementName, padding)
		})()
	}
}

func (f AlertDialog) PaddingEnd(padding int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-right: %dpx; }", elementName, padding)
		})()
	}
}

func (f AlertDialog) PaddingStart(padding int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-left: %dpx; }", elementName, padding)
		})()
	}
}

func (f AlertDialog) PaddingTop(padding int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-top: %dpx; }", elementName, padding)
		})()
	}
}

func (f AlertDialog) VPadding(padding int) AlertDialog {
	return func() *adw.AlertDialog {
		return f.CSSWithCallback(func(elementName string) string {
			return fmt.Sprintf("%s { padding-bottom: %dpx; padding-top: %dpx; }", elementName, padding, padding)
		})()
	}
}



func (f AlertDialog) BindVisible(state *state.State[bool]) AlertDialog {
	return func() *adw.AlertDialog {
		var callbackId string
		var ref weak.ObjectRef
		return f.ConnectConstruct(func(w *adw.AlertDialog) {
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

func (f AlertDialog) BindHMargin(state *state.State[int32]) AlertDialog {
	return func() *adw.AlertDialog {
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

func (f AlertDialog) BindMargin(state *state.State[int32]) AlertDialog {
	return func() *adw.AlertDialog {
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

func (f AlertDialog) BindMarginBottom(state *state.State[int32]) AlertDialog {
	return func() *adw.AlertDialog {
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

func (f AlertDialog) BindMarginEnd(state *state.State[int32]) AlertDialog {
	return func() *adw.AlertDialog {
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

func (f AlertDialog) BindMarginStart(state *state.State[int32]) AlertDialog {
	return func() *adw.AlertDialog {
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

func (f AlertDialog) BindMarginTop(state *state.State[int32]) AlertDialog {
	return func() *adw.AlertDialog {
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

func (f AlertDialog) BindSensitive(state *state.State[bool]) AlertDialog {
	return func() *adw.AlertDialog {
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
