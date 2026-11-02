package bindings

import "reflect"

func ResolveTo[GtkType any, SchwiftyType any](value any) (result GtkType) {
	t := reflect.TypeOf(value)

	if t.AssignableTo(reflect.TypeFor[GtkType]()) {
		// if shouldLogLifecycle {
		// 	logger.Debug("resolved generic from gtk")
		// }
		result = value.(GtkType)
		return
	}

	if t.AssignableTo(reflect.TypeFor[SchwiftyType]()) && t.Kind() == reflect.Func && t.NumOut() == 1 && t.NumIn() == 0 {
		// if shouldLogLifecycle {
		// 	logger.Debug("resolved generic from schwifty")
		// }
		result = reflect.ValueOf(value).Call([]reflect.Value{})[0].Interface().(GtkType)
		return
	}

	// logger.Warn("failed to resolve generic")
	return
}
