package signals

import (
	"sync"
)

type StatefulSignal[T any] struct {
	Signal[func(T) bool]
	currentValue T
	lock         sync.RWMutex
}

func (s *StatefulSignal[T]) CurrentValue() T {
	s.lock.RLock()
	defer s.lock.RUnlock()

	return s.currentValue
}

func (s *StatefulSignal[T]) Notify(callback func(oldValue T) T) {
	s.lock.Lock()
	s.currentValue = callback(s.currentValue)
	newValue := s.currentValue
	s.lock.Unlock()

	s.Signal.Notify(newValue)
}

func (s *StatefulSignal[T]) On(handler func(T) bool) *Subscription {
	s.lock.RLock()
	currentVal := s.currentValue
	sub := s.Signal.On(handler)
	s.lock.RUnlock()

	if handler(currentVal) {
		s.Signal.removeHandler(sub)
	}
	return sub
}

func (s *StatefulSignal[T]) OnLazy(handler func(T) bool) *Subscription {
	return s.Signal.On(handler)
}

func (s *StatefulSignal[T]) Set(value T) {
	s.Notify(func(T) T {
		return value
	})
}

func NewStatefulSignal[T any](initialValue T) *StatefulSignal[T] {
	return &StatefulSignal[T]{
		currentValue: initialValue,
		lock:         sync.RWMutex{},
		Signal:       NewSignal[func(T) bool](),
	}
}
