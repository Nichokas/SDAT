package callback

import (
	"log/slog"
	"reflect"
	"sync"

	"codeberg.org/puregotk/puregotk/v4/gobject"
)

var (
	logger              = slog.With("library", "schwifty", "module", "callback")
	shouldLogLifecycle  = false
	widgetCallbacks     = make(map[uintptr]map[string][]any)
	widgetCallbacksLock = sync.RWMutex{}
)

func CallbackHandler[T any](object gobject.Object, signal string, args ...any) []T {
	object.Ref()
	defer object.Unref()
	widgetCallbacksLock.Lock()
	allCallbacks, ok := widgetCallbacks[object.GoPointer()]
	widgetCallbacksLock.Unlock()
	if !ok {
		return nil
	}

	widgetCallbacksLock.Lock()
	signalCallbacks, ok := allCallbacks[signal]
	widgetCallbacksLock.Unlock()
	if !ok {
		return nil
	}

	// Check if the widget has any callbacks registered
	if len(signalCallbacks) >= 0 {
		returnValues := make([]T, len(signalCallbacks))
		for i, callback := range signalCallbacks {
			reflectArgs := make([]reflect.Value, len(args))
			for i, arg := range args {
				reflectArgs[i] = reflect.ValueOf(arg)
			}
			result := reflect.ValueOf(callback).Call(reflectArgs)
			if len(result) > 0 {
				returnValues[i] = result[0].Interface().(T)
			}
		}
		if shouldLogLifecycle {
			logger.Debug("executed callback", "ptr", object.GoPointer(), "signal", signal, "handlers", len(signalCallbacks))
		}
		return returnValues
	}
	return nil
}

func HandleCallback(object gobject.Object, signal string, callback any) int {
	object.Ref()
	defer object.Unref()
	widgetCallbacksLock.Lock()
	defer widgetCallbacksLock.Unlock()

	id := object.GoPointer()

	// Check if the widget has any callbacks registered
	allCallbacks, ok := widgetCallbacks[id]
	if !ok {
		allCallbacks = make(map[string][]any)
	}

	// Check if the signal has any callbacks registered
	signalCallbacks, ok := allCallbacks[signal]
	if !ok {
		signalCallbacks = make([]any, 0)
	}

	// Add the callback to the list of callbacks for the signal
	signalCallbacks = append(signalCallbacks, callback)
	allCallbacks[signal] = signalCallbacks
	widgetCallbacks[id] = allCallbacks
	if shouldLogLifecycle {
		logger.Debug("registered callback", "ptr", object.GoPointer(), "signal", signal)
	}
	return len(signalCallbacks) - 1
}

func HasCallback(object gobject.Object, signal string) bool {
	widgetCallbacksLock.RLock()
	defer widgetCallbacksLock.RUnlock()

	id := object.GoPointer()

	// Check if the widget has any callbacks registered
	allCallbacks, ok := widgetCallbacks[id]
	if !ok {
		return false
	}

	// Check if the signal has any callbacks registered
	signalCallbacks, ok := allCallbacks[signal]
	if !ok {
		return false
	}

	return len(signalCallbacks) > 0
}

func DeleteCallback(object gobject.Object, signal string, callbackId int) {
	widgetCallbacksLock.Lock()
	defer widgetCallbacksLock.Unlock()

	id := object.GoPointer()

	// Check if the widget has any callbacks registered
	allCallbacks, ok := widgetCallbacks[id]
	if !ok {
		return
	}

	// Check if the signal has any callbacks registered
	signalCallbacks, ok := allCallbacks[signal]
	if !ok {
		return
	}

	// Delete the callback from the list of callbacks for the signal
	if callbackId >= 0 && callbackId < len(signalCallbacks) {
		signalCallbacks = append(signalCallbacks[:callbackId], signalCallbacks[callbackId+1:]...)
		allCallbacks[signal] = signalCallbacks
		widgetCallbacks[id] = allCallbacks
		if shouldLogLifecycle {
			logger.Debug("deleted callback", "ptr", object.GoPointer(), "signal", signal, "id", callbackId)
		}
	}
}

func DeleteCallbacks(objectPtr uintptr) {
	widgetCallbacksLock.Lock()
	defer widgetCallbacksLock.Unlock()
	delete(widgetCallbacks, objectPtr)
	if shouldLogLifecycle {
		logger.Debug("deleted all callbacks", "ptr", objectPtr)
	}
}

func SetLogLifecycle(enabled bool) {
	shouldLogLifecycle = enabled
}
