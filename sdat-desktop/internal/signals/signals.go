package signals

import (
	"maps"
	"reflect"
	"sync"
	"sync/atomic"

	"github.com/google/uuid"
)

const (
	Continue    = false
	Unsubscribe = true
)

type Subscription struct {
	id      uuid.UUID
	removed atomic.Bool
}

type Signal[T any] struct {
	mutex       sync.Mutex
	handlers    map[*Subscription]T
	notifyMutex sync.Mutex
}

func (b *Signal[T]) addHandler(handler T) *Subscription {
	b.mutex.Lock()
	defer b.mutex.Unlock()
	sub := &Subscription{id: uuid.New(), removed: atomic.Bool{}}
	b.handlers[sub] = handler
	return sub
}

func (b *Signal[T]) removeHandler(sub *Subscription) {
	if sub == nil || sub.removed.Load() {
		return
	}

	b.mutex.Lock()
	defer b.mutex.Unlock()
	sub.removed.Store(true)
	delete(b.handlers, sub)
}

func (b *Signal[T]) Notify(args ...any) {
	b.mutex.Lock()
	handlers := maps.Clone(b.handlers)
	b.mutex.Unlock()

	b.notifyMutex.Lock()
	defer b.notifyMutex.Unlock()

	for sub, handler := range handlers {
		if sub.removed.Load() {
			continue
		}
		handlerType := reflect.TypeOf(handler)
		reflectArgs := make([]reflect.Value, len(args))
		for i, arg := range args {
			if arg == nil {
				reflectArgs[i] = reflect.Zero(handlerType.In(i))
			} else {
				reflectArgs[i] = reflect.ValueOf(arg)
			}
		}
		result := reflect.ValueOf(handler).Call(reflectArgs)
		if len(result) > 0 && result[0].CanConvert(reflect.TypeFor[bool]()) && result[0].Bool() {
			b.removeHandler(sub)
		}
	}
}

func (b *Signal[T]) On(handler T) *Subscription {
	return b.addHandler(handler)
}

func (b *Signal[T]) Unsubscribe(sub *Subscription) {
	b.removeHandler(sub)
}

func NewSignal[T any]() Signal[T] {
	return Signal[T]{
		handlers: make(map[*Subscription]T),
	}
}

func ContinueIf(condition bool) bool {
	if condition {
		return Continue
	}
	return Unsubscribe
}
