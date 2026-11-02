package gtk

import (
	"reflect"

	"codeberg.org/dergs/tonearm/pkg/schwifty/callback"
	"codeberg.org/puregotk/puregotk/v4/gtk"
)

//go:generate go run codeberg.org/dergs/tonearm/pkg/schwifty/gen Widget *WrappedWidget gtk
type WrappedWidget struct {
	gtk.Widget
}

type BaseWidgetable interface {
	ToGTK() *gtk.Widget
}

type Widgetable[T any] interface {
	BaseWidgetable
	AddController(controller *gtk.EventController) T
	CSS(css string) T
	Focusable(focusable bool) T
	FocusOnClick(focusOnClick bool) T
	HAlign(align gtk.Align) T
	HExpand(expand bool) T
	HMargin(horizontal int32) T
	Margin(margin int32) T
	MarginBottom(bottom int32) T
	MarginEnd(end int32) T
	MarginStart(start int32) T
	MarginTop(top int32) T
	Opacity(opacity float64) T
	Overflow(overflow gtk.Overflow) T
	VAlign(align gtk.Align) T
	VExpand(expand bool) T
	Visible(visible bool) T
	VMargin(vertical int32) T
}

func ResolveWidget(value any) *gtk.Widget {
	t := reflect.TypeOf(value)

	if value == nil {
		return nil
	}

	if t.AssignableTo(reflect.TypeFor[*gtk.Widget]()) {
		// if shouldLogLifecycle {
		// 	logger.Debug("resolved widget from *gtk.Widget")
		// }
		return value.(*gtk.Widget)
	}

	if t.AssignableTo(reflect.TypeFor[BaseWidgetable]()) {
		// if shouldLogLifecycle {
		// 	logger.Debug("resolved widget from Widgetable")
		// }
		return value.(BaseWidgetable).ToGTK()
	}

	if t.Kind() == reflect.Pointer && t.Elem().Kind() == reflect.Struct {
		if field := reflect.ValueOf(value).Elem().FieldByName("Widget"); field.IsValid() {
			// if shouldLogLifecycle {
			// 	logger.Debug("resolved widget from specialized *gtk.Widget")
			// }
			widget := field.Interface().(gtk.Widget)
			return &widget
		}
	}

	// logger.Warn("failed to resolve widget")
	return nil
}

func ResolveWidgetOnMain(value any) *gtk.Widget {
	t := reflect.TypeOf(value)

	if value == nil {
		return nil
	}

	if t.AssignableTo(reflect.TypeFor[*gtk.Widget]()) {
		return value.(*gtk.Widget)
	}

	if t.AssignableTo(reflect.TypeFor[BaseWidgetable]()) {
		channel := make(chan *gtk.Widget, 1)
		callback.OnMainThreadOncePure(func() {
			channel <- value.(BaseWidgetable).ToGTK()
		})
		return <-channel
	}

	if t.Kind() == reflect.Pointer && t.Elem().Kind() == reflect.Struct {
		if field := reflect.ValueOf(value).Elem().FieldByName("Widget"); field.IsValid() {
			widget := field.Interface().(gtk.Widget)
			return &widget
		}
	}

	return nil
}
