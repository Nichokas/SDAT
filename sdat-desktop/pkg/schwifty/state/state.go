package state

import (
	"runtime"
	"sync"

	"github.com/google/uuid"
)

type StateCallback[T any] func(newValue T)

type State[T any] struct {
	value         T
	callbacks     map[string]StateCallback[T]
	callbacksLock sync.RWMutex
	stateful      bool

	boundState         *State[T]
	boundStateCallback string
}

func (s *State[T]) AddCallback(callback StateCallback[T]) string {
	id := uuid.NewString()
	s.callbacksLock.Lock()
	s.callbacks[id] = callback
	s.callbacksLock.Unlock()
	if s.stateful {
		callback(s.value)
	}
	return id
}

func (s *State[T]) BindState(other *State[T]) {
	s.UnbindState()

	if other == nil {
		return
	}

	runtime.SetFinalizer(s, func(s *State[T]) {
		s.UnbindState()
	})

	s.boundState = other
	s.boundStateCallback = other.AddCallback(s.SetValue)
	s.SetValue(other.Value())
}

func (s *State[T]) UnbindState() {
	if s.boundState != nil {
		s.boundState.RemoveCallback(s.boundStateCallback)
		s.boundState = nil
		s.boundStateCallback = ""
		runtime.SetFinalizer(s, nil)
	}
}

func (s *State[T]) RemoveCallback(id string) {
	s.callbacksLock.Lock()
	delete(s.callbacks, id)
	s.callbacksLock.Unlock()
}

func (s *State[T]) SetValue(value T) {
	s.callbacksLock.RLock()
	for _, callback := range s.callbacks {
		callback(value)
	}
	s.callbacksLock.RUnlock()
	s.value = value
}

func (s *State[T]) Value() T {
	return s.value
}

func New[T any](value T) *State[T] {
	return &State[T]{
		value:     value,
		callbacks: make(map[string]StateCallback[T]),
	}
}

func NewStateful[T any](value T) *State[T] {
	return &State[T]{
		value:     value,
		callbacks: make(map[string]StateCallback[T]),
		stateful:  true,
	}
}

// NewBoundStateful creates a new state that is bound to another state. In combination with
// State[T].BindState(), this is used to conditionnaly derive a state from other states.
func NewBoundStateful[T any](other *State[T]) *State[T] {
	state := NewStateful(other.Value())
	state.BindState(other)
	return state
}
